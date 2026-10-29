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

#[derive(Clone)]
pub struct CheckoutCandidate {
    pub reference: String,
    pub label: String,
}

pub fn commit_checkout_candidates(path: &Path, commit: &str) -> Result<Vec<CheckoutCandidate>> {
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("올바른 커밋을 선택하세요.");
    }
    execute(
        path,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )?;
    let refs = execute(
        path,
        &[
            "for-each-ref",
            "--format=%(refname)%09%(objectname)%09%(symref)",
            "refs/heads/",
            "refs/remotes/",
        ],
    )?;
    let locals: std::collections::HashSet<&str> = refs
        .lines()
        .filter_map(|line| line.split('\t').next()?.strip_prefix("refs/heads/"))
        .collect();
    let remotes = execute(path, &["remote"])?;
    let mut candidates = Vec::new();
    for line in refs.lines() {
        let mut fields = line.split('\t');
        let reference = fields.next().unwrap_or_default();
        let target = fields.next().unwrap_or_default();
        let symbolic = fields.next().unwrap_or_default();
        if target != commit || !symbolic.is_empty() {
            continue;
        }
        let label = if let Some(name) = reference.strip_prefix("refs/heads/") {
            name.to_owned()
        } else if let Some(name) = reference.strip_prefix("refs/remotes/") {
            let branch = remotes
                .lines()
                .filter(|remote| name.starts_with(&format!("{remote}/")))
                .max_by_key(|remote| remote.len())
                .map(|remote| &name[remote.len() + 1..])
                .or_else(|| name.split_once('/').map(|(_, branch)| branch));
            if branch.is_some_and(|branch| locals.contains(branch)) {
                continue;
            }
            name.to_owned()
        } else {
            continue;
        };
        candidates.push(CheckoutCandidate {
            reference: reference.to_owned(),
            label,
        });
    }
    candidates.sort_by(|a, b| a.reference.cmp(&b.reference));
    Ok(candidates)
}

pub fn prepare_commit_branch(path: &Path, reference: &str, commit: &str) -> Result<CheckoutTarget> {
    if !commit_checkout_candidates(path, commit)?
        .iter()
        .any(|candidate| candidate.reference == reference)
    {
        bail!("선택 브랜치가 변경되었습니다. 새로고침 후 다시 선택하세요.");
    }
    let target = prepare_checkout(path, reference)?;
    if !target.needs_creation {
        let local = execute(
            path,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{}", target.local),
            ],
        )?;
        if local != commit {
            bail!("같은 이름의 Local 브랜치가 다른 커밋에 있습니다. 브랜치를 확인하세요.");
        }
    }
    Ok(target)
}

pub fn checkout_commit_branch(
    path: &Path,
    reference: &str,
    commit: &str,
    allow_create: bool,
) -> Result<()> {
    let target = prepare_commit_branch(path, reference, commit)?;
    checkout(path, &target, allow_create)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

impl ResetMode {
    pub const ALL: [Self; 3] = [Self::Soft, Self::Mixed, Self::Hard];
    pub fn label(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Mixed => "mixed",
            Self::Hard => "hard",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Soft => "HEAD만 이동하며 staged·unstaged 변경을 유지합니다.",
            Self::Mixed => "HEAD와 인덱스를 재설정하며 작업 파일은 유지합니다.",
            Self::Hard => {
                "HEAD·인덱스·작업 트리를 재설정합니다. 추적 파일 변경을 폐기하며 대상 경로를 가리는 비추적 파일도 삭제될 수 있습니다."
            }
        }
    }
}

pub fn reset_commit(path: &Path, commit: &str, mode: ResetMode) -> Result<()> {
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("올바른 커밋을 선택하세요.");
    }
    execute(
        path,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )?;
    execute(path, &["reset", &format!("--{}", mode.label()), commit])
        .context("Reset에 실패했습니다.")?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrateKind {
    Merge,
    Rebase,
}

impl IntegrateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Merge => "Merge",
            Self::Rebase => "Rebase",
        }
    }
}

fn validate_operation(path: &Path, commit: &str) -> Result<()> {
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("올바른 커밋을 선택하세요.");
    }
    execute(
        path,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )?;
    for state in [
        "MERGE_HEAD",
        "REVERT_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ] {
        let location = execute(path, &["rev-parse", "--git-path", state])?;
        let location = std::path::PathBuf::from(location);
        if (if location.is_absolute() {
            location
        } else {
            path.join(location)
        })
        .exists()
        {
            bail!("진행 중인 Git 작업을 먼저 해결하거나 중단하세요.");
        }
    }
    Ok(())
}

pub fn revert_commit(path: &Path, commit: &str, no_commit: bool) -> Result<()> {
    validate_operation(path, commit)?;
    let selection = format!("{commit}^!");
    execute(
        path,
        &[
            "revert",
            if no_commit {
                "--no-commit"
            } else {
                "--no-edit"
            },
            &selection,
        ],
    )?;
    if no_commit {
        execute(path, &["revert", "--quit"])?;
    }
    Ok(())
}

