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

fn load_snapshot(path: &Path, selected_commit: Option<&str>, paged: bool) -> Result<Snapshot> {
    let repo = open(path).context("Git 저장소를 열 수 없습니다")?;
    let root = repo.workdir().unwrap_or(repo.path()).canonicalize()?;
    let mut references = Vec::new();
    for reference in repo.references()?.all()?.peeled()? {
        let mut reference = reference.map_err(anyhow::Error::from_boxed)?;
        let full = reference.name().as_bstr().to_str()?.to_owned();
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
            name: reference.name().shorten().to_string(),
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
        reference_rows[reference.kind.index()].push(index);
    }
    let stage = if repo.workdir().is_some() {
        status(&repo)?
    } else {
        Vec::new()
    };
    let history_id = selected_commit
        .filter(|id| commits.iter().any(|c| c.id == *id))
        .or_else(|| commits.first().map(|c| c.id.as_str()));
    let history = match history_id {
        Some(id) => history(&repo, id)?,
        None => Vec::new(),
    };
    let commit_detail = history_id.map(|id| detail(&repo, id)).transpose()?;
    Ok(Snapshot {
        root,
        head,
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
        let conflict = index.entries().iter().any(|e| {
            e.path(&index) == path.as_bstr() && e.stage() != gix::index::entry::Stage::Unconflicted
        });
        let state = if conflict {
            "U"
        } else if matches!(before, Content::Empty) {
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
    files.sort_by(|a, b| b.staged.cmp(&a.staged).then(a.path.cmp(&b.path)));
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

fn read_content(repo: &gix::Repository, file: &FileChange, content: &Content) -> Result<Vec<u8>> {
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
