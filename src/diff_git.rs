use crate::diff_model::*;
use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
pub const MAX_BYTES: u64 = 1024 * 1024;

pub(crate) fn open(path: &Path) -> Result<gix::Repository> {
    let mut options = gix::open::Options::default();
    options.permissions.config.git_binary = false;
    options.permissions.config.system = false;
    options.permissions.attributes.git_binary = false;
    options.permissions.attributes.system = false;
    let mut repo = gix::open_opts(path, options.filter_config_section(|_| false))?;
    inherit_line_endings(&mut repo, &git2::Config::open_default()?)?;
    Ok(repo)
}

fn inherit_line_endings(repo: &mut gix::Repository, defaults: &git2::Config) -> Result<()> {
    use gix::config::tree::{Core, Key};
    let keys: [(&str, &dyn Key); 3] = [
        ("core.autocrlf", &Core::AUTO_CRLF),
        ("core.eol", &Core::EOL),
        ("core.safecrlf", &Core::SAFE_CRLF),
    ];
    let mut inherited = Vec::new();
    for (name, key) in keys {
        if repo.config_snapshot().string(name).is_some() { continue; }
        match defaults.get_string(name) {
            Ok(value) => inherited.push((key, value)),
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if !inherited.is_empty() {
        let mut config = repo.config_snapshot_mut();
        for (key, value) in inherited { config.set_value(key, value.as_str())?; }
        config.commit()?;
    }
    Ok(())
}

pub fn load_stage(path: &Path) -> Result<Vec<FileChange>> {
    let repo = open(path)?;
    if repo.workdir().is_none() {
        return Ok(Vec::new());
    }
    status(&repo)
}
pub fn load_history(path: &Path, id: &str) -> Result<Vec<FileChange>> {
    history(&open(path)?, id)
}
pub fn load_detail(path: &Path, id: &str) -> Result<CommitDetail> {
    detail(&open(path)?, id)
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

fn normalization_only(repo: &gix::Repository, index: &gix::index::State, item: &gix::status::Item) -> Result<bool> {
    use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
    let gix::status::Item::IndexWorktree(gix::status::index_worktree::Item::Modification {
        entry, rela_path,
        status: EntryStatus::Change(Change::Modification { executable_bit_changed: false, content_change: Some(_), .. }), ..
    }) = item else { return Ok(false); };
    let config = repo.config_snapshot();
    let autocrlf = config.boolean("core.autocrlf") == Some(true)
        || config.string("core.autocrlf").is_some_and(|value| value.eq_ignore_ascii_case(b"input"));
    if !autocrlf || entry.mode.bits() & 0o170000 != 0o100000 || entry.stat.size == 0 { return Ok(false); }
    let relative = gix::path::from_bstr(rela_path.as_bstr())?;
    let path = repo.workdir().context("작업 디렉터리가 없습니다.")?.join(relative.as_ref());
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || u64::from(entry.stat.size) == metadata.len() { return Ok(false); }
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    let mut cleaned = pipeline.convert_to_git(fs::File::open(path)?, relative.as_ref(), index)?;
    let original = repo.find_blob(entry.id)?;
    let mut offset = 0;
    let mut buffer = [0; 8192];
    loop {
        let read = cleaned.read(&mut buffer)?;
        if read == 0 { return Ok(offset == original.data.len()); }
        if original.data.get(offset..offset + read) != Some(&buffer[..read]) { return Ok(false); }
        offset += read;
    }
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
        .index_worktree_options_mut(|options| options.thread_limit = Some(1))
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_rewrites(None)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .into_iter(Vec::<gix::bstr::BString>::new())?;
    for item in iter {
        let item = item?;
        if normalization_only(repo, &index, &item)? { continue; }
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
                .join(gix::path::from_bstr(path.as_bstr())?);
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
            .join(gix::path::from_bstr(path.as_bstr())?);
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
    files.sort_by(worktree_file_order);
    Ok(files)
}

fn worktree_file_order(a: &FileChange, b: &FileChange) -> std::cmp::Ordering {
    let status_order = |status: &str| match status {
        "M" => 1,
        "D" => 2,
        "A" => 3,
        "U" => 0,
        _ => 4,
    };
    b.staged.cmp(&a.staged)
        .then_with(|| status_order(&a.status).cmp(&status_order(&b.status)))
        .then_with(|| flat_tree_path_order(&a.path, &b.path))
}

fn flat_tree_path_order(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let mut a = a.split(|byte| *byte == b'/').peekable();
    let mut b = b.split(|byte| *byte == b'/').peekable();
    loop {
        match (a.next(), b.next()) {
            (Some(left), Some(right)) => {
                let order = a.peek().is_some().cmp(&b.peek().is_some())
                    .then_with(|| left.cmp(right));
                if order != std::cmp::Ordering::Equal { return order; }
            }
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
        }
    }
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
            let relative = gix::path::from_bstr(file.path.as_bstr())?;
            let mut path = root.to_path_buf();
            for component in relative.components() {
                if !matches!(component, std::path::Component::Normal(_)) {
                    bail!("잘못된 파일 경로입니다")
                }
                path.push(component);
                let meta = std::fs::symlink_metadata(&path)?;
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        bail!("링크 경로는 미리볼 수 없습니다.");
                    }
                }
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
    diff_with_encoding(path, file, crate::text_encoding::TextEncoding::Utf8)
}

pub fn diff_with_encoding(
    path: &Path,
    file: &FileChange,
    encoding: crate::text_encoding::TextEncoding,
) -> Result<Diff> {
    let repo = open(path)?;
    let mut result = Diff::default();
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
    if matches!(file.after, Content::Worktree) {
        result.worktree = Some(after.clone());
        let index = repo.index_or_empty()?;
        result.index_state = file_index_state(&index, &file.path);
    }
    if !encoding.is_utf16() && (before.contains(&0) || after.contains(&0)) {
        result.note = "바이너리 파일입니다. 텍스트 차이를 표시하지 않습니다.".into();
        return Ok(result);
    }
    let (Some(before), Some(after)) = (encoding.decode(&before), encoding.decode(&after)) else {
        result.note = format!(
            "{} 텍스트가 아닙니다. 텍스트 차이를 표시하지 않습니다.",
            encoding.name()
        );
        return Ok(result);
    };
    let (before, after) = (before.as_ref(), after.as_ref());
    if before.contains('\0') || after.contains('\0') {
        result.note = "바이너리 파일입니다. 텍스트 차이를 표시하지 않습니다.".into();
        return Ok(result);
    }
    let writable = encoding == crate::text_encoding::TextEncoding::Utf8;
    if !writable {
        result.note = "보기 인코딩을 적용했습니다. 줄 단위 작업과 충돌 편집은 UTF-8 보기에서만 지원합니다. 파일 전체 작업을 사용하세요.".into();
    }
    let old_count = before.lines().count();
    let new_count = after.lines().count();
    result.empty_eof = before.is_empty() && after.is_empty();
    let comparison_before = crate::line_endings::comparison_text(before);
    let comparison_after = crate::line_endings::comparison_text(after);
    let diff = similar::TextDiff::configure()
        .timeout(Duration::from_millis(500))
        .diff_lines(comparison_before.as_ref(), comparison_after.as_ref());
    if writable && crate::partial_stage::eligible(file) {
        result.partial = Some(crate::partial_stage::Preview {
            before: before.to_owned(),
            after: after.to_owned(),
        });
    }
    if writable && file.status == "U" && !file.staged && matches!(file.after, Content::Worktree) {
        match crate::conflict::parse(after) {
            Ok(preview) if !preview.regions.is_empty() => {
                result.note = "해결 후 파일 목록에서 Space로 Stage하세요.".into();
                result.conflict = Some(preview);
            }
            Ok(_) => {
                result.conflict_resolved = true;
                result.note =
                    "선택할 충돌 마커가 없습니다. 내용을 확인한 뒤 파일 전체 Stage를 실행하세요."
                        .into();
            }
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
                if change
                    .old_index()
                    .is_some_and(|index| index + 1 == old_count)
                    || change
                        .new_index()
                        .is_some_and(|index| index + 1 == new_count)
                {
                    result.eof_rows.insert(result.lines.len());
                }
                result.lines.push(DiffLine {
                    old: change.old_index().map(|v| v + 1),
                    new: change.new_index().map(|v| v + 1),
                    kind,
                    text: change.value().trim_end_matches(['\r', '\n']).to_owned(),
                });
            }
        }
    }
    if result.lines.is_empty() {
        result.note = "텍스트 내용은 동일합니다. 파일 모드나 줄바꿈 변경일 수 있습니다.".into();
    }
    Ok(result)
}

struct IndexLock {
    target: PathBuf,
    path: PathBuf,
    file: Option<fs::File>,
}
impl IndexLock {
    fn acquire(target: PathBuf) -> Result<Self> {
        let mut name = target.as_os_str().to_owned();
        name.push(".lock");
        let path = PathBuf::from(name);
        let file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .context("인덱스가 잠겨 있습니다.")?;
        Ok(Self {
            target,
            path,
            file: Some(file),
        })
    }
    fn write(mut self, index: &mut gix::index::File) -> Result<()> {
        index.remove_tree();
        let file = self.file.as_mut().context("인덱스 잠금이 없습니다.")?;
        index.write_to(&mut *file, Default::default())?;
        file.flush()?;
        file.sync_all()?;
        drop(self.file.take());
        fs::rename(&self.path, &self.target)?;
        Ok(())
    }
}
impl Drop for IndexLock {
    fn drop(&mut self) {
        drop(self.file.take());
        let _ = fs::remove_file(&self.path);
    }
}

pub fn set_index_content(
    path: &Path,
    raw: &[u8],
    content: Option<&[u8]>,
    mode: u32,
    expected: Option<(gix::ObjectId, u32)>,
) -> Result<()> {
    let repo = open(path)?;
    let lock = IndexLock::acquire(repo.index_path())?;
    let mut index = gix::index::File::at_or_default(
        repo.index_path(),
        repo.object_hash(),
        false,
        Default::default(),
    )?;
    let actual = index
        .entry_by_path(raw.as_bstr())
        .map(|e| (e.id, e.mode.bits()));
    if actual != expected
        || index.entries().iter().any(|e| {
            e.path(&index).as_bytes() == raw && e.stage() != gix::index::entry::Stage::Unconflicted
        })
    {
        bail!("인덱스가 변경되었습니다. 다시 조회하세요.");
    }
    index.remove_entries(|_, p, _| p.as_bytes() == raw);
    if let Some(content) = content {
        let id = repo.write_blob(content)?.detach();
        index.dangerously_push_entry(
            Default::default(),
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode),
            raw.as_bstr(),
        );
    }
    index.sort_entries();
    lock.write(&mut index)?;
    Ok(())
}

/// 작업 파일은 보존하고 선택 파일의 인덱스 상태만 토글합니다.
pub fn toggle_stage(root: &Path, file: &FileChange) -> Result<()> {
    if file.status == "U" {
        return stage_resolved(root, file, &diff(root, file)?);
    }
    let repo = open(root)?;
    let worktree = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    let relative = gix::path::from_bstr(file.path.as_bstr())?;
    let mut path = worktree.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    if components.is_empty() { bail!("잘못된 파일 경로입니다."); }
    for (index, component) in components.iter().enumerate() {
        if !matches!(component, std::path::Component::Normal(_)) { bail!("잘못된 파일 경로입니다."); }
        path.push(component.as_os_str());
        if !file.staged && index + 1 < components.len() {
            let metadata = match fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && matches!(file.after, Content::Empty) => continue,
                result => result?,
            };
            if metadata.is_symlink() { bail!("링크 아래 파일은 Stage할 수 없습니다."); }
            #[cfg(windows)] {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 { bail!("링크 아래 파일은 Stage할 수 없습니다."); }
            }
        }
    }
    let lock = IndexLock::acquire(repo.index_path())?;
    let mut index = gix::index::File::at_or_default(repo.index_path(), repo.object_hash(), false, Default::default())?;
    let expected = if file.staged { &file.after } else { &file.before };
    let expected: Vec<_> = match expected {
        Content::Object(id, mode) => vec![(0, *id, u32::from(*mode))],
        Content::Empty => Vec::new(),
        Content::Worktree => bail!("잘못된 인덱스 상태입니다."),
    };
    if file_index_state(&index, &file.path) != expected { bail!("인덱스가 변경되었습니다. 다시 조회하세요."); }
    let replacement = if file.staged {
        let head = if repo.head()?.is_unborn() { None } else { Some(repo.head_commit()?.tree()?) };
        let original = tree_content(head.as_ref(), &file.path)?;
        if original != file.before { bail!("HEAD가 변경되었습니다. 다시 조회하세요."); }
        match original {
            Content::Object(id, mode) => Some((id, u32::from(mode))),
            _ => None,
        }
    } else if matches!(file.after, Content::Empty) {
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
            Ok(_) => bail!("작업 파일이 다시 생성되었습니다. 다시 조회하세요."),
        }
    } else {
        let (mut pipeline, _) = repo.filter_pipeline(None)?;
        let (id, kind, _) = pipeline.worktree_file_to_object(file.path.as_bstr(), &index)?
            .context("Stage할 파일이 사라졌습니다. 다시 조회하세요.")?;
        Some((id, kind as u32))
    };
    index.remove_entries(|_, path, _| path.as_bytes() == file.path);
    if let Some((id, mode)) = replacement {
        index.dangerously_push_entry(Default::default(), id, gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode), file.path.as_bstr());
    }
    index.sort_entries();
    lock.write(&mut index)
}

