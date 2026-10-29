use crate::{git, model::StashRestoreState};
use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

const AREA: &str = "feather-stash-recovery";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    path: Vec<u8>,
    value: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct SavedFile {
    path: Vec<u8>,
    hash: Option<String>,
    readonly: bool,
    mode: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    state: StashRestoreState,
    index_present: bool,
    index_hash: String,
    full_index_hash: Option<String>,
    tree: Option<String>,
    entries: Vec<Entry>,
    files: Vec<SavedFile>,
    touched: BTreeSet<Vec<u8>>,
    missing_dirs: BTreeSet<Vec<u8>>,
}

struct Locations {
    root: PathBuf,
    git_dir: PathBuf,
    area: PathBuf,
}

struct Lock {
    path: PathBuf,
    _file: fs::File,
}
impl Lock {
    fn acquire(path: PathBuf) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("다른 Git/복원 작업의 잠금이 남아 있습니다. 작업 종료 여부를 확인하세요.")?;
        Ok(Self { path, _file: file })
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn linked(meta: &fs::Metadata) -> bool {
    let mut result = meta.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        result |= meta.file_attributes() & 0x400 != 0;
    }
    result
}

fn plain_dir(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !linked(&meta) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)?;
            Ok(())
        }
        _ => bail!("복원 백업 폴더가 일반 디렉터리가 아닙니다."),
    }
}

fn safe_directory(root: &Path, raw: &[u8]) -> Result<PathBuf> {
    let relative = relative_path(raw)?;
    let full = root.join(&relative);
    let mut path = root.to_owned();
    for part in relative.components() {
        path.push(part.as_os_str());
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() && !linked(&meta) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(full),
            _ => bail!("복원 부모 디렉터리가 링크 또는 다른 파일로 변경되었습니다."),
        }
    }
    Ok(full)
}

fn locations(path: &Path) -> Result<Locations> {
    let repo = git::open(path)?;
    let root = repo
        .workdir()
        .context("작업 디렉터리가 없습니다")?
        .canonicalize()?;
    let git_dir = repo.path().canonicalize()?;
    Ok(Locations {
        root,
        area: git_dir.join(AREA),
        git_dir,
    })
}

fn git_path(path: &Path) -> std::ffi::OsString {
    #[cfg(windows)]
    {
        let text = path.as_os_str().to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}").into();
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return rest.into();
        }
    }
    path.as_os_str().to_owned()
}

