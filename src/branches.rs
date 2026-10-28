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
    execute(
        path,
        &[
            "revert",
            if no_commit {
                "--no-commit"
            } else {
                "--no-edit"
            },
            commit,
        ],
    )?;
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
