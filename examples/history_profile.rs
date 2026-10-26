use feather_git::git;
use std::{path::PathBuf, time::Instant};

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("저장소 경로가 필요합니다"),
    );
    let started = Instant::now();
    let snapshot = git::load_initial(&path, None)?;
    println!(
        "최초 커밋 {}개, 첫 페이지 조회 {:.2}ms",
        snapshot.commits.len(),
        started.elapsed().as_secs_f64() * 1000.
    );
    if snapshot.has_more {
        let started = Instant::now();
        let page = snapshot.session.as_ref().unwrap().next_page()?;
        println!(
            "다음 페이지 {}개, {:.2}ms",
            page.commits.len(),
            started.elapsed().as_secs_f64() * 1000.
        );
    }
    let ids: Vec<_> = snapshot.commits.iter().take(20).map(|c| &c.id).collect();
    if !ids.is_empty() {
        let started = Instant::now();
        for id in &ids {
            git::load_commit(&path, id)?;
        }
        println!(
            "상세 전용 조회 {}회, 평균 {:.2}ms",
            ids.len(),
            started.elapsed().as_secs_f64() * 1000. / ids.len() as f64
        );
        let started = Instant::now();
        git::load(&path, None, Some(ids[0]))?;
        println!(
            "기존 전체 재조회 방식 1회 {:.2}ms",
            started.elapsed().as_secs_f64() * 1000.
        );
    }
    Ok(())
}
