use anyhow::{Context, Result, bail};
use gix::bstr::{BStr, ByteSlice};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn repo(path: &Path) -> Result<gix::Repository> {
    repo_with_options(path, open_options())
}

pub(crate) fn repo_with_options(
    path: &Path,
    options: gix::open::Options,
) -> Result<gix::Repository> {
    let mut repo = gix::discover_opts(path, Default::default(), options)?;
    let mut combined = gix::config::File::default();
    for path in system_configs() {
        combined.append(read_config_file(&path)?)?;
    }
    combined.append(repo.config_snapshot().plumbing().clone())?;
    let mut snapshot = repo.config_snapshot_mut();
    *snapshot = combined;
    snapshot.commit()?;
    Ok(repo)
}

pub fn open_options() -> gix::open::Options {
    let mut options = gix::open::Options::default();
    options.permissions.config.git_binary = false;
    options.permissions.config.system = false;
    options.permissions.attributes.git_binary = false;
    options.permissions.attributes.system = false;
    options
}

fn system_configs() -> Vec<PathBuf> {
    if std::env::var("GIT_CONFIG_NOSYSTEM")
        .is_ok_and(|s| ["1", "true", "yes"].contains(&s.as_str()))
    {
        return Vec::new();
    }
    if let Some(path) = std::env::var_os("GIT_CONFIG_SYSTEM") {
        return vec![PathBuf::from(path)]
            .into_iter()
            .filter(|p| p.is_file())
            .collect();
    }
    let mut paths = Vec::new();
    #[cfg(windows)]
    {
        if let Some(base) = std::env::var_os("ProgramData") {
            paths.push(PathBuf::from(base).join("Git/config"));
        }
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(base) = std::env::var_os(variable) {
                paths.push(PathBuf::from(base).join("Git/etc/gitconfig"));
            }
        }
        if let Some(search) = std::env::var_os("PATH") {
            for entry in std::env::split_paths(&search) {
                if entry.join("git.exe").is_file() {
                    for parent in entry.ancestors().take(3) {
                        paths.push(parent.join("etc/gitconfig"));
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    paths.push(PathBuf::from("/etc/gitconfig"));
    let mut seen = std::collections::BTreeSet::new();
    paths
        .into_iter()
        .filter(|p| p.is_file())
        .filter_map(|p| p.canonicalize().ok())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

pub fn config_path(
    path: &Path,
    global: bool,
    env: &BTreeMap<OsString, OsString>,
) -> Result<PathBuf> {
    if global {
        if let Some(path) = env
            .get(std::ffi::OsStr::new("GIT_CONFIG_GLOBAL"))
            .cloned()
            .or_else(|| std::env::var_os("GIT_CONFIG_GLOBAL"))
        {
            return Ok(path.into());
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .context("전역 설정 경로를 찾을 수 없습니다.")?;
        let home = PathBuf::from(home).join(".gitconfig");
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.parent().unwrap().join(".config"))
            .join("git/config");
        return Ok(if home.exists() || !xdg.exists() {
            home
        } else {
            xdg
        });
    }
    Ok(repo(path)?.common_dir().join("config"))
}

pub fn read_config_file(path: &Path) -> Result<gix::config::File> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    Ok(gix::config::File::from_bytes_no_includes(
        &bytes,
        gix::config::file::Metadata::from(gix::config::Source::Local),
        Default::default(),
    )?)
}

pub fn config_values(
    path: &Path,
    scope: Option<&str>,
    key: &str,
    env: &BTreeMap<OsString, OsString>,
) -> Result<Vec<String>> {
    let values = if scope.is_some() {
        read_config_file(&config_path(path, scope == Some("--global"), env)?)?
            .raw_values(key)
            .unwrap_or_default()
    } else if env.contains_key(std::ffi::OsStr::new("GIT_CONFIG_GLOBAL")) {
        let mut values = config_values(path, Some("--global"), key, env)?;
        if repo(path).is_ok() {
            values.extend(config_values(path, Some("--local"), key, env)?);
        }
        return Ok(values);
    } else if let Ok(repo) = repo(path) {
        repo.config_snapshot()
            .plumbing()
            .raw_values(key)
            .unwrap_or_default()
    } else {
        return config_values(path, Some("--global"), key, env);
    };
    values
        .into_iter()
        .map(|v| Ok(String::from_utf8(v.to_vec())?))
        .collect()
}

pub fn config_set(
    path: &Path,
    global: bool,
    key: &str,
    values: &[String],
    env: &BTreeMap<OsString, OsString>,
) -> Result<()> {
    edit_config(path, global, env, |config| {
        if let Ok(mut existing) = config.raw_values_mut(key) {
            existing.delete_all();
        }
        let (section, field) = key.rsplit_once('.').context("잘못된 설정 키입니다.")?;
        let (section, subsection) = section.split_once('.').map_or((section, None), |(s, sub)| {
            (s, Some(sub.as_bytes().as_bstr()))
        });
        for value in values {
            config
                .new_section(section, subsection.map(|s| s.to_owned()))?
                .push(field, Some(value.as_bytes().as_bstr()))?;
        }
        Ok(())
    })
}

pub fn add_remote(path: &Path, name: &str, url: &str) -> Result<()> {
    if name.chars().any(|c| c.is_control()) || url.chars().any(|c| c.is_control()) {
        bail!("원격 이름과 URL에는 제어문자를 사용할 수 없습니다.");
    }
    let name = name.trim();
    let url = url.trim();
    if name.is_empty()
        || !git2::Reference::is_valid_name(&format!("refs/remotes/{name}/branch"))
        || name.starts_with('-')
        || name.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        bail!("원격 이름을 올바르게 입력하세요.");
    }
    if url.is_empty() || url.starts_with('-') || url.chars().any(|c| c.is_control()) {
        bail!("원격 URL을 올바르게 입력하세요.");
    }
    edit_config(path, false, &BTreeMap::new(), |config| {
        let repo = repo(path)?;
        let snapshot = repo.config_snapshot();
        for file in [config as &gix::config::File, snapshot.plumbing()] {
            if file.sections_by_name("remote").is_some_and(|mut sections| {
                sections.any(|section| {
                    section.value_names().next().is_some()
                        && section
                            .header()
                            .subsection_name()
                            .is_some_and(|sub| sub == name.as_bytes())
                })
            }) {
                bail!("같은 이름의 원격 저장소가 이미 있습니다.");
            }
        }
        let fetch = format!("+refs/heads/*:refs/remotes/{name}/*");
        let mut section =
            config.new_section("remote", Some(name.as_bytes().as_bstr().to_owned()))?;
        section.push("url", Some(url.as_bytes().as_bstr()))?;
        section.push("fetch", Some(fetch.as_bytes().as_bstr()))?;
        Ok(())
    })
}

fn edit_config(
    path: &Path,
    global: bool,
    env: &BTreeMap<OsString, OsString>,
    edit: impl FnOnce(&mut gix::config::File) -> Result<()>,
) -> Result<()> {
    let path = config_path(path, global, env)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lock_name = path.as_os_str().to_owned();
    lock_name.push(".lock");
    let lock_path = PathBuf::from(lock_name);
    let mut lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .context("Git 설정 파일이 잠겨 있습니다.")?;
    let result = (|| {
        let mut config = read_config_file(&path)?;
        edit(&mut config)?;
        config.write_to(&mut lock)?;
        lock.flush()?;
        lock.sync_all()?;
        drop(lock);
        fs::rename(&lock_path, &path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&lock_path);
    }
    result
}

pub fn index(path: &Path, alternative: Option<&std::ffi::OsStr>) -> Result<gix::index::File> {
    let repo = repo(path)?;
    let path = alternative
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.index_path());
    if fs::metadata(&path).is_ok_and(|m| m.len() < (12 + repo.object_hash().len_in_bytes()) as u64)
    {
        bail!("인덱스 파일이 손상되었습니다.");
    }
    Ok(gix::index::File::at_or_default(
        path,
        repo.object_hash(),
        false,
        Default::default(),
    )?)
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

pub fn expand_index(path: &Path) -> Result<()> {
    let repo = repo(path)?;
    let lock = IndexLock::acquire(repo.index_path())?;
    let mut index = index(path, None)?;
    lock.write(&mut index)
}

pub fn set_index_content(
    path: &Path,
    raw: &[u8],
    content: Option<&[u8]>,
    mode: u32,
    expected: Option<(gix::ObjectId, u32)>,
) -> Result<()> {
    let repo = repo(path)?;
    let lock = IndexLock::acquire(repo.index_path())?;
    let mut index = index(path, None)?;
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

pub fn index_tree(repo: &gix::Repository, index: &gix::index::State) -> Result<gix::ObjectId> {
    if index
        .entries()
        .iter()
        .any(|e| e.stage() != gix::index::entry::Stage::Unconflicted)
    {
        bail!("미해결 충돌 파일이 남아 있습니다.");
    }
    let entries = index
        .entries()
        .iter()
        .map(|e| (e.path(index).to_vec(), e.id, e.mode.bits()))
        .collect::<Vec<_>>();
    fn tree(
        repo: &gix::Repository,
        entries: &[(Vec<u8>, gix::ObjectId, u32)],
    ) -> Result<gix::ObjectId> {
        let mut children = BTreeMap::<Vec<u8>, Vec<(Vec<u8>, gix::ObjectId, u32)>>::new();
        let mut result = gix::objs::Tree {
            entries: Vec::new(),
        };
        for (path, id, mode) in entries {
            if let Some(pos) = path.iter().position(|b| *b == b'/') {
                children.entry(path[..pos].to_vec()).or_default().push((
                    path[pos + 1..].to_vec(),
                    *id,
                    *mode,
                ));
            } else {
                result.entries.push(gix::objs::tree::Entry {
                    mode: gix::objs::tree::EntryMode::try_from(*mode)
                        .map_err(|_| anyhow::anyhow!("Invalid index mode"))?,
                    filename: path.clone().into(),
                    oid: *id,
                });
            }
        }
        for (name, entries) in children {
            result.entries.push(gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Tree.into(),
                filename: name.into(),
                oid: tree(repo, &entries)?,
            });
        }
        result.entries.sort();
        Ok(repo.write_object(&result)?.detach())
    }
    tree(repo, &entries)
}

pub fn commit(path: &Path, message: &str) -> Result<()> {
    if message.trim().is_empty() || message.contains('\0') {
        bail!("커밋 메시지를 입력하세요.");
    }
    let repo = repo(path)?;
    let index = index(path, None)?;
    let tree = index_tree(&repo, &index)?;
    let parent = repo.head_id().ok().map(|id| id.detach());
    if let Some(parent) = parent {
        if repo.find_commit(parent)?.tree_id()?.detach() == tree {
            bail!("staged 변경사항이 없습니다.");
        }
    } else if index.entries().is_empty() {
        bail!("staged 변경사항이 없습니다.");
    }
    create_commit(
        &repo,
        tree,
        &parent.into_iter().collect::<Vec<_>>(),
        message,
        None,
    )?;
    Ok(())
}

pub fn create_commit(
    repo: &gix::Repository,
    tree: gix::ObjectId,
    parents: &[gix::ObjectId],
    message: &str,
    author: Option<gix::actor::Signature>,
) -> Result<gix::ObjectId> {
    use gix::refs::{
        Target,
        transaction::{Change, PreviousValue, RefEdit},
    };
    let committer: gix::actor::Signature = repo
        .committer()
        .context("커미터 이름과 이메일을 설정하세요.")??
        .into();
    let author = match author {
        Some(author) => author,
        None => repo
            .author()
            .context("작성자 이름과 이메일을 설정하세요.")??
            .into(),
    };
    for identity in [&author, &committer] {
        if identity.name.trim().is_empty() || identity.email.trim().is_empty() {
            bail!("작성자와 커미터의 이름 및 이메일을 설정하세요.");
        }
    }
    let mut commit = gix::objs::Commit {
        tree,
        parents: parents.iter().copied().collect(),
        author,
        committer,
        encoding: None,
        message: message.into(),
        extra_headers: Default::default(),
    };
    if let Some(options) = repo.commit_signing_options_if_enabled()? {
        commit = commit.sign(options)?;
    }
    let id = repo.write_object(&commit)?.detach();
    repo.edit_reference(RefEdit {
        name: "HEAD".try_into()?,
        deref: true,
        change: Change::Update {
            log: gix::refs::transaction::LogChange {
                message: format!("commit: {}", message.lines().next().unwrap_or_default()).into(),
                ..Default::default()
            },
            expected: parents
                .first()
                .map(|p| PreviousValue::MustExistAndMatch(Target::Object(*p)))
                .unwrap_or(PreviousValue::MustNotExist),
            new: Target::Object(id),
        },
    })?;
    Ok(id)
}

pub fn oid(repo: &gix::Repository, revision: &str) -> Result<gix::ObjectId> {
    Ok(repo.rev_parse_single(BStr::new(revision))?.detach())
}

pub fn stage_paths(path: &Path, paths: &[PathBuf], staged: bool, remove: bool) -> Result<()> {
    use std::collections::BTreeSet;
    let repo = repo(path)?;
    let lock = IndexLock::acquire(repo.index_path())?;
    let root = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    let mut index = index(path, None)?;
    let mut selected = BTreeSet::new();
    fn head_paths(
        repo: &gix::Repository,
        tree: &gix::Tree<'_>,
        prefix: &[u8],
        out: &mut Vec<Vec<u8>>,
    ) -> Result<()> {
        for entry in tree.iter() {
            let entry = entry?;
            let mut raw = prefix.to_vec();
            raw.extend_from_slice(entry.filename());
            if entry.mode().is_tree() {
                raw.push(b'/');
                head_paths(repo, &repo.find_tree(entry.object_id())?, &raw, out)?;
            } else {
                out.push(raw);
            }
        }
        Ok(())
    }
    let mut previous = Vec::new();
    if !staged && let Ok(head) = repo.head_tree() {
        head_paths(&repo, &head, &[], &mut previous)?;
    }
    for relative in paths {
        let raw = gix::path::into_bstr(relative.clone()).replace(b"\\", b"/");
        if !root.join(relative).is_dir() {
            selected.insert(raw.clone());
        }
        for old in &previous {
            if old == &raw || old.starts_with(&[raw.as_slice(), b"/"].concat()) {
                selected.insert(old.clone());
            }
        }
        for entry in index.entries() {
            let p = entry.path(&index);
            if p.as_bytes() == raw || p.starts_with(&[raw.as_slice(), b"/"].concat()) {
                selected.insert(p.to_vec());
            }
        }
        if staged {
            fn visit(root: &Path, p: &Path, out: &mut BTreeSet<Vec<u8>>) -> Result<()> {
                let full = root.join(p);
                let meta = match fs::symlink_metadata(&full) {
                    Ok(v) => v,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(e) => return Err(e.into()),
                };
                if meta.is_dir() && !meta.file_type().is_symlink() {
                    if full.join(".git").exists() {
                        return Ok(());
                    }
                    for entry in fs::read_dir(full)? {
                        let entry = entry?;
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(".git")
                        {
                            continue;
                        }
                        visit(root, &p.join(entry.file_name()), out)?;
                    }
                } else {
                    out.insert(gix::path::into_bstr(p.to_owned()).replace(b"\\", b"/"));
                }
                Ok(())
            }
            visit(root, relative, &mut selected)?;
        }
    }
    let head = repo.head_tree().ok();
    let mut updates = Vec::new();
    let mut attrs = repo.attributes(
        &index,
        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
        gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        None,
    )?;
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    for raw in &selected {
        if staged {
            let relative = gix::path::from_bstr(raw.as_bstr());
            let tracked = index.entry_by_path(raw.as_bstr()).is_some();
            if !tracked && attrs.at_path(relative.as_ref(), None)?.is_excluded() {
                continue;
            }
            let value = pipeline.worktree_file_to_object(raw.as_bstr(), &index)?;
            updates.push((
                raw.clone(),
                value.map(|(id, kind, _)| {
                    let mut mode = gix::index::entry::Mode::from(kind).bits();
                    if mode & 0o170000 == 0o100000
                        && !repo
                            .config_snapshot()
                            .boolean("core.filemode")
                            .unwrap_or(true)
                        && let Some(entry) = index.entry_by_path(raw.as_bstr())
                        && entry.mode.bits() & 0o170000 == 0o100000
                    {
                        mode = entry.mode.bits();
                    }
                    (id, mode)
                }),
            ));
        } else {
            let entry = if remove {
                None
            } else {
                head.as_ref()
                    .map(|t| t.lookup_entry(raw.split(|b| *b == b'/')))
                    .transpose()?
                    .flatten()
            };
            updates.push((
                raw.clone(),
                entry.map(|e| (e.object_id(), e.mode().value() as u32)),
            ));
        }
    }
    drop(attrs);
    for (raw, value) in updates {
        index.remove_entries(|_, p, _| p.as_bytes() == raw);
        if let Some((id, mode)) = value {
            index.dangerously_push_entry(
                Default::default(),
                id,
                gix::index::entry::Flags::empty(),
                gix::index::entry::Mode::from_bits_truncate(mode),
                raw.as_bstr(),
            );
        }
    }
    index.sort_entries();
    lock.write(&mut index)?;
    Ok(())
}