fn file_index_state(index: &gix::index::State, path: &[u8]) -> Vec<(u8, gix::ObjectId, u32)> {
    index
        .entries()
        .iter()
        .filter(|e| e.path(index).as_bytes() == path)
        .map(|e| (e.stage() as u8, e.id, e.mode.bits()))
        .collect()
}

/// 해결한 충돌 파일 전체를 Stage합니다. 미리보기와 잠금 아래 원본을 대조합니다.
pub fn stage_resolved(root: &Path, file: &FileChange, preview: &Diff) -> Result<()> {
    if file.staged || file.status != "U" || !matches!(file.after, Content::Worktree) {
        bail!("충돌을 해결한 작업 파일만 이 작업으로 Stage할 수 있습니다.");
    }
    let repo = open(root)?;
    let lock = IndexLock::acquire(repo.index_path())?;
    let mut index = gix::index::File::at_or_default(
        repo.index_path(),
        repo.object_hash(),
        false,
        Default::default(),
    )?;
    let actual = file_index_state(&index, &file.path);
    if !actual.iter().any(|(stage, _, _)| *stage != 0) {
        bail!("미병합 파일이 아닙니다. 다시 조회하세요.");
    }
    if actual != preview.index_state {
        bail!("인덱스가 변경되었습니다. 다시 조회하세요.");
    }
    let source = read_content(&repo, file, &Content::Worktree)?;
    if Some(&source) != preview.worktree.as_ref() {
        bail!("파일 내용이 변경되었습니다. 다시 조회하세요.");
    }
    let text = std::str::from_utf8(&source)?;
    if !crate::conflict::parse(text)?.regions.is_empty() {
        bail!("미해결 충돌 마커가 남아 있습니다.");
    }
    // 참조 프로젝트와 같은 gix clean 파이프라인으로 줄바꿈/속성을 반영합니다.
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    let (id, _, _) = pipeline
        .worktree_file_to_object(file.path.as_bstr(), &index)?
        .context("Stage할 파일이 사라졌습니다.")?;
    let mode = index
        .entries()
        .iter()
        .find(|e| e.path(&index).as_bytes() == file.path)
        .map_or(0o100644, |e| e.mode.bits());
    if mode & 0o170000 != 0o100000 {
        bail!("일반 파일만 Stage할 수 있습니다.");
    }
    // blob을 읽는 동안 파일이 바뀌었어도 오래된 결과를 쓰지 않습니다.
    if read_content(&repo, file, &Content::Worktree)? != source {
        bail!("파일 내용이 변경되었습니다.");
    }
    index.remove_entries(|_, p, _| p.as_bytes() == file.path);
    index.dangerously_push_entry(
        Default::default(),
        id,
        gix::index::entry::Flags::empty(),
        gix::index::entry::Mode::from_bits_truncate(mode),
        file.path.as_bstr(),
    );
    index.sort_entries();
    lock.write(&mut index)
}

