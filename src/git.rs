use crate::model::*;
use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::Path,
    time::Duration,
};

pub const MAX_BYTES: u64 = 1024 * 1024;

pub(crate) fn open(path: &Path) -> Result<gix::Repository> {
    // 조회 중에도 사용자 정의 clean/process 필터가 실행되지 않도록 한다.
    Ok(gix::discover_opts(
        path,
        Default::default(),
        gix::open::Options::default().filter_config_section(|_| false),
    )?)
}

pub fn repository_root(path: &Path) -> Result<std::path::PathBuf> {
    let repo = open(path).context("Git 저장소를 열 수 없습니다")?;
    Ok(repo.workdir().unwrap_or(repo.path()).canonicalize()?)
}

pub fn load(
    path: &Path,
    _revision: Option<&str>,
    selected_commit: Option<&str>,
) -> Result<Snapshot> {
    load_snapshot(path, selected_commit, false)
}

pub fn load_initial(path: &Path, selected_commit: Option<&str>) -> Result<Snapshot> {
    load_snapshot(path, selected_commit, true)
}

pub fn load_for_refresh(path: &Path, selected_commit: Option<&str>) -> Result<Snapshot> {
    let mut snapshot = load_initial(path, selected_commit)?;
    while snapshot.has_more
        && selected_commit
            .is_some_and(|selected| !snapshot.commits.iter().any(|commit| commit.id == selected))
    {
        let Some(session) = &snapshot.session else {
            break;
        };
        let page = session.next_page()?;
        snapshot.commits.extend(page.commits);
        snapshot.has_more = page.has_more;
    }
    Ok(snapshot)
}

fn load_snapshot(path: &Path, selected_commit: Option<&str>, paged: bool) -> Result<Snapshot> {
    let repo = open(path).context("Git 저장소를 열 수 없습니다")?;
    let root = repo.workdir().unwrap_or(repo.path()).canonicalize()?;
    let mut references = Vec::new();
    let raw_references = repo
        .references()?
        .all()?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(anyhow::Error::from_boxed)?;
    for mut reference in raw_references {
        let full = reference.name().as_bstr().to_str()?.to_owned();
        let name = reference.name().shorten().to_string();
        let kind = if full.starts_with("refs/heads/") {
            RefTab::Local
        } else if full.starts_with("refs/remotes/") {
            RefTab::Remote
        } else if full.starts_with("refs/tags/") {
            RefTab::Tag
        } else {
            continue;
        };
        // 태그가 트리나 blob을 가리키는 경우 커밋 탐색 대상에서 제외한다.
        let target = match reference.peel_to_commit() {
            Ok(commit) => commit.id.to_string(),
            Err(_) => continue,
        };
        references.push(Reference {
            name,
            revision: full,
            kind,
            target,
        });
    }
    references.sort_by(|a, b| a.name.cmp(&b.name));
    if let Some(stash) = repo.try_find_reference("refs/stash")? {
        let mut platform = stash.log_iter();
        if let Some(log) = platform.all()? {
            let entries = log.collect::<std::result::Result<Vec<_>, _>>()?;
            for (i, entry) in entries.into_iter().rev().enumerate() {
                references.push(Reference {
                    name: format!("stash@{{{i}}}  {}", entry.message),
                    revision: entry.new_oid().to_string(),
                    kind: RefTab::Stash,
                    target: entry.new_oid().to_string(),
                });
            }
        }
    }
    let head = repo
        .head_name()?
        .map(|n| n.shorten().to_string())
        .unwrap_or_else(|| "Detached HEAD".into());
    let mut tips = HashSet::new();
    for reference in &references {
        tips.insert(gix::ObjectId::from_hex(reference.target.as_bytes())?);
    }
    if !repo.head()?.is_unborn() {
        tips.insert(repo.head_id()?.detach());
    }
    // 시작점 순서를 고정하여 같은 시각의 이력도 일관되게 조회한다.
    let mut tips: Vec<_> = tips.into_iter().collect();
    tips.sort();
    let session = crate::paging::HistorySession::start(path.to_path_buf(), tips, &references)?;
    let first = session.next_page()?;
    let mut commits = first.commits;
    let mut has_more = first.has_more;
    if !paged {
        while has_more {
            let page = session.next_page()?;
            commits.extend(page.commits);
            has_more = page.has_more;
        }
    }
    let current_branch = repo.head_name()?.map(|n| n.to_string());
    let mut reference_rows: [Vec<usize>; 4] = Default::default();
    for (index, reference) in references.iter().enumerate() {
        if reference.is_remote_head() {
            continue;
        }
        reference_rows[reference.kind.index()].push(index);
    }
    let stage = if repo.workdir().is_some() {
        status(&repo)?
    } else {
        Vec::new()
    };
    let history_id = selected_commit
        .filter(|id| {
            gix::ObjectId::from_hex(id.as_bytes())
                .ok()
                .is_some_and(|id| repo.find_commit(id).is_ok())
        })
        .or_else(|| commits.first().map(|c| c.id.as_str()));
    let history = match history_id {
        Some(id) => history(&repo, id)?,
        None => Vec::new(),
    };
    let commit_detail = history_id.map(|id| detail(&repo, id)).transpose()?;
    Ok(Snapshot {
        stash_restore: crate::stash_recovery::state(path)?,
        unmerged: stage
            .iter()
            .filter(|file| file.status == "U")
            .map(|file| file.path.clone())
            .collect(),
        revert: revert_state(path)?,
        rebase: rebase_state(path)?,
        merge: merge_state(path)?,
        root,
        head,
        head_commit: if repo.head()?.is_unborn() {
            None
        } else {
            Some(repo.head_id()?.to_string())
        },
        references,
        commits,
        stage,
        history,
        commit_detail,
        current_branch,
        reference_rows,
        session: paged.then_some(session),
        has_more,
    })
}

