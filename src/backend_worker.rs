use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    pin::Pin,
    sync::{Arc, Condvar, Mutex, OnceLock},
    task::{Context, Poll, Waker},
    time::Duration,
};

const WORKERS: usize = 2;
static POOL: OnceLock<WorkerPool> = OnceLock::new();
thread_local! {
    static READ: RefCell<Option<Arc<Cancellation>>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct Cancellation {
    cancelled: Mutex<bool>,
    changed: Condvar,
    #[cfg(test)]
    waiting: std::sync::atomic::AtomicBool,
}
impl Cancellation {
    fn cancel(&self) {
        *self.cancelled.lock().unwrap() = true;
        self.changed.notify_all();
    }
    fn is_cancelled(&self) -> bool {
        *self.cancelled.lock().unwrap()
    }
    fn wait(&self, delay: Duration) {
        let cancelled = {
            let guard = self.cancelled.lock().unwrap();
            #[cfg(test)]
            self.waiting
                .store(true, std::sync::atomic::Ordering::Release);
            let (cancelled, _) = self
                .changed
                .wait_timeout_while(guard, delay, |cancelled| !*cancelled)
                .unwrap();
            *cancelled
        };
        if cancelled {
            // 일반 패닉 훅을 실행하지 않고 읽기 작업 경계로만 전달한다.
            resume_unwind(Box::new(Cancelled));
        }
    }
}
struct Cancelled;

pub(super) fn checkpoint() {
    let token = READ.with(|read| read.borrow().clone());
    if token.is_some_and(|token| token.is_cancelled()) {
        resume_unwind(Box::new(Cancelled));
    }
}

pub(super) fn delay(duration: Duration) {
    match READ.with(|read| read.borrow().clone()) {
        Some(token) => token.wait(duration),
        None => std::thread::sleep(duration),
    }
}

// 동기 gateway의 반환형을 유지하면서 읽기 경계에서만 취소를 처리한다.
fn read<T>(token: Arc<Cancellation>, work: impl FnOnce() -> T) -> Option<T> {
    struct Restore(Option<Arc<Cancellation>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            READ.with(|read| *read.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(READ.with(|read| read.replace(Some(token))));
    match catch_unwind(AssertUnwindSafe(|| {
        checkpoint();
        let result = work();
        checkpoint();
        result
    })) {
        Ok(value) => Some(value),
        Err(error) if error.is::<Cancelled>() => None,
        Err(error) => resume_unwind(error),
    }
}

#[derive(Default)]
pub(crate) struct ReadSlot {
    current: Option<Arc<Cancellation>>,
}
impl ReadSlot {
    #[cfg(test)]
    pub(crate) fn observe_cancellation(&mut self) -> impl Fn() -> bool + use<> {
        self.cancel();
        let token = Arc::new(Cancellation::default());
        self.current = Some(token.clone());
        move || token.is_cancelled()
    }

    pub(crate) fn cancel(&mut self) {
        if let Some(token) = self.current.take() {
            token.cancel();
        }
    }
    fn submit<T: Send + 'static>(
        &mut self,
        pool: &WorkerPool,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Completion<Option<T>> {
        self.cancel();
        let token = Arc::new(Cancellation::default());
        self.current = Some(token.clone());
        pool.submit(Some(token.clone()), move || read(token, work))
    }
    pub(crate) fn spawn<T: Send + 'static>(
        &mut self,
        executor: &gpui::BackgroundExecutor,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> gpui::Task<Option<T>> {
        let result = self.submit(POOL.get_or_init(|| WorkerPool::new(WORKERS)), work);
        executor.spawn(result)
    }
}
impl Drop for ReadSlot {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub(super) fn spawn<T: Send + 'static>(
    executor: &gpui::BackgroundExecutor,
    work: impl FnOnce() -> T + Send + 'static,
) -> gpui::Task<T> {
    let result = POOL
        .get_or_init(|| WorkerPool::new(WORKERS))
        .submit(None, work);
    executor.spawn(result)
}

struct CompletionState<T> {
    result: Option<std::thread::Result<T>>,
    waker: Option<Waker>,
}
struct Completion<T>(Arc<Mutex<CompletionState<T>>>);
impl<T> Future for Completion<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.0.lock().unwrap();
        if let Some(result) = state.result.take() {
            drop(state);
            return Poll::Ready(result.unwrap_or_else(|error| resume_unwind(error)));
        }
        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

struct Job {
    cancellation: Option<Arc<Cancellation>>,
    run: Box<dyn FnOnce() + Send>,
}
#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    stopping: bool,
}
#[derive(Default)]
struct Shared {
    state: Mutex<Queue>,
    ready: Condvar,
}
struct WorkerPool {
    shared: Arc<Shared>,
    workers: Vec<std::thread::JoinHandle<()>>,
}
impl WorkerPool {
    fn new(count: usize) -> Self {
        let shared = Arc::new(Shared::default());
        let workers = (0..count)
            .map(|index| {
                let shared = shared.clone();
                std::thread::Builder::new()
                    .name(format!("backend-{index}"))
                    .spawn(move || {
                        loop {
                            let job = {
                                let mut queue = shared.state.lock().unwrap();
                                while queue.jobs.is_empty() && !queue.stopping {
                                    queue = shared.ready.wait(queue).unwrap();
                                }
                                match queue.jobs.pop_front() {
                                    Some(job) => job,
                                    None => return,
                                }
                            };
                            (job.run)();
                        }
                    })
                    .expect("백엔드 작업 스레드 생성 실패")
            })
            .collect();
        Self { shared, workers }
    }
    fn submit<T: Send + 'static>(
        &self,
        cancellation: Option<Arc<Cancellation>>,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Completion<T> {
        let completion = Arc::new(Mutex::new(CompletionState {
            result: None,
            waker: None,
        }));
        let sender = completion.clone();
        let job = Job {
            cancellation,
            run: Box::new(move || {
                let result = catch_unwind(AssertUnwindSafe(work));
                let waker = {
                    let mut state = sender.lock().unwrap();
                    state.result = Some(result);
                    state.waker.take()
                };
                if let Some(waker) = waker {
                    waker.wake();
                }
            }),
        };
        let stale = {
            let mut queue = self.shared.state.lock().unwrap();
            let mut stale = Vec::new();
            let mut pending = VecDeque::new();
            for job in queue.jobs.drain(..) {
                if job
                    .cancellation
                    .as_ref()
                    .is_some_and(|token| token.is_cancelled())
                {
                    stale.push(job);
                } else {
                    pending.push_back(job);
                }
            }
            pending.push_back(job);
            queue.jobs = pending;
            stale
        };
        self.shared.ready.notify_one();
        // 취소된 조회는 실제 I/O 없이 완료시켜 대기 Future와 캡처도 해제한다.
        for job in stale {
            (job.run)();
        }
        Completion(completion)
    }
}
impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().stopping = true;
        self.shared.ready.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::Instant,
    };

    fn finish<T>(future: Completion<T>) -> T {
        use std::{
            future::Future,
            pin::pin,
            task::{Context, Wake, Waker},
        };
        struct ThreadWake(std::thread::Thread);
        impl Wake for ThreadWake {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut future = pin!(future);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Poll::Ready(result) = future.as_mut().poll(&mut cx) {
                return result;
            }
            assert!(Instant::now() < deadline, "백엔드 완료 시간 초과");
            std::thread::park_timeout(Duration::from_millis(10));
        }
    }

    #[test]
    fn pending_completion_wakes_executor_and_ready_completion_returns_immediately() {
        use std::task::Wake;
        struct Notify(mpsc::Sender<()>);
        impl Wake for Notify {
            fn wake(self: Arc<Self>) {
                self.0.send(()).unwrap();
            }
        }
        let pool = WorkerPool::new(1);
        let (release, wait) = mpsc::channel();
        let mut result = Box::pin(pool.submit(None, move || {
            wait.recv_timeout(Duration::from_secs(3)).unwrap();
            71
        }));
        let (notify, awakened) = mpsc::channel();
        let waker = Waker::from(Arc::new(Notify(notify)));
        let mut cx = Context::from_waker(&waker);
        assert!(result.as_mut().poll(&mut cx).is_pending());
        release.send(()).unwrap();
        awakened.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(result.as_mut().poll(&mut cx), Poll::Ready(71));
        let ready = pool.submit(None, || 72);
        assert_eq!(finish(ready), 72);
    }

    #[test]
    fn running_jobs_are_bounded_and_normal_work_is_preserved() {
        let pool = WorkerPool::new(2);
        let (ready, received) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let wait = Arc::new(Mutex::new(wait));
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut jobs = Vec::new();
        for index in 0..20 {
            let (ready, wait, running, peak) =
                (ready.clone(), wait.clone(), running.clone(), peak.clone());
            jobs.push(pool.submit(None, move || {
                let count = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                ready.send(index).unwrap();
                wait.lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(3))
                    .unwrap();
                running.fetch_sub(1, Ordering::SeqCst);
                index
            }));
        }
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(received.recv_timeout(Duration::from_millis(30)).is_err());
        for _ in 0..20 {
            release.send(()).unwrap();
        }
        assert_eq!(
            jobs.into_iter().map(finish).collect::<Vec<_>>(),
            (0..20).collect::<Vec<_>>()
        );
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn replacing_one_hundred_pending_reads_keeps_only_latest() {
        let pool = WorkerPool::new(1);
        let (ready, received) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let blocked = pool.submit(None, move || {
            ready.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(3)).unwrap();
        });
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        let mut slot = ReadSlot::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut jobs = Vec::new();
        for index in 0..100 {
            let calls = calls.clone();
            jobs.push(slot.submit(&pool, move || {
                calls.fetch_add(1, Ordering::SeqCst);
                index
            }));
            assert_eq!(pool.shared.state.lock().unwrap().jobs.len(), 1);
        }
        release.send(()).unwrap();
        finish(blocked);
        let results: Vec<_> = jobs.into_iter().map(finish).collect();
        assert!(results[..99].iter().all(Option::is_none));
        assert_eq!(results[99], Some(99));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cancellation_wakes_delay_without_running_io_and_next_job_succeeds() {
        let pool = WorkerPool::new(1);
        let mut slot = ReadSlot::default();
        let (ready, received) = mpsc::channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let delayed = slot.submit(&pool, move || {
            crate::backend::with_test_delay(1500, std::thread::current().id(), || {
                ready.send(()).unwrap();
                crate::backend::run("cancelled-io", || counted.fetch_add(1, Ordering::SeqCst))
            })
        });
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        let token = slot.current.as_ref().unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !token.waiting.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let start = Instant::now();
        slot.cancel();
        assert_eq!(finish(delayed), None);
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(finish(pool.submit(None, || 42)), 42);
    }

    #[test]
    fn active_io_finishes_but_next_boundary_is_cancelled() {
        let pool = WorkerPool::new(1);
        let mut slot = ReadSlot::default();
        let (ready, received) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let job = slot.submit(&pool, move || {
            crate::backend::run("active", || {
                ready.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(3)).unwrap();
            });
            crate::backend::run("obsolete-next", || panic!("이전 조회가 실행됨"));
        });
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        slot.cancel();
        release.send(()).unwrap();
        assert_eq!(finish(job), None);
    }

    #[test]
    fn drop_cancels_reads_and_errors_and_panics_do_not_kill_worker() {
        let pool = WorkerPool::new(1);
        let mut slot = ReadSlot::default();
        assert_eq!(
            finish(slot.submit(&pool, || Err::<(), _>("읽기 오류"))),
            Some(Err("읽기 오류"))
        );
        let panic = pool.submit(None, || panic!("실제 패닉"));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| finish(panic))).is_err());
        let (ready, received) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let blocked = pool.submit(None, move || {
            ready.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(3)).unwrap();
        });
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        let cancelled = slot.submit(&pool, || panic!("소멸한 조회가 실행됨"));
        drop(slot);
        let write = pool.submit(None, || 17);
        release.send(()).unwrap();
        finish(blocked);
        assert_eq!(finish(cancelled), None);
        assert_eq!(finish(write), 17);
    }
}