pub struct DiscardPreview {
    pub labels: Vec<String>,
    head: Option<gix::ObjectId>,
    files: Vec<DiscardFile>,
}

struct DiscardFile {
    path: Vec<u8>,
    original: Content,
    index: Vec<(u8, gix::ObjectId, u32)>,
    worktree: Option<Vec<u8>>,
    permissions: Option<fs::Permissions>,
}

pub struct DiscardResult {
    pub succeeded: usize,
    pub errors: Vec<String>,
}

fn discard_head(repo: &gix::Repository) -> Result<Option<gix::ObjectId>> {
    Ok(if repo.head()?.is_unborn() { None } else { Some(repo.head_commit()?.id) })
}

fn discard_path(worktree: &Path, raw: &[u8]) -> Result<PathBuf> {
    let relative = gix::path::from_bstr(raw.as_bstr())?;
    let components: Vec<_> = relative.components().collect();
    if components.is_empty() { bail!("잘못된 파일 경로입니다."); }
    let mut path = worktree.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        if !matches!(component, std::path::Component::Normal(_))
            || component.as_os_str().to_string_lossy().eq_ignore_ascii_case(".git") {
            bail!("안전하지 않은 파일 경로입니다.");
        }
        path.push(component.as_os_str());
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if metadata.is_symlink() { bail!("링크 경로의 변경사항은 폐기할 수 없습니다."); }
        #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 { bail!("재분석 지점의 변경사항은 폐기할 수 없습니다."); }
        }
        if index + 1 == components.len() {
            if !metadata.is_file() { bail!("일반 파일만 폐기할 수 있습니다."); }
        } else if !metadata.is_dir() { bail!("파일 경로의 부모가 디렉터리가 아닙니다."); }
    }
    Ok(path)
}

