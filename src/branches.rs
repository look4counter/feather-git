use crate::settings_git::GitContext;
use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

#[derive(Clone)]
pub struct CheckoutTarget {
    pub local: String,
    pub remote: Option<String>,
    pub needs_creation: bool,
}

fn command(path: &Path) -> Command {
    GitContext::new(Some(path.to_owned())).command()
}

fn execute(path: &Path, args: &[&str]) -> Result<String> {
    let output = command(path)
        .args(args)
        .output()
        .context("Git 브랜치 명령을 실행하지 못했습니다.")?;
    if !output.status.success() {
        bail!(
            "브랜치 작업 실패: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn validate_name(path: &Path, name: &str) -> Result<()> {
    if name.is_empty()
        || name == "HEAD"
        || name.starts_with('-')
        || name.chars().any(char::is_control)
    {
        bail!("올바른 브랜치명을 입력하세요.");
    }
    let output = command(path)
        .args(["check-ref-format", &format!("refs/heads/{name}")])
        .output()?;
    if !output.status.success() {
        bail!("사용할 수 없는 브랜치명입니다. 공백이나 '..', '~', ':' 등을 확인하세요.");
    }
    Ok(())
}

fn reference_exists(path: &Path, reference: &str) -> Result<bool> {
    let output = command(path)
        .args(["show-ref", "--verify", "--quiet", reference])
        .output()?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => bail!("브랜치 참조를 확인할 수 없습니다."),
    }
}

fn local_exists(path: &Path, name: &str) -> Result<bool> {
    reference_exists(path, &format!("refs/heads/{name}"))
}

pub fn create(path: &Path, name: &str, commit: &str) -> Result<()> {
    validate_name(path, name)?;
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("선택한 커밋 ID가 올바르지 않습니다.");
    }
    execute(
        path,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )?;
    if local_exists(path, name)? {
        bail!("같은 이름의 로컬 브랜치가 이미 있습니다.");
    }
    execute(path, &["branch", "--no-track", "--", name, commit])?;
    Ok(())
}

pub fn prepare_checkout(path: &Path, reference: &str) -> Result<CheckoutTarget> {
    if let Some(local) = reference.strip_prefix("refs/heads/") {
        validate_name(path, local)?;
        if !local_exists(path, local)? {
            bail!("로컬 브랜치가 더 이상 존재하지 않습니다.");
        }
        return Ok(CheckoutTarget {
            local: local.into(),
            remote: None,
            needs_creation: false,
        });
    }
    if !reference.starts_with("refs/remotes/") || reference.chars().any(char::is_control) {
        bail!("Local 또는 Remote 브랜치를 선택하세요.");
    }
    if !reference_exists(path, reference)? {
        bail!("원격 브랜치 참조가 더 이상 존재하지 않습니다.");
    }
    let symbolic = command(path)
        .args(["symbolic-ref", "--quiet", reference])
        .output()?;
    let reference = if symbolic.status.success() {
        String::from_utf8(symbolic.stdout)?.trim().to_owned()
    } else {
        reference.to_owned()
    };
    let short = reference
        .strip_prefix("refs/remotes/")
        .context("원격 HEAD의 대상이 올바르지 않습니다.")?;
    let remotes = execute(path, &["remote"])?;
    let remote = remotes
        .lines()
        .filter(|remote| short.starts_with(&format!("{remote}/")))
        .max_by_key(|name| name.len())
        .context("원격 브랜치에 대응하는 원격 저장소를 찾지 못했습니다.")?;
    let local = short[remote.len() + 1..].to_owned();
    validate_name(path, &local)?;
    let needs_creation = !local_exists(path, &local)?;
    Ok(CheckoutTarget {
        local,
        remote: Some(reference),
        needs_creation,
    })
}