/// 전체 이력과 작업 상태를 다시 읽지 않고 선택한 커밋만 조회한다.
pub fn load_commit(path: &Path, id: &str) -> Result<CommitSelection> {
    let repo = open(path)?;
    Ok(CommitSelection {
        history: history(&repo, id)?,
        detail: detail(&repo, id)?,
    })
}

pub fn load_stage(path: &Path) -> Result<Vec<FileChange>> {
    let repo = open(path)?;
    if repo.workdir().is_none() {
        bail!("작업 디렉터리가 없는 저장소입니다.");
    }
    status(&repo)
}

pub fn revert_state(path: &Path) -> Result<Option<RevertState>> {
    let repo = open(path)?;
    let dir = repo.path();
    if [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-merge",
        "rebase-apply",
    ]
    .iter()
    .any(|name| dir.join(name).exists())
    {
        return Ok(None);
    }
    let read = |name: &str| -> Result<Option<String>> {
        match std::fs::read_to_string(dir.join(name)) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    };
    let marker = read("REVERT_HEAD")?;
    let todo = read("sequencer/todo")?.unwrap_or_default();
    let mut pending = Vec::new();
    for line in todo
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("revert") {
            return Ok(None);
        }
        pending.push(
            fields
                .next()
                .context("Revert 대기 목록을 읽지 못했습니다.")?
                .to_owned(),
        );
    }
    let Some(target) = marker
        .as_deref()
        .map(str::trim)
        .or_else(|| pending.first().map(String::as_str))
    else {
        return Ok(None);
    };
    let target = repo
        .rev_parse_single(target.as_bytes().as_bstr())?
        .to_string();
    repo.find_commit(gix::ObjectId::from_hex(target.as_bytes())?)?;
    let sequence_head = read("sequencer/head")?.map(|value| value.trim().to_owned());
    let options = read("sequencer/opts")?.unwrap_or_default();
    let mut no_commit = sequence_head.as_ref().map(|_| false);
    for line in options
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#') && !line.starts_with(';'))
    {
        let (key, value) = line.split_once('=').unwrap_or((line, "true"));
        if key.trim().eq_ignore_ascii_case("no-commit") {
            no_commit = match value.trim().trim_matches('"').to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => Some(true),
                "false" | "no" | "off" | "0" => Some(false),
                _ => None,
            };
        }
    }
    let target_modified = if marker.is_some() {
        Some(std::fs::metadata(dir.join("REVERT_HEAD"))?.modified()?)
    } else {
        None
    };
    let session_modified = if sequence_head.is_some() {
        std::fs::metadata(dir.join("sequencer/head"))?.modified()?
    } else {
        target_modified.context("Revert 세션을 읽지 못했습니다.")?
    };
    let index = repo.index_or_empty()?;
    let conflicts = index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
        .map(|entry| entry.path(&index).to_vec())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(Some(RevertState {
        root: repo
            .workdir()
            .context("작업 디렉터리가 없습니다")?
            .canonicalize()?,
        head: repo.head_id()?.to_string(),
        current_branch: repo.head_name()?.map(|name| name.to_string()),
        sequence_head,
        session_modified,
        target,
        target_modified,
        pending_count: pending.len().max(usize::from(marker.is_some())),
        todo,
        options,
        no_commit,
        conflicts,
    }))
}