fn discard_source(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// 확인 팝업에 사용할 원본을 읽기만 하며 인덱스와 파일을 변경하지 않습니다.
pub fn prepare_discard(root: &Path, files: &[FileChange]) -> Result<DiscardPreview> {
    let repo = open(root)?;
    let worktree = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    let actual = status(&repo)?;
    let head = discard_head(&repo)?;
    let tree = head.map(|id| repo.find_commit(id)?.tree()).transpose()?;
    let index = gix::index::File::at_or_default(repo.index_path(), repo.object_hash(), false, Default::default())?;
    let mut preview = DiscardPreview { labels: Vec::new(), head, files: Vec::new() };
    for file in files {
        if preview.files.iter().any(|item| item.path == file.path) { continue; }
        if !actual.contains(file) { bail!("{}: 파일 상태가 변경되었습니다. 다시 조회하세요.", file.label); }
        let path = discard_path(worktree, &file.path).with_context(|| file.label.clone())?;
        let original = tree_content(tree.as_ref(), &file.path)?;
        let entries = file_index_state(&index, &file.path);
        if matches!(original, Content::Object(_, mode) if mode & 0o170000 != 0o100000)
            || entries.iter().any(|(_, _, mode)| mode & 0o170000 != 0o100000) {
            bail!("{}: 일반 파일만 폐기할 수 있습니다.", file.label);
        }
        let source = discard_source(&path)?;
        let permissions = source.as_ref().map(|_| fs::metadata(&path).map(|metadata| metadata.permissions())).transpose()?;
        preview.labels.push(file.label.clone());
        preview.files.push(DiscardFile { path: file.path.clone(), original, index: entries, worktree: source, permissions });
    }
    if preview.files.is_empty() { bail!("선택한 파일이 없습니다."); }
    Ok(preview)
}

fn restore_discard_source(path: &Path, file: &DiscardFile) -> Result<()> {
    if let Some(source) = &file.worktree {
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        fs::write(path, source)?;
        if let Some(permissions) = &file.permissions { fs::set_permissions(path, permissions.clone())?; }
    } else {
        match fs::remove_file(path) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn discard_one(root: &Path, head: Option<gix::ObjectId>, file: &DiscardFile) -> Result<()> {
    let repo = open(root)?;
    let worktree = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    let path = discard_path(worktree, &file.path)?;
    let lock = IndexLock::acquire(repo.index_path())?;
    let mut index = gix::index::File::at_or_default(repo.index_path(), repo.object_hash(), false, Default::default())?;
    if discard_head(&repo)? != head { bail!("HEAD가 변경되었습니다. 다시 조회하세요."); }
    if file_index_state(&index, &file.path) != file.index { bail!("인덱스가 변경되었습니다. 다시 조회하세요."); }
    if discard_source(&path)? != file.worktree { bail!("확인 중 파일 내용이 변경되었습니다. 다시 조회하세요."); }
    if let Some(expected) = &file.permissions {
        let actual = fs::metadata(&path)?.permissions();
        if actual.readonly() != expected.readonly() { bail!("파일 권한이 변경되었습니다. 다시 조회하세요."); }
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            if actual.mode() != expected.mode() { bail!("파일 권한이 변경되었습니다. 다시 조회하세요."); }
        }
    }
    let result = (|| -> Result<()> {
        match file.original {
            Content::Object(_, _) => {
                let git = git2::Repository::open(root)?;
                let id = git2::Oid::from_bytes(head.context("HEAD가 없습니다.")?.as_bytes())?;
                let commit = git.find_commit(id)?;
                let tree = commit.tree()?;
                let relative = gix::path::from_bstr(file.path.as_bstr())?;
                let mut options = git2::build::CheckoutBuilder::new();
                options.force().update_index(false).disable_pathspec_match(true).path(&*relative);
                git.checkout_tree(tree.as_object(), Some(&mut options))?;
            }
            Content::Empty => {
                if file.worktree.is_some() { fs::remove_file(&path)?; }
            }
            Content::Worktree => bail!("잘못된 복구 원본입니다."),
        }
        index.remove_entries(|_, path, _| path.as_bytes() == file.path);
        if let Content::Object(id, mode) = file.original {
            index.dangerously_push_entry(Default::default(), id, gix::index::entry::Flags::empty(),
                gix::index::entry::Mode::from_bits_truncate(u32::from(mode)), file.path.as_bstr());
        }
        index.sort_entries();
        lock.write(&mut index)
    })();
    if let Err(error) = result {
        // Git 쓰기 실패 시 확인 전 작업 파일을 복구하고 복구 실패도 숨기지 않습니다.
        let rollback = discard_path(worktree, &file.path).and_then(|path| restore_discard_source(&path, file));
        return match rollback {
            Ok(()) => Err(error),
            Err(rollback) => Err(anyhow::anyhow!("{error:#}; 원본 복구 실패: {rollback:#}")),
        };
    }
    Ok(())
}

pub fn discard_selected(root: &Path, preview: DiscardPreview) -> DiscardResult {
    let mut result = DiscardResult { succeeded: 0, errors: Vec::new() };
    for (label, file) in preview.labels.iter().zip(&preview.files) {
        match discard_one(root, preview.head, file) {
            Ok(()) => result.succeeded += 1,
            Err(error) => result.errors.push(format!("{label}: {error:#}")),
        }
    }
    result
}

struct WorktreeTemporary(PathBuf);
impl Drop for WorktreeTemporary {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

/// 체크한 줄 폐기는 인덱스를 쓰지 않고 잠금 아래 작업 파일만 교체합니다.
pub fn set_worktree_content(root: &Path, file: &FileChange, source: &[u8], replacement: &[u8]) -> Result<()> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let repo = open(root)?;
    let root = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    let path = discard_path(root, &file.path)?;
    let _lock = IndexLock::acquire(repo.index_path())?;
    let index = gix::index::File::at_or_default(repo.index_path(), repo.object_hash(), false, Default::default())?;
    let expected = match file.before {
        Content::Object(id, mode) => vec![(0, id, u32::from(mode))],
        Content::Empty => Vec::new(),
        _ => bail!("잘못된 인덱스 원본입니다."),
    };
    if file_index_state(&index, &file.path) != expected { bail!("인덱스가 변경되었습니다. 다시 조회하세요."); }
    let expected_source = (!matches!(file.after, Content::Empty)).then_some(source);
    if discard_source(&path)?.as_deref() != expected_source { bail!("작업 파일이 변경되었습니다. 다시 조회하세요."); }
    let permissions = if expected_source.is_some() { Some(fs::metadata(&path)?.permissions()) } else { None };
    if permissions.as_ref().is_some_and(|permissions| permissions.readonly()) { bail!("읽기 전용 파일입니다."); }
    if matches!(file.before, Content::Empty) && replacement.is_empty() {
        if expected_source.is_some() { fs::remove_file(&path)?; }
        return Ok(());
    }
    let parent = path.parent().context("파일 폴더가 없습니다.")?;
    fs::create_dir_all(parent)?;
    let number = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary_path = parent.join(format!(".feather-discard-{}-{number}.tmp", std::process::id()));
    let mut output = fs::OpenOptions::new().write(true).create_new(true).open(&temporary_path)?;
    let temporary = WorktreeTemporary(temporary_path);
    output.write_all(replacement)?; output.flush()?; output.sync_all()?;
    if let Some(permissions) = permissions { fs::set_permissions(&temporary.0, permissions)?; }
    #[cfg(unix)] if expected_source.is_none() {
        use std::os::unix::fs::PermissionsExt;
        let mode = expected.first().map_or(0o644, |(_, _, mode)| mode & 0o777);
        fs::set_permissions(&temporary.0, fs::Permissions::from_mode(mode))?;
    }
    drop(output);
    discard_path(root, &file.path)?;
    if discard_source(&path)?.as_deref() != expected_source { bail!("작업 파일이 변경되었습니다. 다시 조회하세요."); }
    fs::rename(&temporary.0, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn git(root: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "core.autocrlf=false",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Diff Test",
                "-c",
                "user.email=diff@example.invalid",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().into()
    }
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-b", "main"]);
        git(dir.path(), &["config", "core.autocrlf", "false"]);
        dir
    }
    fn commit(root: &Path, message: &str) -> String {
        git(root, &["add", "."]);
        git(root, &["commit", "-m", message]);
        git(root, &["rev-parse", "HEAD"])
    }
    fn discard_fixture() -> tempfile::TempDir {
        let parent = std::env::current_dir().unwrap().join("target/discard-tests");
        fs::create_dir_all(&parent).unwrap();
        let dir = tempfile::tempdir_in(parent).unwrap();
        git(dir.path(), &["init", "-b", "main"]);
        git(dir.path(), &["config", "core.autocrlf", "false"]);
        dir
    }

    #[test]
    fn cross_line_endings_show_only_actual_text_changes_and_keep_eof_changes() {
        let dir = fixture(); let root = dir.path();
        fs::write(root.join("file.txt"), "one\ntwo\nthree\n").unwrap(); commit(root, "base");
        git(root, &["config", "core.autocrlf", "true"]);
        fs::write(root.join("file.txt"), "one\r\ntwo changed\r\nthree\r\n").unwrap();
        let file = load_stage(root).unwrap().into_iter().find(|file| !file.staged && file.path == b"file.txt").unwrap();
        let data = diff(root, &file).unwrap();
        assert_eq!(data.lines.iter().filter(|line| line.kind == '-').count(), 1);
        assert_eq!(data.lines.iter().filter(|line| line.kind == '+').count(), 1);
        assert!(data.lines.iter().any(|line| line.kind == '+' && line.new == Some(2) && line.text == "two changed"));
        fs::write(root.join("file.txt"), "one\r\ntwo\r\nthree\r\n").unwrap();
        assert!(diff(root, &file).unwrap().lines.is_empty());
        fs::write(root.join("file.txt"), "one\r\ntwo\r\nthree").unwrap();
        let data = diff(root, &file).unwrap();
        assert_eq!(data.lines.iter().filter(|line| line.kind == '-').count(), 1);
        assert!(!data.eof_rows.is_empty());
    }

    #[test]
    fn discard_preserves_crlf_and_resolves_only_selected_conflict() {
        let dir = discard_fixture(); let root = dir.path();
        fs::write(root.join(".gitattributes"), "*.txt text eol=crlf\n").unwrap();
        fs::write(root.join("file.txt"), "base\r\n").unwrap();
        commit(root, "base");
        fs::write(root.join("file.txt"), "changed\r\n").unwrap();
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        let result = discard_selected(root, preview);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(root.join("file.txt")).unwrap(), b"base\r\n");
        git(root, &["checkout", "-b", "side"]);
        fs::write(root.join("file.txt"), "side\r\n").unwrap(); commit(root, "side");
        git(root, &["checkout", "main"]);
        fs::write(root.join("file.txt"), "main\r\n").unwrap(); commit(root, "main");
        let merge = std::process::Command::new("git").arg("-C").arg(root).args(["merge", "side"]).output().unwrap();
        assert!(!merge.status.success());
        let files = load_stage(root).unwrap();
        assert!(files.iter().any(|file| file.status == "U"));
        let preview = prepare_discard(root, &files).unwrap();
        let result = discard_selected(root, preview);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(root.join("file.txt")).unwrap(), b"main\r\n");
        assert!(load_stage(root).unwrap().is_empty());
        assert!(root.join(".git/MERGE_HEAD").exists());
    }

    #[cfg(windows)]
    #[test]
    fn discard_rejects_junction_paths_without_touching_the_destination() {
        let dir = discard_fixture(); let root = dir.path();
        let outside = root.parent().unwrap().join(format!("{}-outside", root.file_name().unwrap().to_string_lossy()));
        fs::create_dir(&outside).unwrap(); fs::write(outside.join("file.txt"), "preserve").unwrap();
        let link = root.join("linked");
        let output = std::process::Command::new("cmd").args(["/c", "mklink", "/J"])
            .arg(link.to_string_lossy().replace('/', "\\")).arg(outside.to_string_lossy().replace('/', "\\")).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(discard_path(root, b"linked/file.txt").is_err());
        assert_eq!(fs::read_to_string(outside.join("file.txt")).unwrap(), "preserve");
        fs::remove_dir(&link).unwrap();
        fs::remove_file(outside.join("file.txt")).unwrap(); fs::remove_dir(outside).unwrap();
    }

    #[test]
    fn discard_restores_head_and_removes_new_files_without_touching_other_paths() {
        let dir = discard_fixture(); let root = dir.path();
        for name in ["mixed.txt", "deleted.txt", "other.txt", "한 글[1].bin", "한 글1.bin"] {
            fs::write(root.join(name), b"base\0\n").unwrap();
        }
        let head = commit(root, "base");
        fs::write(root.join("mixed.txt"), b"staged\n").unwrap();
        fs::write(root.join("other.txt"), b"other staged\n").unwrap();
        fs::write(root.join("new-staged.txt"), b"added\n").unwrap();
        git(root, &["add", "mixed.txt", "other.txt", "new-staged.txt"]);
        fs::write(root.join("mixed.txt"), b"unstaged\n").unwrap();
        fs::write(root.join("other.txt"), b"other unstaged\n").unwrap();
        fs::write(root.join("new.txt"), b"new\n").unwrap();
        fs::write(root.join("한 글[1].bin"), b"changed\0\xff").unwrap();
        fs::write(root.join("한 글1.bin"), b"preserve pattern neighbour").unwrap();
        fs::remove_file(root.join("deleted.txt")).unwrap();
        let files = load_stage(root).unwrap().into_iter().filter(|file| file.label != "other.txt" && file.label != "한 글1.bin").collect::<Vec<_>>();
        let index_before = fs::read(root.join(".git/index")).unwrap();
        let preview = prepare_discard(root, &files).unwrap();
        assert_eq!(preview.labels.len(), 5);
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index_before);
        assert_eq!(fs::read(root.join("mixed.txt")).unwrap(), b"unstaged\n");
        let result = discard_selected(root, preview);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.succeeded, 5);
        for name in ["mixed.txt", "deleted.txt", "한 글[1].bin"] { assert_eq!(fs::read(root.join(name)).unwrap(), b"base\0\n"); }
        assert!(!root.join("new.txt").exists()); assert!(!root.join("new-staged.txt").exists());
        assert_eq!(fs::read(root.join("other.txt")).unwrap(), b"other unstaged\n");
        assert_eq!(git(root, &["show", ":other.txt"]), "other staged");
        assert_eq!(fs::read(root.join("한 글1.bin")).unwrap(), b"preserve pattern neighbour");
        assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
        assert!(load_stage(root).unwrap().iter().all(|file| ["other.txt", "한 글1.bin"].contains(&file.label.as_str())));
    }

    #[test]
    fn discard_handles_unborn_staged_deletion_and_rejects_stale_or_locked_sources() {
        let dir = discard_fixture(); let root = dir.path();
        fs::write(root.join("new.txt"), "new").unwrap(); git(root, &["add", "new.txt"]);
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        assert!(discard_selected(root, preview).errors.is_empty());
        assert!(!root.join("new.txt").exists()); assert!(load_stage(root).unwrap().is_empty());
        fs::write(root.join("a.txt"), "base").unwrap(); fs::write(root.join("b.txt"), "base").unwrap(); commit(root, "base");
        fs::remove_file(root.join("a.txt")).unwrap(); git(root, &["add", "a.txt"]);
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        assert!(discard_selected(root, preview).errors.is_empty()); assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "base");
        fs::write(root.join("a.txt"), "changed").unwrap(); fs::write(root.join("b.txt"), "changed").unwrap();
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        fs::write(root.join("a.txt"), "external").unwrap();
        let result = discard_selected(root, preview);
        assert_eq!(result.succeeded, 1); assert_eq!(result.errors.len(), 1);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "external");
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        git(root, &["add", "a.txt"]);
        assert_eq!(discard_selected(root, preview).succeeded, 0);
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        fs::write(root.join(".git/index.lock"), "owner").unwrap();
        assert_eq!(discard_selected(root, preview).succeeded, 0);
        assert_eq!(fs::read_to_string(root.join(".git/index.lock")).unwrap(), "owner");
        fs::remove_file(root.join(".git/index.lock")).unwrap();
        let preview = prepare_discard(root, &load_stage(root).unwrap()).unwrap();
        git(root, &["commit", "--allow-empty", "-m", "head changed"]);
        assert_eq!(discard_selected(root, preview).succeeded, 0);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "external");
        assert!(discard_path(root, b"../outside").is_err());
        assert!(discard_path(root, b".git/config").is_err());
        fs::create_dir(root.join("directory")).unwrap();
        assert!(discard_path(root, b"directory").is_err());
    }

    #[test]
    fn inherited_autocrlf_treats_crlf_as_clean_and_preserves_local_override() {
        let dir = fixture(); let root = dir.path();
        fs::write(root.join("file.txt"), "one\ntwo\n").unwrap(); commit(root, "base");
        fs::write(root.join("file.txt"), "one\r\ntwo\r\n").unwrap();
        let default_dir = tempfile::tempdir().unwrap();
        let mut defaults = git2::Config::open(&default_dir.path().join("gitconfig")).unwrap();
        defaults.set_bool("core.autocrlf", true).unwrap();
        let isolated = || {
            let mut options = gix::open::Options::default();
            options.permissions.config = gix::open::permissions::Config::isolated();
            gix::open_opts(root, options.filter_config_section(|_| false)).unwrap()
        };
        let mut local = isolated();
        inherit_line_endings(&mut local, &defaults).unwrap();
        assert_eq!(local.config_snapshot().boolean("core.autocrlf"), Some(false));
        assert_eq!(status(&local).unwrap().len(), 1);
        git(root, &["config", "--unset", "core.autocrlf"]);
        let config_before = fs::read(root.join(".git/config")).unwrap();
        let mut repo = isolated();
        assert_eq!(status(&repo).unwrap().len(), 1);
        // 실제 open 경로와 같이 첫 상태 조회 전에 설정을 적용한다.
        repo = isolated();
        inherit_line_endings(&mut repo, &defaults).unwrap();
        assert_eq!(repo.config_snapshot().boolean("core.autocrlf"), Some(true));
        let clean = status(&repo).unwrap();
        assert!(clean.is_empty(), "정규화 후 남은 변경: {clean:?}");
        fs::write(root.join("file.txt"), "one\r\nactual change\r\n").unwrap();
        assert_eq!(status(&repo).unwrap().len(), 1);
        assert_eq!(fs::read(root.join(".git/config")).unwrap(), config_before);
    }

    #[test]
    fn clean_worktree_ignores_index_stat_updates_and_preserves_real_changes() {
        let dir = fixture(); let root = dir.path();
        fs::write(root.join("clean.txt"), "unchanged\n").unwrap();
        fs::write(root.join("modified.txt"), "base\n").unwrap();
        commit(root, "base");
        let repo = open(root).unwrap();
        let mut index = gix::index::File::at_or_default(repo.index_path(), repo.object_hash(), false, Default::default()).unwrap();
        for entry in index.entries_mut() { entry.stat = Default::default(); }
        IndexLock::acquire(repo.index_path()).unwrap().write(&mut index).unwrap();
        let index_before = fs::read(repo.index_path()).unwrap();
        assert_eq!(git(root, &["--no-optional-locks", "status", "--porcelain"]), "");
        assert!(load_stage(root).unwrap().is_empty());
        assert_eq!(fs::read(repo.index_path()).unwrap(), index_before);
        fs::write(root.join("modified.txt"), "actual modification\n").unwrap();
        let files = load_stage(root).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].label, "modified.txt");
        assert_eq!(files[0].status, "M");
        assert!(!files[0].staged);
        git(root, &["add", "modified.txt"]);
        let files = load_stage(root).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].staged);
    }

    #[test]
    #[ignore = "명시적으로 지정한 깨끗한 외부 저장소의 읽기 전용 검증"]
    fn external_clean_repository_status_is_read_only() {
        let path = std::path::PathBuf::from(std::env::var_os("FEATHER_VERIFY_CLEAN_REPO").expect("검증 저장소 경로가 필요합니다."));
        let repo = open(&path).unwrap();
        let index_before = fs::read(repo.index_path()).unwrap();

        let mut maintenance = 0; let mut meaningful = 0;
        for item in repo.status(gix::progress::Discard).unwrap()
            .index_worktree_options_mut(|options| options.thread_limit = Some(1))
            .untracked_files(gix::status::UntrackedFiles::Files)
            .index_worktree_rewrites(None)
            .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
            .into_iter(Vec::<gix::bstr::BString>::new()).unwrap() {
            match item.unwrap() {
                gix::status::Item::IndexWorktree(item) if item.summary().is_none() => maintenance += 1,
                _ => meaningful += 1,
            }
        }
        let files = load_stage(&path).unwrap();
        eprintln!("maintenance={maintenance}, meaningful={meaningful}, listed={}", files.len());
        assert_eq!(fs::read(repo.index_path()).unwrap(), index_before);
        assert!(files.is_empty(), "깨끗한 저장소에 {}개 항목을 잘못 표시합니다.", files.len());
    }

    #[test]
    fn eof_annotations_follow_actual_file_end_and_ignore_middle_hunks() {
        for newline in ["", "\n"] {
            let dir = fixture(); let root = dir.path();
            fs::write(root.join("file.txt"), format!("old{newline}")).unwrap(); commit(root, "base");
            fs::write(root.join("file.txt"), format!("new \t한{newline}")).unwrap();
            let file = load_stage(root).unwrap().remove(0); let data = diff(root, &file).unwrap();
            let eof_text: Vec<_> = data.eof_rows.iter().map(|row| data.lines[*row].text.as_str()).collect();
            assert_eq!(eof_text, ["old", "new \t한"]);
            assert!(!data.empty_eof);
            let preview = data.partial.as_ref().unwrap();
            assert_eq!(preview.after, format!("new \t한{newline}"));
        }
        let dir = fixture(); let root = dir.path();
        let original = (0..30).map(|i| format!("line {i}\n")).collect::<String>();
        fs::write(root.join("file.txt"), &original).unwrap(); commit(root, "base");
        fs::write(root.join("file.txt"), original.replace("line 10\n", "middle\n")).unwrap();
        let file = load_stage(root).unwrap().remove(0); let data = diff(root, &file).unwrap();
        assert!(data.eof_rows.is_empty()); assert!(!data.empty_eof);
        fs::write(root.join("file.txt"), "last\n\n").unwrap();
        let data = diff(root, &file).unwrap();
        assert!(data.eof_rows.iter().any(|row| data.lines[*row].text.is_empty()));
        let large = (0..6000).map(|index| format!("new line {index}\n")).collect::<String>();
        fs::write(root.join("large.txt"), large).unwrap();
        let file = load_stage(root).unwrap().into_iter().find(|file| file.label == "large.txt").unwrap();
        let data = diff(root, &file).unwrap();
        assert_eq!(data.lines.len(), 5000);
        assert!(data.eof_rows.is_empty());
        assert!(!data.empty_eof);
    }

    #[test]
    fn eof_annotations_handle_added_deleted_empty_and_binary_files() {
        let dir = fixture(); let root = dir.path();
        fs::write(root.join("base.txt"), "base\n").unwrap(); commit(root, "base");
        fs::write(root.join("empty.txt"), "").unwrap();
        fs::write(root.join("new.txt"), "new\n").unwrap();
        fs::write(root.join("binary.bin"), b"a\0b").unwrap();
        fs::remove_file(root.join("base.txt")).unwrap();
        for file in load_stage(root).unwrap() {
            let data = diff(root, &file).unwrap();
            match file.label.as_str() {
                "empty.txt" => { assert!(data.empty_eof); assert!(data.lines.is_empty()); }
                "binary.bin" => { assert!(!data.empty_eof); assert!(data.eof_rows.is_empty()); }
                "new.txt" | "base.txt" => {
                    assert_eq!(data.eof_rows.len(), 1);
                    let line = &data.lines[*data.eof_rows.iter().next().unwrap()];
                    assert!(matches!(line.kind, '+' | '-'));
                }
                _ => panic!("unexpected file"),
            }
        }
    }

    #[test]
    fn whole_file_stage_toggle_preserves_worktree_and_other_staged_changes() {
        for operation in ["modify", "add", "delete"] {
            let dir = fixture(); let root = dir.path();
            fs::create_dir(root.join("nested")).unwrap();
            fs::write(root.join("nested/file.txt"), "base\n").unwrap();
            fs::write(root.join("other.txt"), "base\n").unwrap();
            commit(root, "base");
            fs::write(root.join("other.txt"), "other staged\n").unwrap();
            git(root, &["add", "other.txt"]);
            let name = if operation == "add" { "new.txt" } else { "nested/file.txt" };
            if operation == "delete" {
                fs::remove_file(root.join(name)).unwrap(); fs::remove_dir(root.join("nested")).unwrap();
            } else { fs::write(root.join(name), b"changed\n\0binary").unwrap(); }
            let file = load_stage(root).unwrap().into_iter().find(|f| f.label == name && !f.staged).unwrap();
            toggle_stage(root, &file).unwrap();
            let staged = load_stage(root).unwrap().into_iter().find(|f| f.label == name && f.staged).unwrap();
            assert_eq!(git(root, &["show", ":other.txt"]), "other staged");
            if operation != "delete" { assert_eq!(fs::read(root.join(name)).unwrap(), b"changed\n\0binary"); }
            else { assert!(!root.join(name).exists()); }
            toggle_stage(root, &staged).unwrap();
            assert_eq!(git(root, &["diff", "--cached", "--name-only"]), "other.txt");
            assert_eq!(git(root, &["show", ":other.txt"]), "other staged");
            if operation != "delete" { assert_eq!(fs::read(root.join(name)).unwrap(), b"changed\n\0binary"); }
            else { assert!(!root.join(name).exists()); }
        }
    }

    #[test]
    fn whole_file_toggle_handles_unborn_head_stale_index_and_lock() {
        let dir = fixture(); let root = dir.path();
        fs::write(root.join("new.txt"), "new\n").unwrap();
        let file = load_stage(root).unwrap().remove(0);
        let repo = open(root).unwrap(); let mut lock_name = repo.index_path().as_os_str().to_owned(); lock_name.push(".lock"); let lock = std::path::PathBuf::from(lock_name);
        fs::write(&lock, "owned lock").unwrap();
        assert!(toggle_stage(root, &file).is_err());
        assert_eq!(fs::read(&lock).unwrap(), b"owned lock");
        fs::remove_file(&lock).unwrap();
        toggle_stage(root, &file).unwrap();
        assert!(toggle_stage(root, &file).is_err());
        assert_eq!(git(root, &["show", ":new.txt"]), "new");
        let staged = load_stage(root).unwrap().into_iter().find(|f| f.staged).unwrap();
        toggle_stage(root, &staged).unwrap();
        assert_eq!(git(root, &["ls-files"]), "");
        assert_eq!(fs::read(root.join("new.txt")).unwrap(), b"new\n");
    }

    #[test]
    fn flat_tree_order_keeps_parent_files_first_and_handles_prefixes_and_unicode() {
        let expected = ["A.txt", "a.txt", "index.html", "한글.txt", "a/file.rs", "a/sub/deep.rs", "aa/file.rs", "b/z.rs", "b/a/deep.rs"];
        let mut paths = expected.into_iter().rev().collect::<Vec<_>>();
        paths.sort_by(|a, b| flat_tree_path_order(a.as_bytes(), b.as_bytes()));
        assert_eq!(paths, expected);
        assert_eq!(flat_tree_path_order(b"same/file", b"same/file"), std::cmp::Ordering::Equal);
        assert!(flat_tree_path_order(b"a", b"a/child").is_lt());
        assert!(flat_tree_path_order(b"a/z", b"a/\xff").is_lt());
        let mut files = ["?", "U", "A", "D", "M"].map(|status| FileChange {
            status: status.into(), staged: false, label: "same.txt".into(), path: b"same.txt".to_vec(),
            before: Content::Empty, after: Content::Empty,
        });
        files.sort_by(worktree_file_order);
        assert_eq!(files.iter().map(|file| file.status.as_str()).collect::<Vec<_>>(), ["U", "M", "D", "A", "?"]);
    }

    #[test]
    fn worktree_conflicts_sort_first_within_each_stage_group() {
        let mut files = Vec::new();
        for staged in [false, true] {
            for (status, path) in [
                ("M", "a.txt"), ("U", "nested/z.txt"), ("D", "a.txt"),
                ("?", "a.txt"), ("A", "a.txt"), ("U", "z.txt"),
                ("U", "nested/a.txt"),
            ] {
                files.push(FileChange {
                    status: status.into(), staged, label: path.into(),
                    path: path.as_bytes().to_vec(), before: Content::Empty, after: Content::Empty,
                });
            }
        }
        files.sort_by(worktree_file_order);
        let actual: Vec<_> = files.iter()
            .map(|file| (file.staged, file.status.as_str(), file.label.as_str())).collect();
        let expected: Vec<_> = [true, false].into_iter().flat_map(|staged| {
            [("U", "z.txt"), ("U", "nested/a.txt"), ("U", "nested/z.txt"),
             ("M", "a.txt"), ("D", "a.txt"), ("A", "a.txt"), ("?", "a.txt")]
                .into_iter().map(move |(status, path)| (staged, status, path))
        }).collect();
        assert_eq!(actual, expected);
        let mut empty: Vec<FileChange> = Vec::new();
        empty.sort_by(worktree_file_order);
        assert!(empty.is_empty());
    }
    #[test]
    fn worktree_groups_sort_modified_deleted_added_then_flat_folder_tree() {
        let dir = fixture(); let root = dir.path();
        fs::create_dir_all(root.join("backend/src")).unwrap();
        for name in ["index.html", "backend/src/gemini.rs", "backend/src/a.rs", "backend/z.rs", "z-deleted.txt", "b-deleted.txt"] {
            fs::write(root.join(name), "base\n").unwrap();
        }
        let original = commit(root, "base");
        for name in ["index.html", "backend/src/gemini.rs", "backend/src/a.rs", "backend/z.rs"] {
            fs::write(root.join(name), "modified\n").unwrap();
        }
        fs::remove_file(root.join("z-deleted.txt")).unwrap();
        fs::remove_file(root.join("b-deleted.txt")).unwrap();
        fs::write(root.join("a-added.txt"), "added\n").unwrap();
        fs::write(root.join("b-added.txt"), "added\n").unwrap();
        git(root, &["add", "--", "index.html", "backend/src/gemini.rs", "z-deleted.txt", "a-added.txt"]);
        fs::write(root.join("index.html"), "modified again\n").unwrap();
        let files = load_stage(root).unwrap();
        let actual: Vec<_> = files.iter().map(|file| (file.staged, file.status.as_str(), file.label.as_str())).collect();
        assert_eq!(actual, [
            (true, "M", "index.html"), (true, "M", "backend/src/gemini.rs"),
            (true, "D", "z-deleted.txt"), (true, "A", "a-added.txt"),
            (false, "M", "index.html"), (false, "M", "backend/z.rs"), (false, "M", "backend/src/a.rs"),
            (false, "D", "b-deleted.txt"), (false, "A", "b-added.txt"),
        ]);
        let display = crate::workspace_backend::changed_files(&files, false);
        for (file, row) in files.iter().zip(display) {
            assert_eq!(file.label, row.path);
            assert_eq!(row.status, if file.staged { format!("S·{}", file.status) } else { file.status.clone() });
        }
        let history = load_history(root, &original).unwrap();
        let paths: Vec<_> = history.iter().map(|file| file.path.clone()).collect();
        let mut sorted = paths.clone(); sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn staged_files_precede_all_unstaged_paths_and_keep_display_rows_aligned() {
        let dir = fixture();
        let root = dir.path();
        for name in ["a.txt", "m.txt", "z.txt"] {
            fs::write(root.join(name), "base\n").unwrap();
        }
        commit(root, "base");
        for name in ["m.txt", "z.txt"] {
            fs::write(root.join(name), "staged\n").unwrap();
            git(root, &["add", "--", name]);
        }
        for name in ["a.txt", "m.txt"] {
            fs::write(root.join(name), "worktree\n").unwrap();
        }
        let files = load_stage(root).unwrap();
        let order: Vec<_> = files.iter().map(|file| (file.label.as_str(), file.staged)).collect();
        assert_eq!(order, [("m.txt", true), ("z.txt", true), ("a.txt", false), ("m.txt", false)]);
        let displayed = crate::workspace_backend::changed_files(&files, false);
        let repo = open(root).unwrap();
        for (file, row) in files.iter().zip(&displayed) {
            assert_eq!(row.path, file.label);
            assert_eq!(row.status.starts_with("S·"), file.staged);
            assert_eq!(read_content(&repo, file, &file.after).unwrap(), if file.staged { b"staged\n".to_vec() } else { b"worktree\n".to_vec() });
        }
        assert_eq!(git(root, &["diff", "--name-only"]), "a.txt\nm.txt");
        let historical = load_history(root, &git(root, &["rev-parse", "HEAD"])).unwrap();
        assert_eq!(historical.iter().map(|file| file.label.as_str()).collect::<Vec<_>>(), ["a.txt", "m.txt", "z.txt"]);
    }

    #[test]
    fn staged_and_unstaged_have_independent_sources_and_numbered_context() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("한글.txt"), "one\ntwo\n").unwrap();
        commit(root, "first");
        fs::write(root.join("한글.txt"), "staged\ntwo\n").unwrap();
        git(root, &["add", "."]);
        fs::write(root.join("한글.txt"), "staged\nworktree\n").unwrap();
        let files = load_stage(root).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].staged);
        assert!(!files[1].staged);
        let repo = open(root).unwrap();
        assert_eq!(
            read_content(&repo, &files[0], &files[0].before).unwrap(),
            b"one\ntwo\n"
        );
        assert_eq!(
            read_content(&repo, &files[0], &files[0].after).unwrap(),
            b"staged\ntwo\n"
        );
        assert_eq!(
            read_content(&repo, &files[1], &files[1].before).unwrap(),
            b"staged\ntwo\n"
        );
        let staged = diff(root, &files[0]).unwrap();
        assert!(staged.partial.is_none());
        let work = diff(root, &files[1]).unwrap();
        assert!(work.partial.is_some());
        assert!(work.lines.iter().any(|line| line.kind == '-'
            && line.old == Some(2)
            && line.new.is_none()
            && line.text == "two"));
        assert!(work.lines.iter().any(|line| line.kind == '+'
            && line.new == Some(2)
            && line.old.is_none()
            && line.text == "worktree"));
    }
    #[test]
    fn history_uses_empty_root_and_first_parent_of_merge_and_full_detail() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("base.txt"), "base\n").unwrap();
        let first = commit(root, "title\n\nbody 한글");
        let initial = load_history(root, &first).unwrap();
        assert!(matches!(initial[0].before, Content::Empty));
        assert!(
            diff(root, &initial[0])
                .unwrap()
                .lines
                .iter()
                .any(|l| l.kind == '+' && l.new == Some(1))
        );
        git(root, &["checkout", "-b", "side"]);
        fs::write(root.join("side.txt"), "side\n").unwrap();
        commit(root, "side");
        git(root, &["checkout", "main"]);
        fs::write(root.join("main.txt"), "main\n").unwrap();
        commit(root, "main");
        git(root, &["merge", "--no-ff", "side", "-m", "merge"]);
        let id = git(root, &["rev-parse", "HEAD"]);
        let changes = load_history(root, &id).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].label, "side.txt");
        let detail = load_detail(root, &id).unwrap();
        assert_eq!(detail.parent_ids.len(), 2);
        assert_eq!(detail.author_email, "diff@example.invalid");
        assert_eq!(
            load_detail(root, &first).unwrap().message,
            "title\n\nbody 한글\n"
        );
    }
    #[test]
    fn preview_limits_non_text_special_paths_and_truncation_are_explicit() {
        let dir = fixture();
        let root = dir.path();
        let repo = open(root).unwrap();
        for (name, bytes, note) in [
            ("binary", vec![0, 1], "바이너리"),
            ("encoded", vec![0xff, 0xfe], "UTF-8"),
            ("huge", vec![b'a'; MAX_BYTES as usize + 1], "1 MiB"),
        ] {
            fs::write(root.join(name), bytes).unwrap();
            let file = load_stage(root)
                .unwrap()
                .into_iter()
                .find(|f| f.label == name)
                .unwrap();
            let diff = diff(root, &file).unwrap();
            assert!(diff.lines.is_empty());
            assert!(diff.note.contains(note), "{}", diff.note);
        }
        let file = FileChange {
            path: b"../outside".to_vec(),
            label: "outside".into(),
            status: "A".into(),
            staged: false,
            before: Content::Empty,
            after: Content::Worktree,
        };
        assert!(read_content(&repo, &file, &file.after).is_err());
        let id = repo.write_blob(b"target").unwrap().detach();
        assert!(read_content(&repo, &file, &Content::Object(id, 0o120000)).is_err());
        let mut file = file;
        file.path = b"long.txt".to_vec();
        fs::write(
            root.join("long.txt"),
            (0..6000).map(|i| format!("line {i}\n")).collect::<String>(),
        )
        .unwrap();
        let data = diff(root, &file).unwrap();
        assert_eq!(data.lines.len(), 5000);
        assert!(data.note.contains("5,000"));
        file.before = Content::Worktree;
        let same = diff(root, &file).unwrap();
        assert!(same.lines.is_empty());
        assert!(same.note.contains("동일"));
    }
    #[test]
    fn conflict_resolution_and_whole_stage_reject_stale_and_preserve_worktree() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("conflict.txt"), "base\n").unwrap();
        commit(root, "base");
        git(root, &["checkout", "-b", "incoming"]);
        fs::write(root.join("conflict.txt"), "incoming\n").unwrap();
        commit(root, "incoming");
        git(root, &["checkout", "main"]);
        fs::write(root.join("conflict.txt"), "current\n").unwrap();
        commit(root, "current");
        let merge = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=diff@example.invalid",
                "merge",
                "incoming",
            ])
            .output()
            .unwrap();
        assert!(!merge.status.success());
        let file = load_stage(root)
            .unwrap()
            .into_iter()
            .find(|f| f.status == "U")
            .unwrap();
        let preview = diff(root, &file).unwrap();
        let before = fs::read(root.join(".git/index")).unwrap();
        assert!(!preview.conflict_resolved);
        assert!(stage_resolved(root, &file, &preview).is_err());
        assert_eq!(before, fs::read(root.join(".git/index")).unwrap());
        crate::conflict::apply(
            root,
            &file.path,
            preview.conflict.as_ref().unwrap(),
            0,
            crate::conflict::Choice::Both,
        )
        .unwrap();
        let resolved = diff(root, &file).unwrap();
        assert!(resolved.conflict_resolved);
        fs::write(root.join("conflict.txt"), "external\n").unwrap();
        assert!(stage_resolved(root, &file, &resolved).is_err());
        fs::write(
            root.join("conflict.txt"),
            resolved.worktree.as_ref().unwrap(),
        )
        .unwrap();
        fs::write(root.join(".git/index.lock"), b"busy").unwrap();
        assert!(stage_resolved(root, &file, &resolved).is_err());
        assert_eq!(fs::read(root.join(".git/index.lock")).unwrap(), b"busy");
        fs::remove_file(root.join(".git/index.lock")).unwrap();
        stage_resolved(root, &file, &resolved).unwrap();
        assert_eq!(git(root, &["show", ":conflict.txt"]), "current\nincoming");
        assert_eq!(
            fs::read(root.join("conflict.txt")).unwrap(),
            resolved.worktree.unwrap()
        );
        assert!(!load_stage(root).unwrap().iter().any(|f| f.status == "U"));
        assert!(stage_resolved(root, &file, &preview).is_err());
    }
    #[test]
    fn index_lock_blocks_partial_stage_without_touching_existing_lock() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("file.txt"), "before\n").unwrap();
        commit(root, "base");
        fs::write(root.join("file.txt"), "after\n").unwrap();
        let file = load_stage(root).unwrap().remove(0);
        let preview = diff(root, &file).unwrap();
        let index = fs::read(root.join(".git/index")).unwrap();
        fs::write(root.join(".git/index.lock"), b"other process").unwrap();
        assert!(
            crate::partial_stage::apply(
                root,
                &file,
                preview.partial.as_ref().unwrap(),
                &preview.lines,
                &crate::partial_stage::hunk_rows(&preview.lines, 0)
            )
            .is_err()
        );
        assert_eq!(
            fs::read(root.join(".git/index.lock")).unwrap(),
            b"other process"
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    use crate::text_encoding::TextEncoding;
    fn bytes(encoding: TextEncoding, text: &str) -> Vec<u8> {
        match encoding {
            TextEncoding::Utf16Le => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            TextEncoding::Utf16Be => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
            TextEncoding::Windows949 | TextEncoding::EucKr => {
                encoding_rs::EUC_KR.encode(text).0.into_owned()
            }
            _ => text.as_bytes().to_vec(),
        }
    }
    #[test]
    fn encoding_diff_preserves_index_worktree_and_disables_non_utf8_writes() {
        for encoding in [
            TextEncoding::Utf8,
            TextEncoding::Windows949,
            TextEncoding::EucKr,
            TextEncoding::Utf16Le,
            TextEncoding::Utf16Be,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            gix::init(root).unwrap();
            let before = bytes(encoding, "원문\r\n");
            let after = bytes(encoding, "변경\r\n");
            set_index_content(root, b"file.txt", Some(&before), 0o100644, None).unwrap();
            fs::write(root.join("file.txt"), &after).unwrap();
            let file = load_stage(root)
                .unwrap()
                .into_iter()
                .find(|f| !f.staged)
                .unwrap();
            let index = fs::read(root.join(".git/index")).unwrap();
            let config = fs::read(root.join(".git/config")).unwrap();
            let data = diff_with_encoding(root, &file, encoding).unwrap();
            assert!(
                data.lines
                    .iter()
                    .any(|line| line.kind == '-' && line.text == "원문")
            );
            assert!(
                data.lines
                    .iter()
                    .any(|line| line.kind == '+' && line.text == "변경")
            );
            assert_eq!(data.partial.is_some(), encoding == TextEncoding::Utf8);
            assert_eq!(data.worktree.as_deref(), Some(after.as_slice()));
            assert_eq!(fs::read(root.join("file.txt")).unwrap(), after);
            assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
            assert_eq!(fs::read(root.join(".git/config")).unwrap(), config);
            assert_eq!(
                read_content(&open(root).unwrap(), &file, &file.before).unwrap(),
                before
            );
            let mut conflicted = file.clone();
            conflicted.status = "U".into();
            let conflict_bytes = bytes(
                encoding,
                "<<<<<<< HEAD\r\n원문\r\n=======\r\n변경\r\n>>>>>>> other\r\n",
            );
            fs::write(root.join("file.txt"), &conflict_bytes).unwrap();
            let data = diff_with_encoding(root, &conflicted, encoding).unwrap();
            assert_eq!(data.conflict.is_some(), encoding == TextEncoding::Utf8);
            assert!(!data.conflict_resolved);
            assert_eq!(fs::read(root.join("file.txt")).unwrap(), conflict_bytes);
        }
    }
    #[test]
    fn utf16_binary_and_decode_errors_are_explicit_and_utf8_can_be_restored() {
        let dir = tempfile::tempdir().unwrap();
        gix::init(dir.path()).unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, [0x61, 0, 0, 0]).unwrap();
        let file = load_stage(dir.path()).unwrap().remove(0);
        let data = diff_with_encoding(dir.path(), &file, TextEncoding::Utf16Le).unwrap();
        assert!(data.note.contains("바이너리"));
        fs::write(&path, [0x00, 0xd8]).unwrap();
        let data = diff_with_encoding(dir.path(), &file, TextEncoding::Utf16Le).unwrap();
        assert!(data.note.contains("UTF-16LE"));
        assert!(data.lines.is_empty());
        fs::write(&path, "복귀\n").unwrap();
        let data = diff(dir.path(), &file).unwrap();
        assert!(data.lines.iter().any(|line| line.text == "복귀"));
        assert!(data.partial.is_some());
    }
}