pub fn checkout(path: &Path, target: &CheckoutTarget, allow_create: bool) -> Result<()> {
    validate_name(path, &target.local)?;
    if local_exists(path, &target.local)? {
        execute(path, &["switch", "--no-guess", "--", &target.local])?;
    } else {
        if !allow_create {
            bail!("로컬 브랜치 생성 확인이 필요합니다.");
        }
        let remote = target
            .remote
            .as_deref()
            .filter(|remote| remote.starts_with("refs/remotes/"))
            .context("추적할 원격 브랜치가 없습니다.")?;
        if !reference_exists(path, remote)? {
            bail!("추적할 원격 브랜치가 더 이상 존재하지 않습니다.");
        }
        execute(
            path,
            &["switch", "--track", "-c", &target.local, "--", remote],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(repo: &Path, args: &[&str]) -> String {
        let output = command(repo).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn repository() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        git(temp.path(), &["init", "-b", "main"]);
        git(temp.path(), &["config", "user.name", "Test"]);
        git(
            temp.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(temp.path(), &["config", "commit.gpgsign", "false"]);
        std::fs::write(temp.path().join("file.txt"), "first").unwrap();
        git(temp.path(), &["add", "."]);
        git(temp.path(), &["commit", "-m", "first"]);
        temp
    }

    #[test]
    fn create_uses_selected_commit_and_never_switches_or_overwrites() {
        let repo = repository();
        let first = git(repo.path(), &["rev-parse", "HEAD"]);
        std::fs::write(repo.path().join("file.txt"), "second").unwrap();
        git(repo.path(), &["commit", "-am", "second"]);
        let head = git(repo.path(), &["rev-parse", "HEAD"]);
        create(repo.path(), "기능/선택커밋", &first).unwrap();
        assert_eq!(
            git(repo.path(), &["rev-parse", "refs/heads/기능/선택커밋"]),
            first
        );
        assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), head);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
            "second"
        );
        assert!(create(repo.path(), "기능/선택커밋", &head).is_err());
        for name in [
            "",
            "-flag",
            "bad..name",
            "a b",
            "@{-1}",
            "HEAD",
            "bad\nname",
        ] {
            assert!(create(repo.path(), name, &head).is_err(), "{name}");
        }
        assert_eq!(
            git(repo.path(), &["rev-parse", "refs/heads/기능/선택커밋"]),
            first
        );
    }

    #[test]
    fn local_and_existing_remote_local_switch_without_moving_the_local_tip() {
        let repo = repository();
        let first = git(repo.path(), &["rev-parse", "HEAD"]);
        git(repo.path(), &["branch", "feature/nested"]);
        git(
            repo.path(),
            &["remote", "add", "origin", "https://example.invalid/repo"],
        );
        git(
            repo.path(),
            &["update-ref", "refs/remotes/origin/feature/nested", &first],
        );
        let target = prepare_checkout(repo.path(), "refs/remotes/origin/feature/nested").unwrap();
        assert!(!target.needs_creation);
        assert_eq!(target.local, "feature/nested");
        checkout(repo.path(), &target, false).unwrap();
        assert_eq!(
            git(repo.path(), &["branch", "--show-current"]),
            "feature/nested"
        );
        let main = prepare_checkout(repo.path(), "refs/heads/main").unwrap();
        checkout(repo.path(), &main, false).unwrap();
        assert_eq!(git(repo.path(), &["branch", "--show-current"]), "main");
    }

    #[test]
    fn remote_creation_requires_confirmation_and_sets_tracking_including_symbolic_head() {
        let repo = repository();
        let head = git(repo.path(), &["rev-parse", "HEAD"]);
        git(
            repo.path(),
            &["remote", "add", "origin", "https://example.invalid/repo"],
        );
        git(
            repo.path(),
            &["update-ref", "refs/remotes/origin/feature/new", &head],
        );
        git(
            repo.path(),
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/feature/new",
            ],
        );
        let target = prepare_checkout(repo.path(), "refs/remotes/origin/HEAD").unwrap();
        assert!(target.needs_creation);
        assert_eq!(target.local, "feature/new");
        assert!(checkout(repo.path(), &target, false).is_err());
        assert!(!local_exists(repo.path(), "feature/new").unwrap());
        assert_eq!(git(repo.path(), &["branch", "--show-current"]), "main");
        checkout(repo.path(), &target, true).unwrap();
        assert_eq!(
            git(repo.path(), &["branch", "--show-current"]),
            "feature/new"
        );
        assert_eq!(
            git(repo.path(), &["config", "branch.feature/new.remote"]),
            "origin"
        );
        assert_eq!(
            git(repo.path(), &["config", "branch.feature/new.merge"]),
            "refs/heads/feature/new"
        );
    }

    #[test]
    fn checkout_conflicts_preserve_head_and_worktree() {
        let repo = repository();
        git(repo.path(), &["branch", "old"]);
        std::fs::write(repo.path().join("file.txt"), "committed").unwrap();
        git(repo.path(), &["commit", "-am", "second"]);
        let before = git(repo.path(), &["rev-parse", "HEAD"]);
        std::fs::write(repo.path().join("file.txt"), "unsaved").unwrap();
        let target = prepare_checkout(repo.path(), "refs/heads/old").unwrap();
        assert!(checkout(repo.path(), &target, false).is_err());
        assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), before);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
            "unsaved"
        );
        assert!(prepare_checkout(repo.path(), "refs/tags/tag").is_err());
        let old = git(repo.path(), &["rev-parse", "refs/heads/old"]);
        git(
            repo.path(),
            &["remote", "add", "origin", "https://example.invalid/repo"],
        );
        git(
            repo.path(),
            &["update-ref", "refs/remotes/origin/new", &old],
        );
        let remote = prepare_checkout(repo.path(), "refs/remotes/origin/new").unwrap();
        assert!(checkout(repo.path(), &remote, true).is_err());
        assert!(!local_exists(repo.path(), "new").unwrap());
        assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), before);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
            "unsaved"
        );
    }

    #[test]
    fn branch_checked_out_in_another_worktree_is_not_forced() {
        let repo = repository();
        let worktree = tempfile::tempdir().unwrap();
        git(repo.path(), &["branch", "occupied"]);
        git(
            repo.path(),
            &[
                "worktree",
                "add",
                worktree.path().to_str().unwrap(),
                "occupied",
            ],
        );
        let before = git(repo.path(), &["rev-parse", "HEAD"]);
        let target = prepare_checkout(repo.path(), "refs/heads/occupied").unwrap();
        assert!(checkout(repo.path(), &target, false).is_err());
        assert_eq!(git(repo.path(), &["branch", "--show-current"]), "main");
        assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), before);
    }
}