pub fn integrate_commit(path: &Path, commit: &str, kind: IntegrateKind) -> Result<()> {
    validate_operation(path, commit)?;
    let args = match kind {
        IntegrateKind::Merge => vec!["merge", "--no-edit", commit],
        IntegrateKind::Rebase => vec!["rebase", commit],
    };
    let output = command(path)
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .args(args)
        .output()?;
    if !output.status.success() {
        bail!(
            "{} 실패: {}\n{}",
            kind.label(),
            String::from_utf8_lossy(&output.stderr).trim(),
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeControl {
    Continue,
    Abort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevertControl {
    Continue,
    Abort,
    Skip,
}

impl RevertControl {
    pub fn label(self) -> &'static str {
        match self {
            Self::Continue => "되돌리기 계속",
            Self::Abort => "되돌리기 중단",
            Self::Skip => "현재 되돌리기 건너뛰기",
        }
    }
}

pub fn control_revert(
    path: &Path,
    expected: &feather_git::model::RevertState,
    action: RevertControl,
    no_commit: bool,
) -> Result<()> {
    let current = feather_git::git::revert_state(path)?
        .context("진행 중인 Revert가 없습니다. 새로고침하세요.")?;
    if !current.same_operation(expected) {
        bail!("Revert 대상이나 옵션/현재 브랜치가 변경되었습니다. 창을 다시 여세요.");
    }
    let argument = match action {
        RevertControl::Continue => {
            if !current.conflicts.is_empty() {
                bail!("미해결 충돌 파일이 남아 있습니다. 수정 후 Stage하세요.");
            }
            if current.no_commit.is_some_and(|stored| stored != no_commit) {
                bail!("시작할 때 선택한 커밋 여부를 유지해야 합니다.");
            }
            if no_commit {
                if current.pending_count > 1 {
                    bail!(
                        "여러 커밋의 no commit Revert는 자동 계속을 지원하지 않습니다. 현재 상태와 Stage 내용은 유지됩니다."
                    );
                }
                "--quit"
            } else {
                "--continue"
            }
        }
        RevertControl::Abort => "--abort",
        RevertControl::Skip => "--skip",
    };
    let output = command(path)
        .env("GIT_EDITOR", "true")
        .args(["revert", argument])
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    if action != RevertControl::Abort
        && let Some(next) = feather_git::git::revert_state(path)?
        && next.same_session(&current)
        && !next.same_operation(&current)
        && !next.conflicts.is_empty()
    {
        return Ok(());
    }
    bail!(
        "{} 실패: {}{}",
        action.label(),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebaseControl {
    Continue,
    Abort,
    Skip,
}

impl RebaseControl {
    pub fn label(self) -> &'static str {
        match self {
            Self::Continue => "리베이스 계속",
            Self::Abort => "리베이스 중단",
            Self::Skip => "커밋 건너뛰기",
        }
    }
}

pub fn control_rebase(
    path: &Path,
    expected: &feather_git::model::RebaseState,
    action: RebaseControl,
) -> Result<()> {
    let current = feather_git::git::rebase_state(path)?
        .context("진행 중인 Rebase가 없습니다. 새로고침하세요.")?;
    if !current.same_operation(expected)
        || (current.branch.starts_with("refs/heads/")
            && current.branch_tip.as_deref() != Some(current.original_head.as_str()))
    {
        bail!("Rebase 단계나 대상 브랜치가 변경되었습니다. 창을 다시 여세요.");
    }
    if action == RebaseControl::Continue && !current.conflicts.is_empty() {
        bail!("미해결 충돌 파일이 남아 있습니다. 수정 후 Stage하세요.");
    }
    let output = command(path)
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .args([
            "rebase",
            match action {
                RebaseControl::Continue => "--continue",
                RebaseControl::Abort => "--abort",
                RebaseControl::Skip => "--skip",
            },
        ])
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    if action != RebaseControl::Abort
        && let Some(next) = feather_git::git::rebase_state(path)?
        && next.same_session(&current)
        && !next.same_operation(&current)
        && !next.conflicts.is_empty()
    {
        return Ok(());
    }
    bail!(
        "{} 실패: {}{}",
        action.label(),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
}

impl MergeControl {
    pub fn label(self) -> &'static str {
        match self {
            Self::Continue => "병합 완료",
            Self::Abort => "병합 중단",
        }
    }
}

pub fn control_merge(
    path: &Path,
    expected: &feather_git::model::MergeState,
    action: MergeControl,
) -> Result<()> {
    let current = feather_git::git::merge_state(path)?
        .context("진행 중인 Merge가 없습니다. 새로고침하세요.")?;
    if !current.same_operation(expected) {
        bail!("병합 대상이나 현재 브랜치가 변경되었습니다. 창을 다시 여세요.");
    }
    if action == MergeControl::Continue && !current.conflicts.is_empty() {
        bail!("미해결 충돌 파일이 남아 있습니다. 수정 후 Stage하세요.");
    }
    let output = command(path)
        .env("GIT_EDITOR", "true")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .args([
            "merge",
            match action {
                MergeControl::Continue => "--continue",
                MergeControl::Abort => "--abort",
            },
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "{} 실패: {}{}",
            action.label(),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

pub fn create_options(
    path: &Path,
    name: &str,
    commit: &str,
    remote: bool,
    switch: bool,
) -> Result<()> {
    create_with_remote(path, name, commit, remote)?;
    if switch {
        let target = prepare_checkout(path, &format!("refs/heads/{name}"))?;
        checkout(path, &target, false)?;
    }
    Ok(())
}

pub fn create_with_remote(path: &Path, name: &str, commit: &str, remote: bool) -> Result<()> {
    let target = if remote {
        let remotes = execute(path, &["remote"])?;
        let names: Vec<_> = remotes.lines().collect();
        Some(if names.contains(&"origin") {
            "origin".to_owned()
        } else if names.len() == 1 {
            names[0].to_owned()
        } else {
            bail!("원격이 없거나 여러 개입니다. origin 원격을 설정하세요.");
        })
    } else {
        None
    };
    create(path, name, commit)?;
    if let Some(remote) = target {
        publish_branch(path, &format!("refs/heads/{name}"), commit, &remote).context(
            "Local 브랜치는 생성되었지만 원격 생성에 실패했습니다. Local 목록의 u로 재시도하세요.",
        )?;
    }
    Ok(())
}

pub fn publish_branch(path: &Path, reference: &str, expected: &str, remote: &str) -> Result<()> {
    let name = reference
        .strip_prefix("refs/heads/")
        .context("Local 브랜치를 선택하세요.")?;
    validate_name(path, name)?;
    if ![40, 64].contains(&expected.len()) || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("브랜치 대상이 올바르지 않습니다.");
    }
    if execute(path, &["rev-parse", "--verify", reference])? != expected {
        bail!("브랜치가 변경되었습니다. 새로고침 후 다시 선택하세요.");
    }
    if remote.is_empty()
        || !execute(path, &["remote"])?
            .lines()
            .any(|name| name == remote)
    {
        bail!("등록된 원격 이름을 입력하세요.");
    }
    // 빈 기대 해시는 서버에 아직 없는 브랜치만 생성하도록 제한한다.
    execute(
        path,
        &[
            "push",
            "-u",
            &format!("--force-with-lease={reference}:"),
            "--",
            remote,
            &format!("{reference}:{reference}"),
        ],
    )?;
    Ok(())
}

pub fn checkout_commit(path: &Path, commit: &str) -> Result<()> {
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("올바른 커밋을 선택하세요.");
    }
    execute(
        path,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )?;
    execute(path, &["switch", "--detach", "--", commit])?;
    Ok(())
}

pub fn create_tag(path: &Path, name: &str, commit: &str) -> Result<()> {
    if name.is_empty() || name.starts_with('-') || name.chars().any(char::is_control) {
        bail!("올바른 태그명을 입력하세요.");
    }
    execute(path, &["check-ref-format", &format!("refs/tags/{name}")])?;
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("선택한 커밋 ID가 올바르지 않습니다.");
    }
    execute(
        path,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )?;
    execute(
        path,
        &["-c", "tag.gpgSign=false", "tag", "--", name, commit],
    )?;
    Ok(())
}

fn validate_delete_reference(path: &Path, reference: &str, expected: &str) -> Result<()> {
    if ![40, 64].contains(&expected.len()) || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("참조 대상이 올바르지 않습니다.");
    }
    if !["refs/heads/", "refs/remotes/", "refs/tags/"]
        .iter()
        .any(|p| reference.starts_with(p))
    {
        bail!("삭제할 브랜치 또는 태그를 선택하세요.");
    }
    execute(path, &["check-ref-format", reference])?;
    let symbolic = command(path)
        .args(["symbolic-ref", "--quiet", reference])
        .output()?;
    if symbolic.status.success() {
        bail!("심볼릭 참조는 삭제할 수 없습니다.");
    }
    let current = execute(
        path,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )?;
    if current != expected {
        bail!("참조 대상이 변경되었습니다. 새로고침 후 다시 선택하세요.");
    }
    Ok(())
}

pub fn delete_reference(path: &Path, reference: &str, expected: &str, server: bool) -> Result<()> {
    validate_delete_reference(path, reference, expected)?;
    if let Some(name) = reference.strip_prefix("refs/heads/") {
        execute(path, &["branch", "-d", "--", name])?;
    } else if let Some(name) = reference.strip_prefix("refs/tags/") {
        execute(path, &["tag", "-d", "--", name])?;
    } else if let Some(short) = reference.strip_prefix("refs/remotes/") {
        if server {
            let remotes = execute(path, &["remote"])?;
            let remote = remotes
                .lines()
                .filter(|r| short.starts_with(&format!("{r}/")))
                .max_by_key(|r| r.len())
                .context("대응하는 원격 저장소가 없습니다.")?;
            let branch = &short[remote.len() + 1..];
            let destination = format!("refs/heads/{branch}");
            execute(
                path,
                &[
                    "push",
                    &format!("--force-with-lease={destination}:{expected}"),
                    "--",
                    remote,
                    &format!(":{destination}"),
                ],
            )?;
            // 사용자 정의 push 설정에서도 로컬 추적 참조를 정리한다.
            if reference_exists(path, reference)? {
                execute(path, &["update-ref", "-d", reference, expected])?;
            }
        } else {
            execute(path, &["branch", "-d", "-r", "--", short])?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteKind {
    Local,
    Remote,
    Tag,
}

impl DeleteKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Local => "Local 브랜치 삭제",
            Self::Remote => "Remote 브랜치 삭제",
            Self::Tag => "Tag 삭제",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeletion {
    pub remote: String,
    pub destination: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpstreamCandidate {
    pub reference: String,
    pub label: String,
    pub expected: String,
    pub mapping: RemoteDeletion,
}

#[derive(Clone)]
pub struct UpstreamPlan {
    pub reference: String,
    pub expected: String,
    pub current: String,
    pub candidates: Vec<UpstreamCandidate>,
}

pub fn plan_upstream(path: &Path, reference: &str, expected: &str) -> Result<UpstreamPlan> {
    let local = reference
        .strip_prefix("refs/heads/")
        .context("Local 브랜치를 선택하세요.")?;
    validate_name(path, local)?;
    if ![40, 64].contains(&expected.len())
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
        || execute(path, &["rev-parse", "--verify", reference])? != expected
    {
        bail!("Local 브랜치가 변경되었습니다. 창을 다시 여세요.");
    }
    let mut mappings = Vec::new();
    for remote in execute(path, &["remote"])?.lines() {
        let output = command(path)
            .args(["config", "--get-all", &format!("remote.{remote}.fetch")])
            .output()?;
        if output.status.success() {
            for spec in String::from_utf8(output.stdout)?.lines() {
                mappings.push((remote.to_owned(), spec.to_owned()));
            }
        }
    }
    let rows = execute(
        path,
        &[
            "for-each-ref",
            "--format=%(refname)%09%(objectname)%09%(symref)%09%(objecttype)%09%(upstream)%09END",
        ],
    )?;
    let mut current = String::new();
    let mut candidates = Vec::new();
    for row in rows.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() != 6 {
            continue;
        }
        if fields[0] == reference {
            current = fields[4].into();
        }
        if fields[0].starts_with("refs/heads/") || !fields[2].is_empty() || fields[3] != "commit" {
            continue;
        }
        let mut matches: Vec<_> = mappings
            .iter()
            .filter_map(|(remote, spec)| {
                refspec_source(spec, fields[0])
                    .filter(|source| source.starts_with("refs/heads/"))
                    .map(|source| (remote.clone(), source))
            })
            .collect();
        matches.sort();
        matches.dedup();
        if matches.len() != 1 {
            continue;
        }
        let (remote, destination) = matches.remove(0);
        candidates.push(UpstreamCandidate {
            reference: fields[0].into(),
            label: format!(
                "{remote}/{}",
                destination
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&destination)
            ),
            expected: fields[1].into(),
            mapping: RemoteDeletion {
                remote,
                destination,
            },
        });
    }
    candidates.sort_by(|a, b| {
        a.label
            .cmp(&b.label)
            .then_with(|| a.reference.cmp(&b.reference))
    });
    Ok(UpstreamPlan {
        reference: reference.into(),
        expected: expected.into(),
        current,
        candidates,
    })
}

pub fn connect_upstream(path: &Path, plan: &UpstreamPlan, selected: usize) -> Result<()> {
    let candidate = plan
        .candidates
        .get(selected)
        .context("연결할 원격 브랜치를 선택하세요.")?;
    let refreshed = plan_upstream(path, &plan.reference, &plan.expected)?;
    if refreshed.current != plan.current || !refreshed.candidates.contains(candidate) {
        bail!("원격 브랜치 또는 연결 설정이 변경되었습니다. 창을 다시 여세요.");
    }
    let local = plan
        .reference
        .strip_prefix("refs/heads/")
        .context("Local 브랜치를 선택하세요.")?;
    execute(
        path,
        &[
            "branch",
            &format!("--set-upstream-to={}", candidate.reference),
            "--",
            local,
        ],
    )?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteReference {
    pub reference: String,
    pub expected: String,
    pub remote: Option<RemoteDeletion>,
}

impl DeleteReference {
    pub fn label(&self) -> &str {
        self.reference
            .strip_prefix("refs/heads/")
            .or_else(|| self.reference.strip_prefix("refs/remotes/"))
            .or_else(|| self.reference.strip_prefix("refs/tags/"))
            .unwrap_or(&self.reference)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletePlan {
    pub kind: DeleteKind,
    pub primary: DeleteReference,
    pub related: Vec<DeleteReference>,
}

fn refspec_source(spec: &str, reference: &str) -> Option<String> {
    let (source, destination) = spec.strip_prefix('+').unwrap_or(spec).split_once(':')?;
    if let Some((prefix, suffix)) = destination.split_once('*') {
        if suffix.contains('*') || source.matches('*').count() != 1 {
            return None;
        }
        let captured = reference.strip_prefix(prefix)?.strip_suffix(suffix)?;
        Some(source.replace('*', captured))
    } else if destination == reference && !source.contains('*') {
        Some(source.to_owned())
    } else {
        None
    }
}

fn remote_deletion(path: &Path, reference: &str) -> Result<RemoteDeletion> {
    let mut matches = Vec::new();
    for remote in execute(path, &["remote"])?.lines() {
        let output = command(path)
            .args(["config", "--get-all", &format!("remote.{remote}.fetch")])
            .output()?;
        if !output.status.success() {
            continue;
        }
        for spec in String::from_utf8(output.stdout)?.lines() {
            if let Some(destination) = refspec_source(spec, reference)
                && destination.starts_with("refs/heads/")
            {
                execute(path, &["check-ref-format", &destination])?;
                matches.push((remote.to_owned(), destination));
            }
        }
    }
    matches.sort();
    matches.dedup();
    if matches.len() != 1 {
        bail!("삭제할 서버 브랜치를 원격 fetch 설정에서 하나로 확인할 수 없습니다.");
    }
    let (remote, destination) = matches.remove(0);
    Ok(RemoteDeletion {
        remote,
        destination,
    })
}

fn deletion_reference(path: &Path, reference: &str, expected: &str) -> Result<DeleteReference> {
    validate_delete_reference(path, reference, expected)?;
    let remote = if reference.starts_with("refs/remotes/") {
        Some(remote_deletion(path, reference)?)
    } else {
        None
    };
    Ok(DeleteReference {
        reference: reference.into(),
        expected: expected.into(),
        remote,
    })
}

pub fn plan_reference_deletion(path: &Path, reference: &str, expected: &str) -> Result<DeletePlan> {
    let primary = deletion_reference(path, reference, expected)?;
    let kind = if reference.starts_with("refs/heads/") {
        DeleteKind::Local
    } else if reference.starts_with("refs/remotes/") {
        DeleteKind::Remote
    } else {
        DeleteKind::Tag
    };
    let mut related = Vec::new();
    if kind != DeleteKind::Tag {
        let rows = execute(
            path,
            &[
                "for-each-ref",
                "--format=%(refname)%09%(objectname)%09%(upstream)",
                "refs/heads/",
                "refs/remotes/",
            ],
        )?;
        let refs: std::collections::BTreeMap<&str, (&str, &str)> = rows
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                Some((
                    fields.next()?,
                    (fields.next()?, fields.next().unwrap_or_default()),
                ))
            })
            .collect();
        if kind == DeleteKind::Local {
            if let Some((_, upstream)) = refs.get(reference)
                && upstream.starts_with("refs/remotes/")
                && let Some((hash, _)) = refs.get(upstream)
            {
                related.push(deletion_reference(path, upstream, hash)?);
            }
        } else {
            for (local, (hash, upstream)) in refs {
                if local.starts_with("refs/heads/") && upstream == reference {
                    related.push(deletion_reference(path, local, hash)?);
                }
            }
        }
    }
    Ok(DeletePlan {
        kind,
        primary,
        related,
    })
}

fn preflight_local_deletion(path: &Path, target: &DeleteReference) -> Result<()> {
    validate_delete_reference(path, &target.reference, &target.expected)?;
    let worktrees = execute(path, &["worktree", "list", "--porcelain"])?;
    if worktrees
        .lines()
        .any(|line| line.strip_prefix("branch ") == Some(target.reference.as_str()))
    {
        bail!(
            "사용 중인 로컬 브랜치는 삭제할 수 없습니다: {}",
            target.label()
        );
    }
    let merged = command(path)
        .args(["merge-base", "--is-ancestor", &target.reference, "HEAD"])
        .output()?;
    if !merged.status.success() {
        bail!(
            "현재 HEAD에 병합되지 않은 로컬 브랜치는 삭제하지 않습니다: {}",
            target.label()
        );
    }
    // branch -d also protects commits not merged into the configured upstream.
    let upstream = command(path)
        .args([
            "rev-parse",
            "--verify",
            &format!(
                "{}@{{upstream}}",
                target
                    .reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&target.reference)
            ),
        ])
        .output()?;
    if upstream.status.success() {
        let upstream = String::from_utf8(upstream.stdout)?.trim().to_owned();
        let merged = command(path)
            .args(["merge-base", "--is-ancestor", &target.reference, &upstream])
            .output()?;
        if !merged.status.success() {
            bail!(
                "추적 원격에 병합되지 않은 로컬 브랜치는 삭제하지 않습니다: {}",
                target.label()
            );
        }
    }
    Ok(())
}

fn delete_server_reference(path: &Path, target: &DeleteReference) -> Result<()> {
    let remote = target
        .remote
        .as_ref()
        .context("서버 삭제 대상이 없습니다.")?;
    execute(
        path,
        &[
            "push",
            &format!(
                "--force-with-lease={}:{}",
                remote.destination, target.expected
            ),
            "--",
            &remote.remote,
            &format!(":{}", remote.destination),
        ],
    )?;
    if reference_exists(path, &target.reference)? {
        execute(
            path,
            &["update-ref", "-d", &target.reference, &target.expected],
        )
        .context(
            "서버 브랜치는 삭제되었지만 추적 참조 정리에 실패했습니다. 새로고침 후 확인하세요.",
        )?;
    }
    Ok(())
}

pub fn delete_reference_plan(path: &Path, plan: &DeletePlan, delete_related: bool) -> Result<()> {
    let current = plan_reference_deletion(path, &plan.primary.reference, &plan.primary.expected)?;
    if current.primary != plan.primary || (delete_related && current != *plan) {
        bail!("브랜치 연결 또는 대상이 변경되었습니다. 팝업을 다시 열어 확인하세요.");
    }
    if delete_related && plan.related.is_empty() {
        bail!("함께 삭제할 연결 브랜치가 없습니다.");
    }
    let targets: Vec<_> = std::iter::once(&plan.primary)
        .chain(plan.related.iter().filter(|_| delete_related))
        .collect();
    let paired_with_remote = targets.iter().any(|target| target.remote.is_some());
    for target in &targets {
        validate_delete_reference(path, &target.reference, &target.expected)?;
        if paired_with_remote && target.reference.starts_with("refs/heads/") {
            preflight_local_deletion(path, target)?;
        }
    }
    let mut remote_deleted = false;
    for target in targets.iter().filter(|target| target.remote.is_some()) {
        delete_server_reference(path, target)?;
        remote_deleted = true;
    }
    for target in targets.iter().filter(|target| target.remote.is_none()) {
        delete_reference(path, &target.reference, &target.expected, false).with_context(|| {
            if remote_deleted { format!("원격 브랜치는 삭제되었지만 {} 삭제에 실패했습니다. 일부 작업이 완료되었으니 새로고침 후 확인하세요.", target.label()) }
            else { format!("{} 삭제에 실패했습니다.", target.label()) }
        })?;
    }
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

pub fn is_current_branch(path: &Path, reference: &str) -> Result<bool> {
    let output = command(path)
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .output()?;
    match output.status.code() {
        Some(0) => Ok(String::from_utf8(output.stdout)?.trim() == reference),
        Some(1) => Ok(false),
        _ => bail!("현재 브랜치를 확인하지 못했습니다."),
    }
}

pub fn checkout(path: &Path, target: &CheckoutTarget, allow_create: bool) -> Result<()> {
    validate_name(path, &target.local)?;
    if is_current_branch(path, &format!("refs/heads/{}", target.local))? {
        return Ok(());
    }
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

    fn deletion_repository() -> (tempfile::TempDir, tempfile::TempDir, String) {
        let repo = repository();
        let bare = tempfile::tempdir().unwrap();
        git(bare.path(), &["init", "--bare"]);
        git(
            repo.path(),
            &["remote", "add", "origin", bare.path().to_str().unwrap()],
        );
        git(
            repo.path(),
            &["push", "origin", "HEAD:refs/heads/server-topic"],
        );
        git(repo.path(), &["branch", "different-local"]);
        git(
            repo.path(),
            &[
                "branch",
                "--set-upstream-to=origin/server-topic",
                "different-local",
            ],
        );
        let hash = git(repo.path(), &["rev-parse", "HEAD"]);
        (repo, bare, hash)
    }

    #[test]
    fn upstream_connects_selected_local_without_checkout_push_or_worktree_changes() {
        let repo = repository();
        let path = repo.path();
        let head = git(path, &["rev-parse", "HEAD"]);
        git(path, &["remote", "add", "origin", "unused-origin"]);
        git(path, &["remote", "add", "backup", "unused-backup"]);
        git(path, &["branch", "topic"]);
        git(path, &["update-ref", "refs/remotes/origin/main", &head]);
        git(path, &["update-ref", "refs/remotes/backup/other", &head]);
        git(
            path,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
            ],
        );
        std::fs::write(path.join("file.txt"), "unsaved").unwrap();
        let index = std::fs::read(path.join(".git/index")).unwrap();
        let plan = plan_upstream(path, "refs/heads/topic", &head).unwrap();
        assert!(plan.current.is_empty());
        assert_eq!(
            plan.candidates
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>(),
            vec!["backup/other", "origin/main"]
        );
        connect_upstream(path, &plan, 0).unwrap();
        assert_eq!(git(path, &["config", "branch.topic.remote"]), "backup");
        assert_eq!(
            git(path, &["config", "branch.topic.merge"]),
            "refs/heads/other"
        );
        let plan = plan_upstream(path, "refs/heads/topic", &head).unwrap();
        assert_eq!(plan.current, "refs/remotes/backup/other");
        connect_upstream(path, &plan, 1).unwrap();
        assert_eq!(git(path, &["config", "branch.topic.remote"]), "origin");
        let plan = plan_upstream(path, "refs/heads/main", &head).unwrap();
        connect_upstream(path, &plan, 0).unwrap();
        assert_eq!(git(path, &["config", "branch.main.remote"]), "backup");
        assert_eq!(git(path, &["branch", "--show-current"]), "main");
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(path, &["rev-parse", "refs/heads/topic"]), head);
        assert_eq!(git(path, &["rev-parse", "refs/remotes/backup/other"]), head);
        assert_eq!(std::fs::read(path.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "unsaved"
        );
    }

    #[test]
    fn upstream_excludes_ambiguous_mappings_and_rejects_deleted_local_without_config_writes() {
        let repo = repository();
        let path = repo.path();
        let head = git(path, &["rev-parse", "HEAD"]);
        git(path, &["remote", "add", "one", "unused-one"]);
        git(path, &["remote", "add", "two", "unused-two"]);
        git(
            path,
            &["config", "remote.one.fetch", "+refs/heads/*:refs/shared/*"],
        );
        git(
            path,
            &["config", "remote.two.fetch", "+refs/heads/*:refs/shared/*"],
        );
        git(path, &["update-ref", "refs/shared/topic", &head]);
        git(path, &["branch", "local"]);
        let empty = plan_upstream(path, "refs/heads/local", &head).unwrap();
        assert!(empty.candidates.is_empty());
        let before = std::fs::read(path.join(".git/config")).unwrap();
        assert!(connect_upstream(path, &empty, 0).is_err());
        assert_eq!(std::fs::read(path.join(".git/config")).unwrap(), before);
        git(
            path,
            &["config", "remote.two.fetch", "+refs/heads/*:refs/other/*"],
        );
        let plan = plan_upstream(path, "refs/heads/local", &head).unwrap();
        assert_eq!(plan.candidates.len(), 1);
        git(path, &["branch", "-d", "local"]);
        let before = std::fs::read(path.join(".git/config")).unwrap();
        assert!(connect_upstream(path, &plan, 0).is_err());
        assert_eq!(std::fs::read(path.join(".git/config")).unwrap(), before);
    }

    #[test]
    fn upstream_supports_custom_fetch_mapping_and_rejects_stale_or_missing_targets() {
        let repo = repository();
        let path = repo.path();
        let head = git(path, &["rev-parse", "HEAD"]);
        assert!(
            plan_upstream(path, "refs/heads/main", &head)
                .unwrap()
                .candidates
                .is_empty()
        );
        git(path, &["remote", "add", "custom", "unused"]);
        git(
            path,
            &[
                "config",
                "remote.custom.fetch",
                "+refs/heads/server/*:refs/custom/*",
            ],
        );
        git(path, &["update-ref", "refs/custom/nested/topic", &head]);
        let plan = plan_upstream(path, "refs/heads/main", &head).unwrap();
        assert_eq!(plan.candidates[0].label, "custom/server/nested/topic");
        connect_upstream(path, &plan, 0).unwrap();
        assert_eq!(
            git(path, &["config", "branch.main.merge"]),
            "refs/heads/server/nested/topic"
        );
        assert!(connect_upstream(path, &plan, 0).is_err());
        let plan = plan_upstream(path, "refs/heads/main", &head).unwrap();
        git(
            path,
            &[
                "config",
                "remote.custom.fetch",
                "+refs/heads/changed/*:refs/custom/*",
            ],
        );
        assert!(connect_upstream(path, &plan, 0).is_err());
        git(
            path,
            &[
                "config",
                "remote.custom.fetch",
                "+refs/heads/server/*:refs/custom/*",
            ],
        );
        let plan = plan_upstream(path, "refs/heads/main", &head).unwrap();
        git(path, &["commit", "--allow-empty", "-m", "new"]);
        assert!(connect_upstream(path, &plan, 0).is_err());
        let new_head = git(path, &["rev-parse", "HEAD"]);
        let plan = plan_upstream(path, "refs/heads/main", &new_head).unwrap();
        git(path, &["update-ref", "refs/custom/nested/topic", &new_head]);
        assert!(connect_upstream(path, &plan, 0).is_err());
        let plan = plan_upstream(path, "refs/heads/main", &new_head).unwrap();
        git(path, &["update-ref", "-d", "refs/custom/nested/topic"]);
        assert!(connect_upstream(path, &plan, 0).is_err());
        assert!(plan_upstream(path, "refs/remotes/custom/topic", &new_head).is_err());
        assert!(connect_upstream(path, &plan, 10).is_err());
    }

    #[test]
    fn local_deletion_uses_upstream_and_optionally_deletes_the_server_branch() {
        for include_related in [false, true] {
            let (repo, bare, hash) = deletion_repository();
            let plan =
                plan_reference_deletion(repo.path(), "refs/heads/different-local", &hash).unwrap();
            assert_eq!(plan.kind, DeleteKind::Local);
            assert_eq!(plan.related[0].label(), "origin/server-topic");
            delete_reference_plan(repo.path(), &plan, include_related).unwrap();
            assert!(!reference_exists(repo.path(), "refs/heads/different-local").unwrap());
            assert_eq!(
                reference_exists(bare.path(), "refs/heads/server-topic").unwrap(),
                !include_related
            );
            assert_eq!(
                reference_exists(repo.path(), "refs/remotes/origin/server-topic").unwrap(),
                !include_related
            );
        }
    }

    #[test]
    fn local_only_deletion_preserves_standard_upstream_merge_policy() {
        let (repo, bare, _) = deletion_repository();
        git(repo.path(), &["checkout", "different-local"]);
        git(repo.path(), &["commit", "--allow-empty", "-m", "published"]);
        git(
            repo.path(),
            &["push", "origin", "HEAD:refs/heads/server-topic"],
        );
        let hash = git(repo.path(), &["rev-parse", "HEAD"]);
        git(repo.path(), &["checkout", "main"]);
        let plan =
            plan_reference_deletion(repo.path(), "refs/heads/different-local", &hash).unwrap();
        delete_reference_plan(repo.path(), &plan, false).unwrap();
        assert!(!reference_exists(repo.path(), "refs/heads/different-local").unwrap());
        assert!(reference_exists(bare.path(), "refs/heads/server-topic").unwrap());
    }

    #[test]
    fn remote_deletion_lists_all_tracking_locals_and_deletes_only_when_checked() {
        for include_related in [false, true] {
            let (repo, bare, hash) = deletion_repository();
            git(repo.path(), &["branch", "second-local"]);
            git(
                repo.path(),
                &[
                    "branch",
                    "--set-upstream-to=origin/server-topic",
                    "second-local",
                ],
            );
            let plan =
                plan_reference_deletion(repo.path(), "refs/remotes/origin/server-topic", &hash)
                    .unwrap();
            assert_eq!(plan.kind, DeleteKind::Remote);
            assert_eq!(
                plan.related
                    .iter()
                    .map(DeleteReference::label)
                    .collect::<Vec<_>>(),
                ["different-local", "second-local"]
            );
            delete_reference_plan(repo.path(), &plan, include_related).unwrap();
            assert!(!reference_exists(bare.path(), "refs/heads/server-topic").unwrap());
            for reference in ["refs/heads/different-local", "refs/heads/second-local"] {
                assert_eq!(
                    reference_exists(repo.path(), reference).unwrap(),
                    !include_related
                );
            }
        }
    }

    #[test]
    fn unrelated_local_and_tag_have_no_checkbox_targets() {
        let repo = repository();
        let hash = git(repo.path(), &["rev-parse", "HEAD"]);
        git(repo.path(), &["branch", "untracked"]);
        let plan = plan_reference_deletion(repo.path(), "refs/heads/untracked", &hash).unwrap();
        assert!(plan.related.is_empty());
        assert!(delete_reference_plan(repo.path(), &plan, true).is_err());
        delete_reference_plan(repo.path(), &plan, false).unwrap();
        git(repo.path(), &["tag", "remove-tag"]);
        let plan = plan_reference_deletion(repo.path(), "refs/tags/remove-tag", &hash).unwrap();
        assert_eq!(plan.kind, DeleteKind::Tag);
        assert!(plan.related.is_empty());
        delete_reference_plan(repo.path(), &plan, false).unwrap();
        assert!(!reference_exists(repo.path(), "refs/tags/remove-tag").unwrap());
        let (repo, bare, hash) = deletion_repository();
        git(
            repo.path(),
            &["branch", "--unset-upstream", "different-local"],
        );
        let plan = plan_reference_deletion(repo.path(), "refs/remotes/origin/server-topic", &hash)
            .unwrap();
        assert!(plan.related.is_empty());
        delete_reference_plan(repo.path(), &plan, false).unwrap();
        assert!(!reference_exists(bare.path(), "refs/heads/server-topic").unwrap());
        assert!(reference_exists(repo.path(), "refs/heads/different-local").unwrap());
    }

    #[test]
    fn related_deletion_preflights_current_worktree_other_worktree_and_unmerged_locals() {
        for mode in ["current", "other", "unmerged", "unpublished"] {
            let (repo, bare, hash) = deletion_repository();
            let worktree = tempfile::tempdir().unwrap();
            match mode {
                "current" => {
                    git(
                        repo.path(),
                        &["branch", "--set-upstream-to=origin/server-topic", "main"],
                    );
                }
                "other" => {
                    git(
                        repo.path(),
                        &[
                            "worktree",
                            "add",
                            worktree.path().to_str().unwrap(),
                            "different-local",
                        ],
                    );
                }
                _ => {
                    git(repo.path(), &["checkout", "different-local"]);
                    git(repo.path(), &["commit", "--allow-empty", "-m", "unique"]);
                    git(repo.path(), &["checkout", "main"]);
                    if mode == "unpublished" {
                        git(repo.path(), &["merge", "--ff-only", "different-local"]);
                    }
                }
            }
            let plan =
                plan_reference_deletion(repo.path(), "refs/remotes/origin/server-topic", &hash)
                    .unwrap();
            assert!(
                delete_reference_plan(repo.path(), &plan, true).is_err(),
                "{mode}"
            );
            assert!(reference_exists(bare.path(), "refs/heads/server-topic").unwrap());
            assert!(reference_exists(repo.path(), "refs/heads/different-local").unwrap());
        }
    }

    #[test]
    fn related_deletion_rejects_changed_links_and_remote_lease_before_local_changes() {
        for mode in ["link", "local", "server"] {
            let (repo, bare, hash) = deletion_repository();
            let plan =
                plan_reference_deletion(repo.path(), "refs/heads/different-local", &hash).unwrap();
            match mode {
                "link" => {
                    git(
                        repo.path(),
                        &["branch", "--unset-upstream", "different-local"],
                    );
                }
                "local" => {
                    git(repo.path(), &["commit", "--allow-empty", "-m", "new"]);
                    let new = git(repo.path(), &["rev-parse", "HEAD"]);
                    git(
                        repo.path(),
                        &["update-ref", "refs/heads/different-local", &new],
                    );
                }
                _ => {
                    git(
                        repo.path(),
                        &["commit", "--allow-empty", "-m", "server changed"],
                    );
                    git(
                        bare.path(),
                        &[
                            "fetch",
                            repo.path().to_str().unwrap(),
                            "HEAD:refs/heads/server-topic",
                        ],
                    );
                }
            }
            assert!(
                delete_reference_plan(repo.path(), &plan, true).is_err(),
                "{mode}"
            );
            assert!(reference_exists(repo.path(), "refs/heads/different-local").unwrap());
            assert!(reference_exists(bare.path(), "refs/heads/server-topic").unwrap());
        }
    }

    #[test]
    fn deletion_respects_custom_fetch_destination_without_guessing_names() {
        let (repo, bare, hash) = deletion_repository();
        git(
            repo.path(),
            &[
                "config",
                "--replace-all",
                "remote.origin.fetch",
                "+refs/heads/*:refs/remotes/cache/*",
            ],
        );
        git(repo.path(), &["fetch", "origin"]);
        let plan =
            plan_reference_deletion(repo.path(), "refs/heads/different-local", &hash).unwrap();
        assert_eq!(plan.related[0].reference, "refs/remotes/cache/server-topic");
        assert_eq!(plan.related[0].remote.as_ref().unwrap().remote, "origin");
        assert_eq!(
            plan.related[0].remote.as_ref().unwrap().destination,
            "refs/heads/server-topic"
        );
        delete_reference_plan(repo.path(), &plan, true).unwrap();
        assert!(!reference_exists(bare.path(), "refs/heads/server-topic").unwrap());
        assert!(!reference_exists(repo.path(), "refs/heads/different-local").unwrap());
    }

    #[test]
    fn revert_conflict_continue_preserves_commit_mode_and_prior_staged_changes() {
        for no_commit in [false, true] {
            let (repo, head, target) = conflicted_revert(no_commit, no_commit);
            let path = repo.path();
            let state = feather_git::git::revert_state(path).unwrap().unwrap();
            assert_eq!(state.no_commit, Some(no_commit));
            assert_eq!(state.target, target);
            assert_eq!(state.pending_count, 1);
            assert_eq!(state.conflicts, vec![b"file.txt".to_vec()]);
            assert_eq!(
                feather_git::git::load_initial(path, None).unwrap().revert,
                Some(state.clone())
            );
            assert!(control_revert(path, &state, RevertControl::Continue, no_commit).is_err());
            resolve_rebase_file(path, "file.txt", "resolved revert");
            assert!(control_revert(path, &state, RevertControl::Continue, !no_commit).is_err());
            control_revert(path, &state, RevertControl::Continue, no_commit).unwrap();
            assert!(feather_git::git::revert_state(path).unwrap().is_none());
            assert_eq!(git(path, &["rev-parse", "HEAD"]) == head, no_commit);
            if no_commit {
                assert_eq!(git(path, &["show", ":file.txt"]), "resolved revert");
                assert_eq!(git(path, &["show", ":prior.txt"]), "prior staged");
                assert_eq!(
                    git(path, &["diff", "--cached", "--name-only"]),
                    "file.txt\nprior.txt"
                );
            } else {
                assert_eq!(git(path, &["rev-parse", "HEAD^"]), head);
                assert_eq!(git(path, &["show", "HEAD:file.txt"]), "resolved revert");
                assert!(git(path, &["log", "-1", "--format=%s"]).starts_with("Revert"));
            }
        }
    }

    fn conflicted_revert(
        no_commit: bool,
        prior_staged: bool,
    ) -> (tempfile::TempDir, String, String) {
        let repo = repository();
        let path = repo.path();
        git(path, &["config", "core.autocrlf", "false"]);
        std::fs::write(path.join("file.txt"), "target").unwrap();
        git(path, &["commit", "-am", "target"]);
        let target = git(path, &["rev-parse", "HEAD"]);
        std::fs::write(path.join("file.txt"), "later").unwrap();
        git(path, &["commit", "-am", "later"]);
        let head = git(path, &["rev-parse", "HEAD"]);
        if prior_staged {
            std::fs::write(path.join("prior.txt"), "prior staged").unwrap();
            git(path, &["add", "prior.txt"]);
        }
        assert!(revert_commit(path, &target, no_commit).is_err());
        (repo, head, target)
    }

    #[test]
    fn revert_abort_and_skip_restore_clean_single_operation_without_new_commit() {
        for no_commit in [false, true] {
            for action in [RevertControl::Abort, RevertControl::Skip] {
                let (repo, head, _) = conflicted_revert(no_commit, false);
                let path = repo.path();
                let state = feather_git::git::revert_state(path).unwrap().unwrap();
                control_revert(path, &state, action, no_commit).unwrap();
                assert!(feather_git::git::revert_state(path).unwrap().is_none());
                assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
                assert_eq!(
                    std::fs::read_to_string(path.join("file.txt")).unwrap(),
                    "later"
                );
                assert!(git(path, &["status", "--porcelain"]).is_empty());
                assert!(control_revert(path, &state, action, no_commit).is_err());
            }
        }
    }

    #[test]
    fn revert_failed_commit_preserves_resolution_for_retry_and_rejects_stale_options() {
        let (repo, head, _) = conflicted_revert(false, false);
        let path = repo.path();
        let state = feather_git::git::revert_state(path).unwrap().unwrap();
        let mut stale = state.clone();
        stale.root = path.join("other");
        assert!(control_revert(path, &stale, RevertControl::Abort, false).is_err());
        stale = state.clone();
        stale.session_modified += std::time::Duration::from_secs(1);
        assert!(control_revert(path, &stale, RevertControl::Skip, false).is_err());
        stale = state.clone();
        stale.todo.push_str("\n# changed");
        assert!(control_revert(path, &stale, RevertControl::Abort, false).is_err());
        let opts = path.join(".git/sequencer/opts");
        git(
            path,
            &[
                "config",
                "--file",
                opts.to_str().unwrap(),
                "options.no-commit",
                "true",
            ],
        );
        assert!(control_revert(path, &state, RevertControl::Continue, false).is_err());
        std::fs::write(opts, &state.options).unwrap();
        resolve_rebase_file(path, "file.txt", "saved resolution");
        git(path, &["config", "user.name", ""]);
        assert!(control_revert(path, &state, RevertControl::Continue, false).is_err());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        let current = feather_git::git::revert_state(path).unwrap().unwrap();
        assert!(current.conflicts.is_empty());
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "saved resolution"
        );
        git(path, &["config", "user.name", "Test"]);
        control_revert(path, &current, RevertControl::Continue, false).unwrap();
        assert!(feather_git::git::revert_state(path).unwrap().is_none());
    }

    #[test]
    fn external_single_revert_requires_explicit_commit_mode_and_supports_no_commit() {
        for no_commit in [false, true] {
            let (repo, head, target) = conflicted_revert(false, false);
            let path = repo.path();
            let first = feather_git::git::revert_state(path).unwrap().unwrap();
            control_revert(path, &first, RevertControl::Abort, false).unwrap();
            let output = command(path)
                .args([
                    "revert",
                    if no_commit {
                        "--no-commit"
                    } else {
                        "--no-edit"
                    },
                    &target,
                ])
                .output()
                .unwrap();
            assert!(!output.status.success());
            let state = feather_git::git::revert_state(path).unwrap().unwrap();
            assert_eq!(state.no_commit, None);
            assert_eq!(state.pending_count, 1);
            resolve_rebase_file(path, "file.txt", "external resolution");
            control_revert(path, &state, RevertControl::Continue, no_commit).unwrap();
            assert!(feather_git::git::revert_state(path).unwrap().is_none());
            assert_eq!(git(path, &["rev-parse", "HEAD"]) == head, no_commit);
        }
    }

    #[test]
    fn revert_sequence_continues_to_next_conflict_without_finishing_early() {
        let (repo, head, _, _) = conflicted_revert_sequence(false);
        let path = repo.path();
        let first = feather_git::git::revert_state(path).unwrap().unwrap();
        assert_eq!(first.pending_count, 2);
        resolve_rebase_file(path, "file.txt", "resolved first");
        control_revert(path, &first, RevertControl::Continue, false).unwrap();
        let second = feather_git::git::revert_state(path).unwrap().unwrap();
        assert!(second.same_session(&first) && !second.same_operation(&first));
        assert_eq!(second.pending_count, 1);
        assert_eq!(second.conflicts, vec![b"second.txt".to_vec()]);
        assert!(control_revert(path, &first, RevertControl::Skip, false).is_err());
        resolve_rebase_file(path, "second.txt", "resolved second");
        control_revert(path, &second, RevertControl::Continue, false).unwrap();
        assert!(feather_git::git::revert_state(path).unwrap().is_none());
        assert_eq!(git(path, &["rev-parse", "HEAD~2"]), head);
        assert_eq!(git(path, &["show", "HEAD:file.txt"]), "resolved first");
        assert_eq!(git(path, &["show", "HEAD:second.txt"]), "resolved second");
    }

    fn conflicted_revert_sequence(no_commit: bool) -> (tempfile::TempDir, String, String, String) {
        let repo = repository();
        let path = repo.path();
        git(path, &["config", "core.autocrlf", "false"]);
        std::fs::write(path.join("second.txt"), "base").unwrap();
        git(path, &["add", "."]);
        git(path, &["commit", "-m", "base"]);
        std::fs::write(path.join("file.txt"), "target first").unwrap();
        git(path, &["commit", "-am", "target first"]);
        let first = git(path, &["rev-parse", "HEAD"]);
        std::fs::write(path.join("second.txt"), "target second").unwrap();
        git(path, &["commit", "-am", "target second"]);
        let second = git(path, &["rev-parse", "HEAD"]);
        std::fs::write(path.join("file.txt"), "later first").unwrap();
        std::fs::write(path.join("second.txt"), "later second").unwrap();
        git(path, &["commit", "-am", "later"]);
        let head = git(path, &["rev-parse", "HEAD"]);
        let output = command(path)
            .args([
                "revert",
                if no_commit {
                    "--no-commit"
                } else {
                    "--no-edit"
                },
                &first,
                &second,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        (repo, head, first, second)
    }

    #[test]
    fn multi_commit_no_commit_revert_refuses_automatic_continue_without_mutation() {
        let (repo, head, _, _) = conflicted_revert_sequence(true);
        let path = repo.path();
        let state = feather_git::git::revert_state(path).unwrap().unwrap();
        assert_eq!(state.no_commit, Some(true));
        resolve_rebase_file(path, "file.txt", "saved resolution");
        let before_index = std::fs::read(path.join(".git/index")).unwrap();
        let before_state = feather_git::git::revert_state(path).unwrap().unwrap();
        assert!(control_revert(path, &state, RevertControl::Continue, true).is_err());
        assert_eq!(
            feather_git::git::revert_state(path).unwrap(),
            Some(before_state)
        );
        assert_eq!(
            std::fs::read(path.join(".git/index")).unwrap(),
            before_index
        );
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "saved resolution"
        );
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
    }

    #[test]
    fn revert_in_linked_worktree_is_isolated_from_other_operations() {
        let (repo, head, target) = conflicted_revert(false, false);
        let worktree = tempfile::tempdir().unwrap();
        let path = worktree.path().join("linked");
        git(
            repo.path(),
            &["worktree", "add", "--detach", path.to_str().unwrap(), &head],
        );
        assert!(revert_commit(&path, &target, true).is_err());
        let state = feather_git::git::revert_state(&path).unwrap().unwrap();
        assert_eq!(state.current_branch, None);
        assert_eq!(state.no_commit, Some(true));
        assert!(
            feather_git::git::load_initial(&path, None)
                .unwrap()
                .revert
                .is_some()
        );
        control_revert(&path, &state, RevertControl::Abort, true).unwrap();
        assert!(feather_git::git::revert_state(&path).unwrap().is_none());
        assert!(
            feather_git::git::revert_state(repo.path())
                .unwrap()
                .is_some()
        );
        let rebase = conflicted_rebase("merge").0;
        assert!(
            feather_git::git::revert_state(rebase.path())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn revert_state_excludes_cherry_pick_and_merge_conflicts() {
        let (repo, _, target) = conflicted_revert(false, false);
        let path = repo.path();
        let state = feather_git::git::revert_state(path).unwrap().unwrap();
        control_revert(path, &state, RevertControl::Abort, false).unwrap();
        let output = command(path)
            .args(["cherry-pick", &target])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(path.join(".git/CHERRY_PICK_HEAD").exists());
        assert!(feather_git::git::revert_state(path).unwrap().is_none());
        let merge = conflicted_merge().0;
        assert!(
            feather_git::git::revert_state(merge.path())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn revert_commit_respects_no_commit_and_rejects_invalid_targets() {
        for no_commit in [false, true] {
            let repo = repository();
            let path = repo.path();
            std::fs::write(path.join("added.txt"), "added").unwrap();
            git(path, &["add", "."]);
            git(path, &["commit", "-m", "added"]);
            let target = git(path, &["rev-parse", "HEAD"]);
            assert!(revert_commit(path, "--hard", no_commit).is_err());
            revert_commit(path, &target, no_commit).unwrap();
            assert!(feather_git::git::revert_state(path).unwrap().is_none());
            assert!(!path.join("added.txt").exists());
            assert_eq!(git(path, &["rev-parse", "HEAD"]) == target, no_commit);
            assert_eq!(
                !git(path, &["diff", "--cached", "--name-only"]).is_empty(),
                no_commit
            );
        }
    }

    #[test]
    fn revert_conflict_keeps_worktree_and_prevents_repeated_operation() {
        let repo = repository();
        let path = repo.path();
        std::fs::write(path.join("file.txt"), "second").unwrap();
        git(path, &["commit", "-am", "second"]);
        let target = git(path, &["rev-parse", "HEAD"]);
        std::fs::write(path.join("file.txt"), "third").unwrap();
        git(path, &["commit", "-am", "third"]);
        let head = git(path, &["rev-parse", "HEAD"]);
        assert!(revert_commit(path, &target, false).is_err());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert!(!git(path, &["ls-files", "-u"]).is_empty());
        let before = std::fs::read(path.join("file.txt")).unwrap();
        assert!(
            revert_commit(path, &target, false)
                .unwrap_err()
                .to_string()
                .contains("진행 중")
        );
        assert_eq!(std::fs::read(path.join("file.txt")).unwrap(), before);
    }

    #[test]
    fn rebase_continues_through_two_conflicts_and_finishes_only_after_last_commit() {
        let (repo, original, onto) = conflicted_rebase("merge");
        let path = repo.path();
        let first = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert_eq!(first.original_head, original);
        assert_eq!((first.step, first.total), (Some(1), Some(2)));
        assert!(first.current_commit.is_some());
        assert_eq!(first.conflicts, vec![b"file.txt".to_vec()]);
        assert!(feather_git::git::merge_state(path).unwrap().is_none());
        assert_eq!(
            feather_git::git::load_initial(path, None).unwrap().rebase,
            Some(first.clone())
        );
        assert!(control_rebase(path, &first, RebaseControl::Continue).is_err());
        resolve_rebase_file(path, "file.txt", "resolved one");
        control_rebase(path, &first, RebaseControl::Continue).unwrap();
        let second = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert!(second.same_session(&first) && !second.same_operation(&first));
        assert_eq!((second.step, second.total), (Some(2), Some(2)));
        assert_ne!(second.current_commit, first.current_commit);
        assert_eq!(second.conflicts, vec![b"second.txt".to_vec()]);
        assert!(control_rebase(path, &first, RebaseControl::Abort).is_err());
        resolve_rebase_file(path, "second.txt", "resolved two");
        control_rebase(path, &second, RebaseControl::Continue).unwrap();
        assert!(feather_git::git::rebase_state(path).unwrap().is_none());
        assert_eq!(git(path, &["branch", "--show-current"]), "topic");
        assert_eq!(git(path, &["rev-parse", "HEAD~2"]), onto);
        assert_eq!(git(path, &["show", "HEAD:file.txt"]), "resolved one");
        assert_eq!(git(path, &["show", "HEAD:second.txt"]), "resolved two");
        assert!(control_rebase(path, &second, RebaseControl::Continue).is_err());
    }

    fn conflicted_rebase(backend: &str) -> (tempfile::TempDir, String, String) {
        let repo = repository();
        let path = repo.path();
        git(path, &["config", "core.autocrlf", "false"]);
        git(path, &["config", "rebase.backend", backend]);
        std::fs::write(path.join("second.txt"), "base\n").unwrap();
        git(path, &["add", "."]);
        git(path, &["commit", "-m", "base"]);
        git(path, &["switch", "-c", "topic"]);
        std::fs::write(path.join("file.txt"), "topic one\n").unwrap();
        git(path, &["commit", "-am", "topic one"]);
        std::fs::write(path.join("second.txt"), "topic two\n").unwrap();
        git(path, &["commit", "-am", "topic two"]);
        let original = git(path, &["rev-parse", "HEAD"]);
        git(path, &["switch", "main"]);
        std::fs::write(path.join("file.txt"), "main one\n").unwrap();
        std::fs::write(path.join("second.txt"), "main two\n").unwrap();
        git(path, &["commit", "-am", "main"]);
        let onto = git(path, &["rev-parse", "HEAD"]);
        git(path, &["switch", "topic"]);
        assert!(integrate_commit(path, &onto, IntegrateKind::Rebase).is_err());
        (repo, original, onto)
    }

    fn resolve_rebase_file(path: &Path, name: &str, text: &str) {
        std::fs::write(path.join(name), text).unwrap();
        crate::setup::toggle_files(
            path,
            &std::collections::BTreeMap::from([(name.as_bytes().to_vec(), true)]),
        )
        .unwrap();
    }

    #[test]
    fn rebase_abort_restores_original_branch_and_rejects_stale_session_or_branch_tip() {
        let (repo, original, onto) = conflicted_rebase("merge");
        let path = repo.path();
        let state = feather_git::git::rebase_state(path).unwrap().unwrap();
        let mut stale = state.clone();
        stale.step = Some(5);
        assert!(control_rebase(path, &stale, RebaseControl::Skip).is_err());
        stale = state.clone();
        stale.root = path.join("other");
        assert!(control_rebase(path, &stale, RebaseControl::Abort).is_err());
        stale = state.clone();
        stale.marker_modified += std::time::Duration::from_secs(1);
        assert!(control_rebase(path, &stale, RebaseControl::Abort).is_err());
        stale = state.clone();
        stale.onto = original.clone();
        assert!(control_rebase(path, &stale, RebaseControl::Abort).is_err());
        git(path, &["update-ref", "refs/heads/topic", &onto]);
        assert!(control_rebase(path, &state, RebaseControl::Abort).is_err());
        git(path, &["update-ref", "refs/heads/topic", &original]);
        control_rebase(path, &state, RebaseControl::Abort).unwrap();
        assert!(feather_git::git::rebase_state(path).unwrap().is_none());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), original);
        assert_eq!(git(path, &["branch", "--show-current"]), "topic");
        assert_eq!(git(path, &["show", "HEAD:file.txt"]), "topic one");
        assert!(git(path, &["status", "--porcelain"]).is_empty());
        assert!(control_rebase(path, &state, RebaseControl::Abort).is_err());
    }

    #[test]
    fn rebase_skip_omits_current_commit_and_can_pause_on_next_conflict() {
        let (repo, _, onto) = conflicted_rebase("merge");
        let path = repo.path();
        let first = feather_git::git::rebase_state(path).unwrap().unwrap();
        control_rebase(path, &first, RebaseControl::Skip).unwrap();
        let second = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert_eq!(second.step, Some(2));
        assert_eq!(second.conflicts, vec![b"second.txt".to_vec()]);
        resolve_rebase_file(path, "second.txt", "kept second");
        control_rebase(path, &second, RebaseControl::Continue).unwrap();
        assert!(feather_git::git::rebase_state(path).unwrap().is_none());
        assert_eq!(git(path, &["rev-parse", "HEAD^"]), onto);
        assert_eq!(git(path, &["show", "HEAD:file.txt"]), "main one");
        assert_eq!(git(path, &["show", "HEAD:second.txt"]), "kept second");
    }

    #[test]
    fn rebase_apply_backend_supports_continue_and_abort() {
        let (repo, original, _) = conflicted_rebase("apply");
        let path = repo.path();
        let first = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert_eq!(first.backend, "rebase-apply");
        assert_eq!((first.step, first.total), (Some(1), Some(2)));
        resolve_rebase_file(path, "file.txt", "apply resolution");
        control_rebase(path, &first, RebaseControl::Continue).unwrap();
        let second = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert_eq!(second.step, Some(2));
        assert!(!second.conflicts.is_empty());
        control_rebase(path, &second, RebaseControl::Abort).unwrap();
        assert_eq!(git(path, &["rev-parse", "HEAD"]), original);
        assert!(feather_git::git::rebase_state(path).unwrap().is_none());
    }

    #[test]
    fn rebase_continue_failure_keeps_resolution_for_retry() {
        let (repo, _, _) = conflicted_rebase("merge");
        let path = repo.path();
        let state = feather_git::git::rebase_state(path).unwrap().unwrap();
        resolve_rebase_file(path, "file.txt", "saved resolution");
        git(path, &["config", "user.name", ""]);
        assert!(control_rebase(path, &state, RebaseControl::Continue).is_err());
        let current = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert!(current.same_operation(&state) && current.conflicts.is_empty());
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "saved resolution"
        );
        git(path, &["config", "user.name", "Test"]);
        control_rebase(path, &current, RebaseControl::Continue).unwrap();
        let next = feather_git::git::rebase_state(path).unwrap().unwrap();
        assert_eq!(next.step, Some(2));
        control_rebase(path, &next, RebaseControl::Abort).unwrap();
    }

    #[test]
    fn git_am_conflict_is_not_exposed_as_rebase() {
        let repo = repository();
        let path = repo.path();
        git(path, &["switch", "-c", "patch"]);
        std::fs::write(path.join("file.txt"), "patch change").unwrap();
        git(path, &["commit", "-am", "patch"]);
        let patch = git(path, &["format-patch", "-1", "--stdout"]);
        git(path, &["switch", "main"]);
        std::fs::write(path.join("file.txt"), "main change").unwrap();
        git(path, &["commit", "-am", "main"]);
        let patch_path = repo.path().join("incoming.patch");
        std::fs::write(&patch_path, patch).unwrap();
        let output = command(path)
            .args(["am", patch_path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(path.join(".git/rebase-apply/applying").exists());
        assert!(feather_git::git::rebase_state(path).unwrap().is_none());
        assert!(
            feather_git::git::load_initial(path, None)
                .unwrap()
                .rebase
                .is_none()
        );
        git(path, &["am", "--abort"]);
    }

    #[test]
    fn rebase_in_linked_worktree_is_isolated_from_original_worktree() {
        let (repo, original, onto) = conflicted_rebase("merge");
        let worktree = tempfile::tempdir().unwrap();
        let path = worktree.path().join("linked");
        git(
            repo.path(),
            &[
                "worktree",
                "add",
                "--detach",
                path.to_str().unwrap(),
                &original,
            ],
        );
        assert!(integrate_commit(&path, &onto, IntegrateKind::Rebase).is_err());
        let state = feather_git::git::rebase_state(&path).unwrap().unwrap();
        assert!(
            feather_git::git::load_initial(&path, None)
                .unwrap()
                .rebase
                .is_some()
        );
        control_rebase(&path, &state, RebaseControl::Abort).unwrap();
        assert!(feather_git::git::rebase_state(&path).unwrap().is_none());
        assert!(
            feather_git::git::rebase_state(repo.path())
                .unwrap()
                .is_some()
        );
        assert_eq!(git(&path, &["rev-parse", "HEAD"]), original);
    }

    #[test]
    fn merge_conflicts_can_be_staged_and_completed_with_both_parents() {
        let (repo, head, target) = conflicted_merge();
        let path = repo.path();
        let state = feather_git::git::merge_state(path).unwrap().unwrap();
        assert_eq!(state.conflicts, vec![b"file.txt".to_vec()]);
        let snapshot = feather_git::git::load_initial(path, None).unwrap();
        assert_eq!(snapshot.merge, Some(state.clone()));
        let conflicts: Vec<_> = snapshot
            .stage
            .iter()
            .filter(|file| file.status == "U")
            .collect();
        assert_eq!(conflicts.len(), 1);
        assert!(!conflicts[0].staged);
        let diff = feather_git::git::diff(path, conflicts[0]).unwrap();
        assert!(diff.lines.iter().any(|line| line.text.contains("<<<<<<<")));
        let before = std::fs::read(path.join("file.txt")).unwrap();
        assert!(control_merge(path, &state, MergeControl::Continue).is_err());
        assert_eq!(std::fs::read(path.join("file.txt")).unwrap(), before);
        std::fs::write(path.join("file.txt"), "resolved\n").unwrap();
        crate::setup::toggle_files(
            path,
            &std::collections::BTreeMap::from([(b"file.txt".to_vec(), true)]),
        )
        .unwrap();
        assert!(
            feather_git::git::merge_state(path)
                .unwrap()
                .unwrap()
                .conflicts
                .is_empty()
        );
        control_merge(path, &state, MergeControl::Continue).unwrap();
        assert!(feather_git::git::merge_state(path).unwrap().is_none());
        assert_eq!(git(path, &["rev-parse", "HEAD^1"]), head);
        assert_eq!(git(path, &["rev-parse", "HEAD^2"]), target);
        assert_eq!(git(path, &["show", "HEAD:file.txt"]), "resolved");
        assert!(control_merge(path, &state, MergeControl::Continue).is_err());
    }

    fn conflicted_merge() -> (tempfile::TempDir, String, String) {
        let repo = repository();
        let path = repo.path();
        git(path, &["switch", "-c", "topic"]);
        std::fs::write(path.join("file.txt"), "topic\n").unwrap();
        git(path, &["commit", "-am", "topic"]);
        let target = git(path, &["rev-parse", "HEAD"]);
        git(path, &["switch", "main"]);
        std::fs::write(path.join("file.txt"), "main\n").unwrap();
        git(path, &["commit", "-am", "main"]);
        let head = git(path, &["rev-parse", "HEAD"]);
        assert!(integrate_commit(path, &target, IntegrateKind::Merge).is_err());
        (repo, head, target)
    }

    #[test]
    fn merge_abort_restores_head_and_worktree_and_rejects_stale_state() {
        let (repo, head, _) = conflicted_merge();
        let path = repo.path();
        let state = feather_git::git::merge_state(path).unwrap().unwrap();
        let mut stale = state.clone();
        stale.root = path.join("other");
        assert!(control_merge(path, &stale, MergeControl::Abort).is_err());
        stale = state.clone();
        stale.marker_modified += std::time::Duration::from_secs(1);
        assert!(control_merge(path, &stale, MergeControl::Abort).is_err());
        stale = state.clone();
        stale.current_branch = Some("refs/heads/elsewhere".into());
        assert!(control_merge(path, &stale, MergeControl::Abort).is_err());
        stale = state.clone();
        stale.head = state.merge_heads[0].clone();
        assert!(control_merge(path, &stale, MergeControl::Abort).is_err());
        stale = state.clone();
        stale.merge_heads = vec![head.clone()];
        assert!(control_merge(path, &stale, MergeControl::Abort).is_err());
        control_merge(path, &state, MergeControl::Abort).unwrap();
        assert!(feather_git::git::merge_state(path).unwrap().is_none());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "main\n"
        );
        assert!(git(path, &["status", "--porcelain"]).is_empty());
        assert!(control_merge(path, &state, MergeControl::Abort).is_err());
    }

    #[test]
    fn merge_delete_resolution_can_be_staged_and_completed() {
        let repo = repository();
        let path = repo.path();
        git(path, &["switch", "-c", "topic"]);
        std::fs::write(path.join("file.txt"), "topic").unwrap();
        git(path, &["commit", "-am", "topic"]);
        let target = git(path, &["rev-parse", "HEAD"]);
        git(path, &["switch", "main"]);
        git(path, &["rm", "file.txt"]);
        git(path, &["commit", "-m", "delete"]);
        assert!(integrate_commit(path, &target, IntegrateKind::Merge).is_err());
        let state = feather_git::git::merge_state(path).unwrap().unwrap();
        let files = feather_git::git::load_stage(path).unwrap();
        assert_eq!(files.iter().filter(|file| file.status == "U").count(), 1);
        std::fs::remove_file(path.join("file.txt")).unwrap();
        crate::setup::toggle_files(
            path,
            &std::collections::BTreeMap::from([(b"file.txt".to_vec(), true)]),
        )
        .unwrap();
        assert!(
            !feather_git::git::load_stage(path)
                .unwrap()
                .iter()
                .any(|file| file.status == "U")
        );
        control_merge(path, &state, MergeControl::Continue).unwrap();
        assert!(feather_git::git::merge_state(path).unwrap().is_none());
        assert_eq!(git(path, &["rev-parse", "HEAD^2"]), target);
        assert!(!path.join("file.txt").exists());
    }

    #[test]
    fn merge_control_works_in_linked_worktree_without_affecting_other_worktree() {
        let (repo, head, target) = conflicted_merge();
        let worktree = tempfile::tempdir().unwrap();
        let path = worktree.path().join("linked");
        git(
            repo.path(),
            &["worktree", "add", "--detach", path.to_str().unwrap(), &head],
        );
        assert!(path.join(".git").is_file());
        assert!(integrate_commit(&path, &target, IntegrateKind::Merge).is_err());
        let state = feather_git::git::merge_state(&path).unwrap().unwrap();
        assert_eq!(state.current_branch, None);
        assert_eq!(state.conflicts.len(), 1);
        assert!(
            feather_git::git::load_initial(&path, None)
                .unwrap()
                .merge
                .is_some()
        );
        control_merge(&path, &state, MergeControl::Abort).unwrap();
        assert!(feather_git::git::merge_state(&path).unwrap().is_none());
        assert!(
            feather_git::git::merge_state(repo.path())
                .unwrap()
                .is_some()
        );
        assert_eq!(git(&path, &["rev-parse", "HEAD"]), head);
    }

    #[test]
    fn merge_continue_failure_preserves_resolved_state_for_retry() {
        let (repo, head, _) = conflicted_merge();
        let path = repo.path();
        let state = feather_git::git::merge_state(path).unwrap().unwrap();
        std::fs::write(path.join("file.txt"), "resolved").unwrap();
        crate::setup::toggle_files(
            path,
            &std::collections::BTreeMap::from([(b"file.txt".to_vec(), true)]),
        )
        .unwrap();
        git(path, &["config", "user.name", ""]);
        assert!(control_merge(path, &state, MergeControl::Continue).is_err());
        let current = feather_git::git::merge_state(path).unwrap().unwrap();
        assert!(current.same_operation(&state) && current.conflicts.is_empty());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "resolved"
        );
        git(path, &["config", "user.name", "Test"]);
        control_merge(path, &state, MergeControl::Continue).unwrap();
        assert!(feather_git::git::merge_state(path).unwrap().is_none());
    }

    #[test]
    fn merge_and_rebase_use_selected_base_and_preserve_conflicts() {
        for kind in [IntegrateKind::Merge, IntegrateKind::Rebase] {
            let repo = repository();
            let path = repo.path();
            git(path, &["switch", "-c", "topic"]);
            std::fs::write(path.join("topic.txt"), "topic").unwrap();
            git(path, &["add", "."]);
            git(path, &["commit", "-m", "topic"]);
            let target = git(path, &["rev-parse", "HEAD"]);
            git(path, &["switch", "main"]);
            std::fs::write(path.join("main.txt"), "main").unwrap();
            git(path, &["add", "."]);
            git(path, &["commit", "-m", "main"]);
            assert!(integrate_commit(path, "--abort", kind).is_err());
            integrate_commit(path, &target, kind).unwrap();
            assert_eq!(git(path, &["branch", "--show-current"]), "main");
            assert!(path.join("topic.txt").exists() && path.join("main.txt").exists());
            if kind == IntegrateKind::Rebase {
                assert_eq!(git(path, &["rev-parse", "HEAD^"]), target);
            } else {
                assert_eq!(git(path, &["rev-parse", "HEAD^2"]), target);
            }
        }
        for kind in [IntegrateKind::Merge, IntegrateKind::Rebase] {
            let repo = repository();
            let path = repo.path();
            git(path, &["switch", "-c", "topic"]);
            std::fs::write(path.join("file.txt"), "topic").unwrap();
            git(path, &["commit", "-am", "topic"]);
            let target = git(path, &["rev-parse", "HEAD"]);
            git(path, &["switch", "main"]);
            std::fs::write(path.join("file.txt"), "main").unwrap();
            git(path, &["commit", "-am", "main"]);
            assert!(integrate_commit(path, &target, kind).is_err());
            assert!(!git(path, &["ls-files", "-u"]).is_empty());
            let before = std::fs::read(path.join("file.txt")).unwrap();
            assert!(
                integrate_commit(path, &target, kind)
                    .unwrap_err()
                    .to_string()
                    .contains("진행 중")
            );
            assert_eq!(std::fs::read(path.join("file.txt")).unwrap(), before);
        }
    }

    #[test]
    fn reset_modes_change_only_their_documented_state() {
        for mode in ResetMode::ALL {
            let repo = repository();
            let path = repo.path();
            git(path, &["config", "core.autocrlf", "false"]);
            let first = git(path, &["rev-parse", "HEAD"]);
            std::fs::write(path.join("file.txt"), "second").unwrap();
            git(path, &["commit", "-am", "second"]);
            let second = git(path, &["rev-parse", "HEAD"]);
            std::fs::write(path.join("file.txt"), "staged").unwrap();
            git(path, &["add", "file.txt"]);
            std::fs::write(path.join("file.txt"), "unstaged").unwrap();
            std::fs::write(path.join("new.txt"), "untracked").unwrap();
            assert!(reset_commit(path, "--hard", mode).is_err());
            assert_eq!(git(path, &["rev-parse", "HEAD"]), second);
            reset_commit(path, &first, mode).unwrap();
            assert_eq!(git(path, &["rev-parse", "HEAD"]), first);
            assert_eq!(git(path, &["symbolic-ref", "HEAD"]), "refs/heads/main");
            assert_eq!(
                git(path, &["show", ":file.txt"]),
                if mode == ResetMode::Soft {
                    "staged"
                } else {
                    "first"
                }
            );
            assert_eq!(
                std::fs::read_to_string(path.join("file.txt")).unwrap(),
                if mode == ResetMode::Hard {
                    "first"
                } else {
                    "unstaged"
                }
            );
            assert_eq!(
                std::fs::read_to_string(path.join("new.txt")).unwrap(),
                "untracked"
            );
        }
    }

    #[test]
    fn branch_creation_option_publishes_only_when_requested() {
        let repo = repository();
        let path = repo.path();
        let commit = git(path, &["rev-parse", "HEAD"]);
        assert!(create_with_remote(path, "missing", &commit, true).is_err());
        assert!(!local_exists(path, "missing").unwrap());
        create_with_remote(path, "local", &commit, false).unwrap();
        create_options(path, "switch-new", &commit, false, true).unwrap();
        assert_eq!(
            git(path, &["symbolic-ref", "HEAD"]),
            "refs/heads/switch-new"
        );
        let bare = tempfile::tempdir().unwrap();
        git(bare.path(), &["init", "--bare"]);
        git(
            path,
            &["remote", "add", "origin", bare.path().to_str().unwrap()],
        );
        create_with_remote(path, "published", &commit, true).unwrap();
        assert_eq!(
            git(bare.path(), &["rev-parse", "refs/heads/published"]),
            commit
        );
        assert_eq!(git(path, &["config", "branch.published.remote"]), "origin");
        assert!(!reference_exists(bare.path(), "refs/heads/local").unwrap());
        git(path, &["push", "origin", "HEAD:refs/heads/existing"]);
        git(path, &["commit", "--allow-empty", "-m", "next"]);
        let next = git(path, &["rev-parse", "HEAD"]);
        assert!(create_with_remote(path, "existing", &next, true).is_err());
        assert!(local_exists(path, "existing").unwrap());
        assert_eq!(
            git(bare.path(), &["rev-parse", "refs/heads/existing"]),
            commit
        );
    }

    #[test]
    fn commit_branch_candidates_and_selection_use_exact_tip_and_ignore_symbolic() {
        let repo = repository();
        let path = repo.path();
        let first = git(path, &["rev-parse", "HEAD"]);
        let only = commit_checkout_candidates(path, &first).unwrap();
        assert_eq!(only.len(), 1);
        assert_eq!(only[0].reference, "refs/heads/main");
        git(path, &["update-ref", "refs/remotes/origin/main", &first]);
        assert_eq!(commit_checkout_candidates(path, &first).unwrap().len(), 1);
        git(
            path,
            &[
                "remote",
                "add",
                "team/origin",
                "https://example.invalid/team",
            ],
        );
        create(path, "feature/nested", &first).unwrap();
        git(
            path,
            &[
                "update-ref",
                "refs/remotes/team/origin/feature/nested",
                &first,
            ],
        );
        let filtered = commit_checkout_candidates(path, &first).unwrap();
        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .all(|candidate| candidate.reference.starts_with("refs/heads/"))
        );
        git(path, &["branch", "-d", "feature/nested"]);
        git(
            path,
            &[
                "update-ref",
                "-d",
                "refs/remotes/team/origin/feature/nested",
            ],
        );
        create(path, "other", &first).unwrap();
        git(
            path,
            &["remote", "add", "origin", "https://example.invalid/repo"],
        );
        git(path, &["update-ref", "refs/remotes/origin/topic", &first]);
        git(
            path,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/topic",
            ],
        );
        let candidates = commit_checkout_candidates(path, &first).unwrap();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].label, "main");
        assert_eq!(candidates[2].label, "origin/topic");
        checkout_commit_branch(path, "refs/heads/other", &first, false).unwrap();
        assert_eq!(git(path, &["symbolic-ref", "HEAD"]), "refs/heads/other");
        git(path, &["commit", "--allow-empty", "-m", "next"]);
        let second = git(path, &["rev-parse", "HEAD"]);
        assert!(checkout_commit_branch(path, "refs/heads/other", &first, false).is_err());
        git(path, &["branch", "topic", &second]);
        assert!(
            commit_checkout_candidates(path, &first)
                .unwrap()
                .iter()
                .all(|candidate| candidate.reference != "refs/remotes/origin/topic")
        );
        assert!(prepare_commit_branch(path, "refs/remotes/origin/topic", &first).is_err());
        git(path, &["branch", "-d", "topic"]);
        assert!(
            prepare_commit_branch(path, "refs/remotes/origin/topic", &first)
                .unwrap()
                .needs_creation
        );
        checkout_commit_branch(path, "refs/remotes/origin/topic", &first, true).unwrap();
        assert_eq!(git(path, &["symbolic-ref", "HEAD"]), "refs/heads/topic");
        assert_eq!(git(path, &["rev-parse", "HEAD"]), first);
        git(path, &["commit", "--allow-empty", "-m", "unreferenced"]);
        let orphan = git(path, &["rev-parse", "HEAD"]);
        git(path, &["reset", "--hard", &first]);
        assert!(
            commit_checkout_candidates(path, &orphan)
                .unwrap()
                .is_empty()
        );
        assert!(commit_checkout_candidates(path, "--all").is_err());
    }

    #[test]
    fn publish_selected_local_sets_upstream_without_overwriting_server() {
        let repo = repository();
        let path = repo.path();
        let bare = tempfile::tempdir().unwrap();
        git(bare.path(), &["init", "--bare"]);
        git(
            path,
            &["remote", "add", "backup", bare.path().to_str().unwrap()],
        );
        let commit = git(path, &["rev-parse", "HEAD"]);
        create(path, "feature/nested", &commit).unwrap();
        publish_branch(path, "refs/heads/feature/nested", &commit, "backup").unwrap();
        assert_eq!(
            git(bare.path(), &["rev-parse", "refs/heads/feature/nested"]),
            commit
        );
        assert_eq!(
            git(path, &["config", "branch.feature/nested.remote"]),
            "backup"
        );
        assert_eq!(
            git(path, &["config", "branch.feature/nested.merge"]),
            "refs/heads/feature/nested"
        );
        git(path, &["commit", "--allow-empty", "-m", "next"]);
        let head = git(path, &["rev-parse", "HEAD"]);
        git(path, &["branch", "-f", "feature/nested", &head]);
        assert!(publish_branch(path, "refs/heads/feature/nested", &head, "backup").is_err());
        assert!(publish_branch(path, "refs/heads/feature/nested", &commit, "backup").is_err());
        assert!(publish_branch(path, "refs/tags/example", &head, "backup").is_err());
        assert_eq!(
            git(bare.path(), &["rev-parse", "refs/heads/feature/nested"]),
            commit
        );
        assert_eq!(git(path, &["symbolic-ref", "HEAD"]), "refs/heads/main");
    }

    #[test]
    fn commit_checkout_detaches_and_protects_worktree_conflicts() {
        let repo = repository();
        let path = repo.path();
        let first = git(path, &["rev-parse", "HEAD"]);
        std::fs::write(path.join("file.txt"), "second").unwrap();
        git(path, &["commit", "-am", "second"]);
        let second = git(path, &["rev-parse", "HEAD"]);
        std::fs::write(path.join("file.txt"), "local change").unwrap();
        assert!(checkout_commit(path, &first).is_err());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), second);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "local change"
        );
        git(path, &["restore", "file.txt"]);
        assert!(is_current_branch(path, "refs/heads/main").unwrap());
        assert!(!is_current_branch(path, "refs/heads/other").unwrap());
        checkout_commit(path, &first).unwrap();
        assert!(!is_current_branch(path, "refs/heads/main").unwrap());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), first);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "first"
        );
        assert!(
            !command(path)
                .args(["symbolic-ref", "--quiet", "HEAD"])
                .output()
                .unwrap()
                .status
                .success()
        );
        assert_eq!(git(path, &["rev-parse", "refs/heads/main"]), second);
        assert!(checkout_commit(path, "--force").is_err());
    }

    #[test]
    fn tags_and_reference_deletion_preserve_head_and_files() {
        let repo = repository();
        let path = repo.path();
        let commit = git(path, &["rev-parse", "HEAD"]);
        git(path, &["commit", "--allow-empty", "-m", "next"]);
        let head = git(path, &["rev-parse", "HEAD"]);
        create_tag(path, "태그/선택", &commit).unwrap();
        assert_eq!(git(path, &["rev-parse", "refs/tags/태그/선택"]), commit);
        assert!(create_tag(path, "태그/선택", &head).is_err());
        for name in ["", "-bad", "a b", "a..b"] {
            assert!(create_tag(path, name, &commit).is_err());
        }
        create(path, "remove", &commit).unwrap();
        std::fs::write(path.join("file.txt"), "work in progress").unwrap();
        assert!(delete_reference(path, "refs/heads/main", &head, false).is_err());
        assert!(delete_reference(path, "refs/tags/태그/선택", &head, false).is_err());
        delete_reference(path, "refs/heads/remove", &commit, false).unwrap();
        delete_reference(path, "refs/tags/태그/선택", &commit, false).unwrap();
        assert!(!reference_exists(path, "refs/heads/remove").unwrap());
        assert!(!reference_exists(path, "refs/tags/태그/선택").unwrap());
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "work in progress"
        );
    }

    #[test]
    fn remote_deletion_distinguishes_tracking_and_server_and_protects_new_tip() {
        let source = repository();
        let bare = tempfile::tempdir().unwrap();
        git(bare.path(), &["init", "--bare"]);
        let path = source.path();
        git(
            path,
            &["remote", "add", "origin", bare.path().to_str().unwrap()],
        );
        git(path, &["push", "origin", "HEAD:refs/heads/topic"]);
        let old = git(path, &["rev-parse", "HEAD"]);
        delete_reference(path, "refs/remotes/origin/topic", &old, false).unwrap();
        assert_eq!(git(bare.path(), &["rev-parse", "refs/heads/topic"]), old);
        git(path, &["fetch", "origin"]);
        git(path, &["commit", "--allow-empty", "-m", "new"]);
        let new = git(path, &["rev-parse", "HEAD"]);
        git(
            bare.path(),
            &["fetch", path.to_str().unwrap(), "HEAD:refs/heads/topic"],
        );
        assert!(delete_reference(path, "refs/remotes/origin/topic", &old, true).is_err());
        assert_eq!(git(bare.path(), &["rev-parse", "refs/heads/topic"]), new);
        git(path, &["fetch", "origin"]);
        git(
            path,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/topic",
            ],
        );
        assert!(delete_reference(path, "refs/remotes/origin/HEAD", &new, false).is_err());
        delete_reference(path, "refs/remotes/origin/topic", &new, true).unwrap();
        assert!(!reference_exists(bare.path(), "refs/heads/topic").unwrap());
        assert!(!reference_exists(path, "refs/remotes/origin/topic").unwrap());
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