pub fn rebase_state(path: &Path) -> Result<Option<RebaseState>> {
    let repo = open(path)?;
    let backend = if repo.path().join("rebase-merge").is_dir() {
        "rebase-merge"
    } else if repo.path().join("rebase-apply").is_dir()
        && repo.path().join("rebase-apply/rebasing").exists()
    {
        "rebase-apply"
    } else {
        return Ok(None);
    };
    let dir = repo.path().join(backend);
    let read = |name: &str| -> Result<Option<String>> {
        match std::fs::read_to_string(dir.join(name)) {
            Ok(value) => Ok(Some(value.trim().to_owned())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    };
    let original_head = read("orig-head")?.context("Rebase 시작 커밋을 읽지 못했습니다.")?;
    let onto = read("onto")?.context("Rebase 기반 커밋을 읽지 못했습니다.")?;
    let branch = read("head-name")?.context("Rebase 대상 브랜치를 읽지 못했습니다.")?;
    for id in [&original_head, &onto] {
        repo.find_commit(gix::ObjectId::from_hex(id.as_bytes())?)?;
    }
    let branch_tip = if branch.starts_with("refs/heads/") {
        repo.try_find_reference(branch.as_str())?
            .map(|mut reference| -> Result<String> {
                Ok(reference.peel_to_commit()?.id.to_string())
            })
            .transpose()?
    } else {
        None
    };
    let current_commit = read(if backend == "rebase-merge" {
        "stopped-sha"
    } else {
        "original-commit"
    })?
    .filter(|value| !value.is_empty());
    if let Some(id) = &current_commit {
        repo.find_commit(gix::ObjectId::from_hex(id.as_bytes())?)?;
    }
    let number = |name: &str| -> Result<Option<usize>> {
        read(name)?
            .map(|value| value.parse().map_err(anyhow::Error::from))
            .transpose()
    };
    let index = repo.index_or_empty()?;
    let conflicts = index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
        .map(|entry| entry.path(&index).to_vec())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(Some(RebaseState {
        root: repo
            .workdir()
            .context("작업 디렉터리가 없습니다")?
            .canonicalize()?,
        backend: backend.into(),
        marker_modified: std::fs::metadata(dir.join("orig-head"))?.modified()?,
        original_head,
        branch,
        branch_tip,
        onto,
        head: repo.head_id()?.to_string(),
        current_branch: repo.head_name()?.map(|name| name.to_string()),
        current_commit,
        step: number(if backend == "rebase-merge" {
            "msgnum"
        } else {
            "next"
        })?,
        total: number(if backend == "rebase-merge" {
            "end"
        } else {
            "last"
        })?,
        conflicts,
    }))
}

pub fn merge_state(path: &Path) -> Result<Option<MergeState>> {
    let repo = open(path)?;
    let git_dir = repo.path();
    if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        return Ok(None);
    }
    let mut marker = match std::fs::File::open(git_dir.join("MERGE_HEAD")) {
        Ok(marker) => marker,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let marker_modified = marker.metadata()?.modified()?;
    let mut contents = String::new();
    marker.read_to_string(&mut contents)?;
    let merge_heads = contents
        .lines()
        .map(|line| {
            let id = gix::ObjectId::from_hex(line.trim().as_bytes())?;
            repo.find_commit(id)?;
            Ok(id.to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    if merge_heads.is_empty() {
        bail!("병합 대상 정보를 읽지 못했습니다.");
    }
    let index = repo.index_or_empty()?;
    let conflicts = index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
        .map(|entry| entry.path(&index).to_vec())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(Some(MergeState {
        root: repo
            .workdir()
            .context("작업 디렉터리가 없습니다")?
            .canonicalize()?,
        marker_modified,
        head: repo.head_id()?.to_string(),
        current_branch: repo.head_name()?.map(|name| name.to_string()),
        merge_heads,
        conflicts,
    }))
}

fn detail(repo: &gix::Repository, id: &str) -> Result<CommitDetail> {
    let commit = repo.find_commit(gix::ObjectId::from_hex(id.as_bytes())?)?;
    let decoded = commit.decode()?;
    let author = decoded.author()?;
    let committer = decoded.committer()?;
    let author_time = author.time()?;
    let commit_time = committer.time()?;
    Ok(CommitDetail {
        id: id.to_owned(),
        parent_ids: commit.parent_ids().map(|id| id.to_string()).collect(),
        author: author.name.to_str_lossy().into_owned(),
        author_email: author.email.to_str_lossy().into_owned(),
        author_time: author_time.seconds,
        author_offset: author_time.offset,
        committer: committer.name.to_str_lossy().into_owned(),
        committer_email: committer.email.to_str_lossy().into_owned(),
        commit_time: commit_time.seconds,
        commit_offset: commit_time.offset,
        message: decoded.message.to_str_lossy().into_owned(),
    })
}

fn tree_content(tree: Option<&gix::Tree<'_>>, path: &[u8]) -> Result<Content> {
    let Some(tree) = tree else {
        return Ok(Content::Empty);
    };
    Ok(match tree.lookup_entry(path.split(|b| *b == b'/'))? {
        Some(entry) => Content::Object(entry.object_id(), entry.mode().value()),
        None => Content::Empty,
    })
}

fn status(repo: &gix::Repository) -> Result<Vec<FileChange>> {
    let index = repo.index_or_empty()?;
    let mut conflicts: BTreeMap<Vec<u8>, Content> = BTreeMap::new();
    for entry in index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        let base = conflicts
            .entry(entry.path(&index).to_vec())
            .or_insert(Content::Empty);
        if entry.stage() as u8 == 1 {
            *base = Content::Object(entry.id, entry.mode.bits() as u16);
        }
    }
    let head = if repo.head()?.is_unborn() {
        None
    } else {
        Some(repo.head_commit()?.tree()?)
    };
    let mut files = Vec::new();
    let iter = repo
        .status(gix::progress::Discard)?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_rewrites(None)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .into_iter(Vec::<gix::bstr::BString>::new())?;
    for item in iter {
        let item = item?;
        let path = item.location().to_vec();
        if conflicts.contains_key(&path) {
            continue;
        }
        let staged = matches!(item, gix::status::Item::TreeIndex(_));
        let entry = index.entry_by_path(path.as_bstr());
        let indexed = entry
            .map(|e| Content::Object(e.id, e.mode.bits() as u16))
            .unwrap_or(Content::Empty);
        let (before, after) = if staged {
            (tree_content(head.as_ref(), &path)?, indexed)
        } else {
            let work_path = repo
                .workdir()
                .expect("worktree checked")
                .join(gix::path::from_bstr(path.as_bstr()));
            let after = match std::fs::symlink_metadata(work_path) {
                Ok(_) => Content::Worktree,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Content::Empty,
                Err(err) => return Err(err.into()),
            };
            (indexed, after)
        };
        let state = if matches!(before, Content::Empty) {
            "A"
        } else if matches!(after, Content::Empty) {
            "D"
        } else {
            "M"
        };
        files.push(FileChange {
            label: path.to_str_lossy().into_owned(),
            path,
            status: state.into(),
            staged,
            before,
            after,
        });
    }
    for (path, base) in conflicts {
        let work_path = repo
            .workdir()
            .context("작업 디렉터리가 없습니다")?
            .join(gix::path::from_bstr(path.as_bstr()));
        let after = match std::fs::symlink_metadata(work_path) {
            Ok(_) => Content::Worktree,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Content::Empty,
            Err(error) => return Err(error.into()),
        };
        files.push(FileChange {
            label: path.to_str_lossy().into_owned(),
            path,
            status: "U".into(),
            staged: false,
            before: base,
            after,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path).then(b.staged.cmp(&a.staged)));
    Ok(files)
}

// 객체 ID와 파일 모드를 비교하므로 외부 diff 도구나 필터를 실행하지 않는다.
fn flatten(
    tree: &gix::Tree<'_>,
    prefix: &[u8],
    out: &mut BTreeMap<Vec<u8>, Content>,
) -> Result<()> {
    for entry in tree.iter() {
        let entry = entry?;
        let mut path = prefix.to_vec();
        path.extend_from_slice(entry.filename());
        if entry.mode().is_tree() {
            path.push(b'/');
            flatten(&entry.object()?.try_into_tree()?, &path, out)?;
        } else {
            out.insert(
                path,
                Content::Object(entry.object_id(), entry.mode().value()),
            );
        }
    }
    Ok(())
}

fn history(repo: &gix::Repository, id: &str) -> Result<Vec<FileChange>> {
    let commit = repo.find_commit(gix::ObjectId::from_hex(id.as_bytes())?)?;
    let mut old = BTreeMap::new();
    if let Some(parent) = commit.parent_ids().next() {
        flatten(&parent.object()?.try_into_commit()?.tree()?, b"", &mut old)?;
    }
    let mut new = BTreeMap::new();
    flatten(&commit.tree()?, b"", &mut new)?;
    let mut paths: Vec<_> = old.keys().chain(new.keys()).cloned().collect();
    paths.sort();
    paths.dedup();
    let mut files = Vec::new();
    for path in paths {
        let before = old.remove(&path).unwrap_or(Content::Empty);
        let after = new.remove(&path).unwrap_or(Content::Empty);
        if let (Content::Object(a, am), Content::Object(b, bm)) = (&before, &after)
            && a == b
            && am == bm
        {
            continue;
        }
        let status = if matches!(before, Content::Empty) {
            "A"
        } else if matches!(after, Content::Empty) {
            "D"
        } else {
            "M"
        };
        files.push(FileChange {
            label: path.to_str_lossy().into_owned(),
            path,
            status: status.into(),
            staged: false,
            before,
            after,
        });
    }
    Ok(files)
}

pub(crate) fn read_content(
    repo: &gix::Repository,
    file: &FileChange,
    content: &Content,
) -> Result<Vec<u8>> {
    match content {
        Content::Empty => Ok(Vec::new()),
        Content::Object(_, mode) if mode & 0o170000 != 0o100000 => {
            bail!("심볼릭 링크 또는 서브모듈입니다. 텍스트 미리보기를 제공하지 않습니다.")
        }
        Content::Object(id, _) => {
            let header = repo.find_header(*id)?;
            if header.size() > MAX_BYTES {
                bail!("1 MiB를 초과한 파일입니다.")
            }
            Ok(repo.find_blob(*id)?.data.clone())
        }
        Content::Worktree => {
            let root = repo.workdir().context("작업 디렉터리가 없습니다")?;
            let relative = gix::path::from_bstr(file.path.as_bstr());
            let mut path = root.to_path_buf();
            for component in relative.components() {
                if !matches!(component, std::path::Component::Normal(_)) {
                    bail!("잘못된 파일 경로입니다")
                }
                path.push(component);
                let meta = std::fs::symlink_metadata(&path)?;
                if meta.file_type().is_symlink() {
                    bail!("심볼릭 링크입니다. 텍스트 미리보기를 제공하지 않습니다.")
                }
            }
            let meta = std::fs::symlink_metadata(&path)?;
            if !meta.is_file() {
                bail!("서브모듈 또는 특수 파일입니다.")
            }
            if meta.len() > MAX_BYTES {
                bail!("1 MiB를 초과한 파일입니다.")
            }
            let mut data = Vec::new();
            std::fs::File::open(path)?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut data)?;
            if data.len() as u64 > MAX_BYTES {
                bail!("1 MiB를 초과한 파일입니다.")
            }
            Ok(data)
        }
    }
}

pub fn diff(path: &Path, file: &FileChange) -> Result<Diff> {
    let repo = open(path)?;
    let mut result = Diff {
        title: file.label.clone(),
        ..Diff::default()
    };
    let before = match read_content(&repo, file, &file.before) {
        Ok(data) => data,
        Err(err) => {
            result.note = err.to_string();
            return Ok(result);
        }
    };
    let after = match read_content(&repo, file, &file.after) {
        Ok(data) => data,
        Err(err) => {
            result.note = err.to_string();
            return Ok(result);
        }
    };
    if before.contains(&0) || after.contains(&0) {
        result.note = "바이너리 파일입니다. 텍스트 차이를 표시하지 않습니다.".into();
        return Ok(result);
    }
    let (Ok(before), Ok(after)) = (std::str::from_utf8(&before), std::str::from_utf8(&after))
    else {
        result.note = "UTF-8 텍스트가 아닙니다. 텍스트 차이를 표시하지 않습니다.".into();
        return Ok(result);
    };
    let diff = similar::TextDiff::configure()
        .timeout(Duration::from_millis(500))
        .diff_lines(before, after);
    if crate::partial_stage::eligible(file) {
        result.partial = Some(crate::partial_stage::Preview {
            before: before.to_owned(),
            after: after.to_owned(),
        });
    }
    if file.status == "U" && !file.staged && matches!(file.after, Content::Worktree) {
        match crate::conflict::parse(after) {
            Ok(preview) if !preview.regions.is_empty() => {
                result.note = "충돌 구간에 커서를 놓고 상단에서 선택하세요. 해결 후 파일 목록에서 Space로 Stage하세요. 현재/들어오는 변경은 마커의 위/아래 내용입니다.".into();
                result.conflict = Some(preview);
            }
            Ok(_) => result.note =
                "선택할 충돌 마커가 없습니다. 내용을 확인한 뒤 파일 목록에서 Space로 Stage하세요."
                    .into(),
            Err(error) => result.note = error.to_string(),
        }
    }
    for group in diff.grouped_ops(3) {
        if result.lines.len() >= 5000 {
            result.note = "차이를 5,000줄까지 표시합니다.".into();
            return Ok(result);
        }
        let first = &group[0];
        result.lines.push(DiffLine {
            old: None,
            new: None,
            kind: '@',
            text: format!(
                "@@ -{} +{} @@",
                first.old_range().start + 1,
                first.new_range().start + 1
            ),
        });
        for op in group {
            for change in diff.iter_changes(&op) {
                if result.lines.len() >= 5000 {
                    result.note = "차이를 5,000줄까지 표시합니다.".into();
                    return Ok(result);
                }
                let kind = match change.tag() {
                    similar::ChangeTag::Delete => '-',
                    similar::ChangeTag::Insert => '+',
                    similar::ChangeTag::Equal => ' ',
                };
                result.lines.push(DiffLine {
                    old: change.old_index().map(|v| v + 1),
                    new: change.new_index().map(|v| v + 1),
                    kind,
                    text: change
                        .value()
                        .trim_end_matches(['\r', '\n'])
                        .replace('\t', "    "),
                });
            }
        }
    }
    if result.lines.is_empty() {
        result.note = "텍스트 내용은 동일합니다. 파일 모드나 줄바꿈 변경일 수 있습니다.".into();
    }
    Ok(result)
}