fn command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root).env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn run(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = command(root).args(args).output()?;
    if !output.status.success() {
        bail!(
            "Git 복원 상태 조회 실패: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}

fn head(root: &Path) -> Result<String> {
    Ok(
        String::from_utf8(run(root, &["rev-parse", "--verify", "HEAD"])?)?
            .trim()
            .into(),
    )
}
fn branch(root: &Path) -> Result<Option<String>> {
    let output = command(root)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()?;
    match output.status.code() {
        Some(0) => Ok(Some(String::from_utf8(output.stdout)?.trim().into())),
        Some(1) => Ok(None),
        _ => bail!("현재 브랜치를 확인하지 못했습니다."),
    }
}

fn relative_path(raw: &[u8]) -> Result<PathBuf> {
    let relative = gix::path::from_bstr(raw.as_bstr());
    if raw.is_empty() || relative.is_absolute() {
        bail!("저장소 내부 파일만 복구할 수 있습니다.");
    }
    for part in relative.components() {
        let Component::Normal(name) = part else {
            bail!("잘못된 복원 경로입니다.");
        };
        if name.to_string_lossy().eq_ignore_ascii_case(".git") {
            bail!("Git 메타데이터 경로는 복원할 수 없습니다.");
        }
        #[cfg(windows)]
        if name.to_string_lossy().contains(':') {
            bail!("지원하지 않는 복원 경로입니다.");
        }
    }
    Ok(relative.into_owned())
}

fn safe_path(root: &Path, raw: &[u8]) -> Result<PathBuf> {
    let relative = relative_path(raw)?;
    let mut path = root.to_owned();
    let count = relative.components().count();
    for (index, part) in relative.components().enumerate() {
        path.push(part.as_os_str());
        match fs::symlink_metadata(&path) {
            Ok(meta) if linked(&meta) || (index + 1 < count && !meta.is_dir()) => {
                bail!("링크 또는 파일/디렉터리 경계는 안전한 Stash 복원을 지원하지 않습니다.")
            }
            Ok(meta) if index + 1 == count && !meta.is_file() => {
                bail!("일반 파일만 안전한 Stash 복원을 지원합니다.")
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

fn hash(path: &Path) -> Result<String> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || linked(&meta) {
        bail!("일반 백업 파일이 아닙니다.");
    }
    Ok(gix::objs::compute_stream_hash(
        gix::hash::Kind::Sha1,
        gix::objs::Kind::Blob,
        &mut fs::File::open(path)?,
        meta.len(),
        &mut gix::progress::Discard,
        &AtomicBool::new(false),
    )?
    .to_string())
}

fn file_hash(path: &Path) -> Result<Option<String>> {
    match fs::symlink_metadata(path) {
        Ok(_) => hash(path).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn entries(root: &Path) -> Result<Vec<Entry>> {
    let bytes = run(root, &["ls-files", "--stage", "-z"])?;
    bytes
        .split(|byte| *byte == 0)
        .filter(|row| !row.is_empty())
        .map(|row| {
            let split = row
                .iter()
                .position(|byte| *byte == b'\t')
                .context("인덱스 항목을 읽지 못했습니다.")?;
            Ok(Entry {
                value: row[..split].to_vec(),
                path: row[split + 1..].to_vec(),
            })
        })
        .collect()
}

fn grouped(entries: &[Entry]) -> BTreeMap<&[u8], Vec<&[u8]>> {
    let mut grouped: BTreeMap<&[u8], Vec<&[u8]>> = BTreeMap::new();
    for entry in entries {
        grouped.entry(&entry.path).or_default().push(&entry.value);
    }
    grouped
}

fn unmerged(root: &Path) -> Result<Vec<Vec<u8>>> {
    Ok(entries(root)?
        .into_iter()
        .filter(|entry| !entry.value.ends_with(b" 0"))
        .map(|entry| entry.path)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn ensure_idle(loc: &Locations) -> Result<()> {
    if [
        "MERGE_HEAD",
        "REVERT_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
        "index.lock",
    ]
    .iter()
    .any(|name| loc.git_dir.join(name).exists())
    {
        bail!("다른 Git 작업이나 인덱스 잠금이 남아 있습니다.");
    }
    Ok(())
}

fn job(loc: &Locations, id: &str) -> Result<PathBuf> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!("잘못된 복원 세션입니다.");
    }
    let path = loc.area.join(id);
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_dir()
        || linked(&meta)
        || path.canonicalize()?.parent() != Some(loc.area.canonicalize()?.as_path())
    {
        bail!("복원 백업 경로가 변경되었습니다.");
    }
    Ok(path)
}

fn load(loc: &Locations) -> Result<Option<Manifest>> {
    match fs::symlink_metadata(&loc.area) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(meta) if meta.is_dir() && !linked(&meta) => {}
        _ => bail!("복원 백업 폴더가 변경되었습니다."),
    }
    let path = loc.area.join("active.json");
    let raw = match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(meta) if meta.is_file() && !linked(&meta) && meta.len() <= 64 * 1024 * 1024 => {
            fs::read(&path)?
        }
        _ => bail!("복원 상태 파일을 읽을 수 없습니다."),
    };
    let manifest: Manifest = serde_json::from_slice(&raw)?;
    if manifest.version != 1 || manifest.state.root != loc.root {
        bail!("복원 상태의 저장소 또는 버전이 올바르지 않습니다.");
    }
    job(loc, &manifest.state.id)?;
    let mut paths = BTreeSet::new();
    for file in &manifest.files {
        relative_path(&file.path)?;
        if !paths.insert(file.path.clone()) {
            bail!("복원 백업에 중복 경로가 있습니다.");
        }
    }
    if !manifest.touched.is_subset(&paths) {
        bail!("복원 대상 범위가 변경되었습니다.");
    }
    for raw in &manifest.missing_dirs {
        let parent = relative_path(raw)?;
        if !manifest.files.iter().any(|file| {
            relative_path(&file.path).is_ok_and(|path| path != parent && path.starts_with(&parent))
        }) {
            bail!("복원 부모 디렉터리 범위가 변경되었습니다.");
        }
    }
    Ok(Some(manifest))
}

fn save(loc: &Locations, manifest: &Manifest) -> Result<()> {
    let temp = loc.area.join("active.tmp");
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)?;
    let result = (|| -> Result<()> {
        file.write_all(&serde_json::to_vec(manifest)?)?;
        file.sync_all()?;
        fs::rename(&temp, loc.area.join("active.json"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

fn retire(loc: &Locations, manifest: &Manifest) -> Result<()> {
    let dir = job(loc, &manifest.state.id)?;
    fs::remove_file(loc.area.join("active.json"))?;
    for index in 0..manifest.files.len() {
        let _ = fs::remove_file(dir.join(index.to_string()));
    }
    let _ = fs::remove_file(dir.join("index"));
    let _ = fs::remove_file(dir.join("full-index"));
    if let Some(tree) = &manifest.tree {
        let _ = command(&loc.root)
            .args([
                "update-ref",
                "-d",
                &format!("refs/feather-stash-recovery/{}", manifest.state.id),
                tree,
            ])
            .output();
    }
    let _ = fs::remove_dir(dir);
    Ok(())
}

pub fn state(path: &Path) -> Result<Option<StashRestoreState>> {
    if git::open(path)?.workdir().is_none() {
        return Ok(None);
    }
    let loc = locations(path)?;
    let Some(mut manifest) = load(&loc)? else {
        return Ok(None);
    };
    manifest.state.conflicts = unmerged(&loc.root)?;
    Ok(Some(manifest.state))
}

pub fn restore(path: &Path, stash: &str) -> Result<()> {
    if ![40, 64].contains(&stash.len()) || !stash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("올바른 Stash를 선택하세요.");
    }
    let loc = locations(path)?;
    plain_dir(&loc.area)?;
    let _lock = Lock::acquire(loc.area.join("operation.lock"))?;
    ensure_idle(&loc)?;
    if load(&loc)?.is_some() || !unmerged(&loc.root)?.is_empty() {
        bail!("진행 중인 복원이나 미해결 충돌을 먼저 처리하세요.");
    }
    if !String::from_utf8(run(&loc.root, &["stash", "list", "--format=%H"])?)?
        .lines()
        .any(|id| id.eq_ignore_ascii_case(stash))
    {
        bail!("선택한 Stash가 더 이상 없습니다.");
    }
    let parents = String::from_utf8(run(
        &loc.root,
        &["rev-list", "--parents", "-n", "1", stash],
    )?)?;
    let parents: Vec<_> = parents.split_whitespace().collect();
    if !(3..=4).contains(&parents.len()) {
        bail!("올바른 Stash 커밋이 아닙니다.");
    }
    let original = entries(&loc.root)?;
    let mut paths: BTreeSet<Vec<u8>> = original.iter().map(|entry| entry.path.clone()).collect();
    for tree in std::iter::once(stash).chain(parents.iter().skip(2).copied()) {
        for row in run(&loc.root, &["ls-tree", "-r", "-z", tree])?
            .split(|byte| *byte == 0)
            .filter(|row| !row.is_empty())
        {
            let split = row
                .iter()
                .position(|byte| *byte == b'\t')
                .context("Stash 경로를 읽지 못했습니다.")?;
            if !row.starts_with(b"100644 ") && !row.starts_with(b"100755 ") {
                bail!("링크/서브모듈 Stash는 안전한 복원 취소를 지원하지 않습니다.");
            }
            paths.insert(row[split + 1..].to_vec());
        }
    }
    let id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let dir = loc.area.join(&id);
    fs::create_dir(&dir)?;
    let index_path = loc.git_dir.join("index");
    let index_present = index_path.exists();
    if index_present {
        fs::copy(&index_path, dir.join("index"))?;
    } else {
        fs::write(dir.join("index"), [])?;
    }
    let mut manifest = Manifest {
        version: 1,
        state: StashRestoreState {
            root: loc.root.clone(),
            id,
            stash: stash.into(),
            head: head(&loc.root)?,
            current_branch: branch(&loc.root)?,
            revision: 0,
            applying: true,
            error: None,
            conflicts: Vec::new(),
        },
        index_present,
        index_hash: hash(&dir.join("index"))?,
        full_index_hash: None,
        tree: None,
        entries: original,
        files: Vec::new(),
        touched: BTreeSet::new(),
        missing_dirs: BTreeSet::new(),
    };
    for raw in paths {
        let path = safe_path(&loc.root, &raw)?;
        let mut ancestor = path.parent();
        while let Some(parent) = ancestor.filter(|parent| *parent != loc.root) {
            if !parent.exists() {
                manifest.missing_dirs.insert(
                    gix::path::into_bstr(parent.strip_prefix(&loc.root)?.to_owned()).to_vec(),
                );
            }
            ancestor = parent.parent();
        }
        let meta = fs::symlink_metadata(&path).ok();
        let readonly = meta
            .as_ref()
            .is_some_and(|meta| meta.permissions().readonly());
        #[cfg(unix)]
        let mode = meta.as_ref().map(|meta| {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode()
        });
        #[cfg(not(unix))]
        let mode = None;
        let index = manifest.files.len();
        let saved_hash = if meta.is_some() {
            fs::copy(&path, dir.join(index.to_string()))?;
            Some(hash(&dir.join(index.to_string()))?)
        } else {
            None
        };
        manifest.files.push(SavedFile {
            path: raw,
            hash: saved_hash,
            readonly,
            mode,
        });
    }
    if head(&loc.root)? != manifest.state.head
        || branch(&loc.root)? != manifest.state.current_branch
        || file_hash(&index_path)?
            != if index_present {
                Some(manifest.index_hash.clone())
            } else {
                None
            }
    {
        bail!("백업 중 저장소 상태가 변경되었습니다. 다시 시도하세요.");
    }
    for file in &manifest.files {
        if file_hash(&safe_path(&loc.root, &file.path)?)? != file.hash {
            bail!("백업 중 파일이 변경되었습니다. 다시 시도하세요.");
        }
    }
    if index_present {
        let full = dir.join("full-index");
        fs::copy(dir.join("index"), &full)?;
        let output = command(&loc.root)
            .env("GIT_INDEX_FILE", git_path(&full))
            .args(["update-index", "--no-split-index"])
            .output()?;
        if !output.status.success() {
            bail!("원본 인덱스를 보관하지 못했습니다.");
        }
        let output = command(&loc.root)
            .env("GIT_INDEX_FILE", git_path(&full))
            .arg("write-tree")
            .output()?;
        if !output.status.success() {
            bail!(
                "원본 Stage 내용을 보관하지 못했습니다: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let tree = String::from_utf8(output.stdout)?.trim().to_owned();
        run(
            &loc.root,
            &[
                "update-ref",
                &format!("refs/feather-stash-recovery/{}", manifest.state.id),
                &tree,
            ],
        )?;
        manifest.tree = Some(tree);
        manifest.full_index_hash = Some(hash(&full)?);
    }
    save(&loc, &manifest)?;
    let output = command(&loc.root)
        .args(["stash", "apply", "--index", stash])
        .output();
    let error = match output {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(format!(
            "Stash 복원 결과를 확인하세요. 원본 Stash는 유지됩니다: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )),
        Err(error) => Some(format!("Stash 복원을 실행하지 못했습니다: {error}")),
    };
    let current = entries(&loc.root)?;
    let before_entries = grouped(&manifest.entries);
    let after_entries = grouped(&current);
    for file in &manifest.files {
        if before_entries.get(file.path.as_slice()) != after_entries.get(file.path.as_slice())
            || file_hash(&safe_path(&loc.root, &file.path)?)? != file.hash
        {
            manifest.touched.insert(file.path.clone());
        }
    }
    manifest.state.applying = false;
    manifest.state.revision = 1;
    manifest.state.error = error.clone();
    save(&loc, &manifest)?;
    if let Some(error) = error {
        bail!("{error}");
    }
    retire(&loc, &manifest)
}

pub fn complete(path: &Path, expected: &StashRestoreState) -> Result<()> {
    control(path, expected, false)
}
pub fn cancel(path: &Path, expected: &StashRestoreState) -> Result<()> {
    control(path, expected, true)
}

fn control(path: &Path, expected: &StashRestoreState, cancel: bool) -> Result<()> {
    let loc = locations(path)?;
    plain_dir(&loc.area)?;
    let _operation = Lock::acquire(loc.area.join("operation.lock"))?;
    ensure_idle(&loc)?;
    let manifest = load(&loc)?.context("진행 중인 Stash 복원이 없습니다.")?;
    if manifest.state.id != expected.id
        || manifest.state.revision != expected.revision
        || manifest.state.root != expected.root
        || manifest.state.stash != expected.stash
        || manifest.state.head != expected.head
        || manifest.state.current_branch != expected.current_branch
        || manifest.state.applying
    {
        bail!("Stash 복원 세션이 변경되었거나 적용 중입니다. 새로고침하세요.");
    }
    if head(&loc.root)? != manifest.state.head
        || branch(&loc.root)? != manifest.state.current_branch
    {
        bail!("HEAD 또는 현재 브랜치가 바뀌었습니다. 복원 백업을 유지합니다.");
    }
    if !cancel {
        if !unmerged(&loc.root)?.is_empty() {
            bail!("충돌 파일을 수정한 뒤 Stage하세요.");
        }
        return retire(&loc, &manifest);
    }
    let index_lock = Lock::acquire(loc.git_dir.join("index.lock"))?;
    let outside = |entries: Vec<Entry>| -> Vec<Entry> {
        entries
            .into_iter()
            .filter(|entry| !manifest.touched.contains(&entry.path))
            .collect()
    };
    if outside(entries(&loc.root)?) != outside(manifest.entries.clone()) {
        bail!(
            "복원 범위 밖의 Stage 상태가 바뀌었습니다. 추가 작업을 보호하기 위해 취소하지 않습니다."
        );
    }
    let dir = job(&loc, &manifest.state.id)?;
    if hash(&dir.join("index"))? != manifest.index_hash {
        bail!("원본 인덱스 백업이 손상되었습니다.");
    }
    for parent in &manifest.missing_dirs {
        safe_directory(&loc.root, parent)?;
    }
    let mut original_index = dir.join("index");
    if let Some(tree) = &manifest.tree {
        if String::from_utf8(run(
            &loc.root,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/feather-stash-recovery/{}", manifest.state.id),
            ],
        )?)?
        .trim()
            != tree
        {
            bail!("원본 Stage 보관 참조가 변경되었습니다.");
        }
        if manifest.full_index_hash.as_ref() != Some(&hash(&dir.join("full-index"))?) {
            bail!("원본 전체 인덱스 백업이 손상되었습니다.");
        }
    }
    if manifest.index_present
        && !command(&loc.root)
            .env("GIT_INDEX_FILE", git_path(&dir.join("index")))
            .args(["ls-files", "--stage", "-z"])
            .output()?
            .status
            .success()
    {
        original_index = dir.join("full-index");
        if manifest.full_index_hash.is_none()
            || !command(&loc.root)
                .env("GIT_INDEX_FILE", git_path(&original_index))
                .args(["ls-files", "--stage", "-z"])
                .output()?
                .status
                .success()
        {
            bail!("원본 인덱스 백업이나 공유 인덱스를 읽을 수 없습니다.");
        }
    }
    for (index, file) in manifest
        .files
        .iter()
        .enumerate()
        .filter(|(_, file)| manifest.touched.contains(&file.path))
    {
        safe_path(&loc.root, &file.path)?;
        if let Some(expected) = &file.hash
            && hash(&dir.join(index.to_string()))? != *expected
        {
            bail!("원본 파일 백업이 손상되었습니다.");
        }
    }
    for (index, file) in manifest
        .files
        .iter()
        .enumerate()
        .filter(|(_, file)| manifest.touched.contains(&file.path))
    {
        let path = safe_path(&loc.root, &file.path)?;
        if file.hash.is_some() {
            let parent = path.parent().context("복원 경로를 읽지 못했습니다.")?;
            let mut checked = loc.root.clone();
            for part in parent.strip_prefix(&loc.root)?.components() {
                checked.push(part.as_os_str());
                plain_dir(&checked)?;
            }
            let temp = parent.join(format!(".feather-{}-{index}", manifest.state.id));
            let mut guard = Lock::acquire(temp.clone())?;
            std::io::copy(
                &mut fs::File::open(dir.join(index.to_string()))?,
                &mut guard._file,
            )?;
            guard._file.sync_all()?;
            let mut permissions = fs::metadata(&temp)?.permissions();
            permissions.set_readonly(file.readonly);
            #[cfg(unix)]
            if let Some(mode) = file.mode {
                use std::os::unix::fs::PermissionsExt;
                permissions.set_mode(mode);
            }
            fs::set_permissions(&temp, permissions)?;
            fs::rename(&temp, &path)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
    }
    if manifest.index_present {
        fs::copy(original_index, &index_lock.path)?;
        index_lock._file.sync_all()?;
        fs::rename(&index_lock.path, loc.git_dir.join("index"))?;
    } else if loc.git_dir.join("index").exists() {
        fs::remove_file(loc.git_dir.join("index"))?;
    }
    let mut parents: Vec<_> = manifest.missing_dirs.iter().collect();
    parents.sort_by_key(|path| std::cmp::Reverse(path.len()));
    for parent in parents {
        let path = safe_directory(&loc.root, parent)?;
        {
            if let Ok(meta) = fs::symlink_metadata(&path)
                && meta.is_dir()
                && !linked(&meta)
            {
                let _ = fs::remove_dir(path);
            }
        }
    }
    retire(&loc, &manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(root: &Path, args: &[&str]) -> Vec<u8> {
        run(root, args).unwrap()
    }
    fn fixture() -> (tempfile::TempDir, String) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-b", "main"]);
        for (key, value) in [
            ("user.name", "Test"),
            ("user.email", "test@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
        ] {
            git(root, &["config", key, value]);
        }
        for name in [
            "conflict.txt",
            "prior.txt",
            "other.txt",
            "delete.txt",
            "한글.txt",
        ] {
            fs::write(root.join(name), "base\n").unwrap();
        }
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "base"]);
        fs::write(root.join("conflict.txt"), "stashed\n").unwrap();
        fs::remove_file(root.join("delete.txt")).unwrap();
        fs::write(root.join("한글.txt"), [0, 1, 2, 3]).unwrap();
        fs::create_dir(root.join("new-dir")).unwrap();
        fs::write(root.join("new-dir/new.txt"), "from stash\n").unwrap();
        git(
            root,
            &["stash", "push", "--include-untracked", "-m", "saved"],
        );
        let stash = String::from_utf8(git(root, &["rev-parse", "refs/stash"]))
            .unwrap()
            .trim()
            .to_owned();
        fs::write(root.join("conflict.txt"), "current\n").unwrap();
        git(root, &["commit", "-am", "current"]);
        fs::write(root.join("prior.txt"), "prior stage\n").unwrap();
        git(root, &["add", "prior.txt"]);
        fs::write(root.join("prior.txt"), "prior worktree\n").unwrap();
        fs::write(root.join("local-untracked.txt"), "local\n").unwrap();
        (temp, stash)
    }

    #[test]
    fn successful_restore_keeps_stash_and_removes_recovery_state() {
        let (temp, stash) = fixture();
        let root = temp.path();
        git(root, &["reset", "--hard", &format!("{stash}^1")]);
        restore(root, &stash).unwrap();
        assert!(state(root).unwrap().is_none());
        assert_eq!(fs::read(root.join("conflict.txt")).unwrap(), b"stashed\n");
        assert_eq!(
            fs::read(root.join("new-dir/new.txt")).unwrap(),
            b"from stash\n"
        );
        assert_eq!(
            git(root, &["rev-parse", "refs/stash"]),
            format!("{stash}\n").as_bytes()
        );
        assert!(git(root, &["for-each-ref", "refs/feather-stash-recovery/"]).is_empty());
    }

    #[test]
    fn failed_apply_without_unmerged_entries_can_be_cancelled() {
        let (temp, stash) = fixture();
        let root = temp.path();
        fs::write(root.join("conflict.txt"), "local edit\n").unwrap();
        assert!(restore(root, &stash).is_err());
        let pending = state(root).unwrap().unwrap();
        assert!(pending.conflicts.is_empty());
        assert!(pending.error.is_some());
        cancel(root, &pending).unwrap();
        assert_eq!(
            fs::read(root.join("conflict.txt")).unwrap(),
            b"local edit\n"
        );
        assert!(state(root).unwrap().is_none());
    }

    #[test]
    fn recovery_state_is_isolated_from_linked_worktree() {
        let (temp, stash) = fixture();
        let root = temp.path();
        git(
            root,
            &["worktree", "add", "-b", "linked", "linked-tree", "HEAD"],
        );
        assert!(restore(root, &stash).is_err());
        assert!(state(root).unwrap().is_some());
        assert!(state(&root.join("linked-tree")).unwrap().is_none());
        let pending = state(root).unwrap().unwrap();
        cancel(root, &pending).unwrap();
    }

    #[test]
    fn conflict_completion_keeps_resolution_index_and_original_stash() {
        let (temp, stash) = fixture();
        let root = temp.path();
        let original_head = head(root).unwrap();
        let error = restore(root, &stash).unwrap_err();
        assert!(state(root).unwrap().is_some(), "{error:#}");
        let pending = state(root).unwrap().unwrap();
        assert!(!pending.applying);
        assert_eq!(pending.conflicts, vec![b"conflict.txt".to_vec()]);
        assert!(complete(root, &pending).is_err());
        fs::write(root.join("conflict.txt"), "resolved\n").unwrap();
        git(root, &["add", "conflict.txt"]);
        complete(root, &pending).unwrap();
        assert!(state(root).unwrap().is_none());
        assert_eq!(head(root).unwrap(), original_head);
        assert_eq!(git(root, &["show", ":conflict.txt"]), b"resolved\n");
        assert_eq!(
            String::from_utf8(git(root, &["rev-parse", "refs/stash"]))
                .unwrap()
                .trim(),
            stash
        );
        assert_eq!(
            fs::read(root.join("local-untracked.txt")).unwrap(),
            b"local\n"
        );
    }

    #[test]
    fn cancellation_restores_original_index_worktree_untracked_and_deleted_files() {
        let (temp, stash) = fixture();
        let root = temp.path();
        let index = fs::read(root.join(".git/index")).unwrap();
        assert!(restore(root, &stash).is_err());
        let pending = state(root).unwrap().unwrap();
        fs::write(root.join("other.txt"), "later outside edit\n").unwrap();
        fs::write(root.join("conflict.txt"), "discard this resolution\n").unwrap();
        git(root, &["add", "conflict.txt"]);
        cancel(root, &pending).unwrap();
        assert!(state(root).unwrap().is_none());
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(root.join("conflict.txt")).unwrap(), b"current\n");
        assert_eq!(
            fs::read(root.join("prior.txt")).unwrap(),
            b"prior worktree\n"
        );
        assert_eq!(git(root, &["show", ":prior.txt"]), b"prior stage\n");
        assert_eq!(
            fs::read(root.join("other.txt")).unwrap(),
            b"later outside edit\n"
        );
        assert_eq!(fs::read(root.join("delete.txt")).unwrap(), b"base\n");
        assert_eq!(fs::read(root.join("한글.txt")).unwrap(), b"base\n");
        assert_eq!(
            fs::read(root.join("local-untracked.txt")).unwrap(),
            b"local\n"
        );
        assert!(!root.join("new-dir/new.txt").exists());
        assert_eq!(
            String::from_utf8(git(root, &["rev-parse", "refs/stash"]))
                .unwrap()
                .trim(),
            stash
        );
    }

    #[test]
    fn cancel_rejects_outside_stage_head_changes_and_old_session_without_writes() {
        let (temp, stash) = fixture();
        let root = temp.path();
        assert!(restore(root, &stash).is_err());
        let pending = state(root).unwrap().unwrap();
        let mut stale = pending.clone();
        stale.revision += 1;
        assert!(cancel(root, &stale).is_err());
        fs::write(root.join("other.txt"), "new stage\n").unwrap();
        git(root, &["add", "other.txt"]);
        let index = fs::read(root.join(".git/index")).unwrap();
        let conflict = fs::read(root.join("conflict.txt")).unwrap();
        assert!(cancel(root, &pending).is_err());
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(root.join("conflict.txt")).unwrap(), conflict);
        let current = head(root).unwrap();
        let parent = String::from_utf8(git(root, &["rev-parse", "HEAD^"]))
            .unwrap()
            .trim()
            .to_owned();
        git(root, &["update-ref", "refs/heads/main", &parent]);
        assert!(cancel(root, &pending).is_err());
        assert!(complete(root, &pending).is_err());
        git(root, &["update-ref", "refs/heads/main", &current]);
        assert!(state(root).unwrap().is_some());
    }

    #[test]
    fn corrupted_metadata_paths_and_interrupted_apply_never_restore_files() {
        let (temp, stash) = fixture();
        let root = temp.path();
        assert!(restore(root, &stash).is_err());
        let pending = state(root).unwrap().unwrap();
        let loc = locations(root).unwrap();
        let mut manifest = load(&loc).unwrap().unwrap();
        let index = fs::read(root.join(".git/index")).unwrap();
        let conflict = fs::read(root.join("conflict.txt")).unwrap();
        manifest.state.applying = true;
        save(&loc, &manifest).unwrap();
        assert!(cancel(root, &pending).is_err());
        manifest.state.applying = false;
        manifest.missing_dirs.insert(b"../outside".to_vec());
        save(&loc, &manifest).unwrap();
        assert!(state(root).is_err());
        assert!(cancel(root, &pending).is_err());
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(root.join("conflict.txt")).unwrap(), conflict);
    }

    #[test]
    fn damaged_backup_and_index_lock_preserve_data_and_recovery_state() {
        let (temp, stash) = fixture();
        let root = temp.path();
        assert!(restore(root, &stash).is_err());
        let pending = state(root).unwrap().unwrap();
        let loc = locations(root).unwrap();
        let manifest = load(&loc).unwrap().unwrap();
        let dir = job(&loc, &pending.id).unwrap();
        let file = manifest
            .files
            .iter()
            .position(|file| file.path == b"conflict.txt")
            .unwrap();
        let original = fs::read(dir.join(file.to_string())).unwrap();
        let index = fs::read(root.join(".git/index")).unwrap();
        fs::write(dir.join(file.to_string()), "corrupted").unwrap();
        assert!(cancel(root, &pending).is_err());
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        fs::write(dir.join(file.to_string()), original).unwrap();
        let _index_lock = Lock::acquire(root.join(".git/index.lock")).unwrap();
        assert!(cancel(root, &pending).is_err());
        assert!(state(root).unwrap().is_some());
    }
}
