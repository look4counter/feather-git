use std::{
    fmt,
    fs::{File, OpenOptions},
    io::Write,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

static TRACE: OnceLock<Option<Mutex<File>>> = OnceLock::new();

pub(super) fn enabled() -> bool {
    TRACE
        .get_or_init(|| {
            let path = std::env::var_os("FEATHER_IME_TRACE")?;
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
                .map(Mutex::new)
        })
        .is_some()
}

pub(super) fn record(message: fmt::Arguments<'_>) {
    if !enabled() {
        return;
    }
    if let Some(Some(file)) = TRACE.get()
        && let Ok(mut file) = file.lock()
    {
        let _ = writeln!(
            file,
            "{:?} pid={} {message}",
            SystemTime::now(),
            std::process::id()
        );
    }
}
