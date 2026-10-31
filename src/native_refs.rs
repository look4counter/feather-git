use crate::native_local as local;
use anyhow::{Context, Result, bail};
use gix::{bstr::ByteSlice, refs::transaction::PreviousValue};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn remotes(path: &Path) -> Result<Vec<String>> {
    let repo = local::repo(path)?;
    let config = repo.config_snapshot();
    let mut names = BTreeSet::new();
    if let Some(sections) = config.plumbing().sections_by_name("remote") {
        for section in sections {
            if section.value_names().next().is_none() {
                continue;
            }
            if let Some(name) = section.header().subsection_name() {
                names.insert(name.to_str()?.to_owned());
            }
        }
    }
    Ok(names.into_iter().collect())
}
pub fn upstream(path: &Path, branch: &str) -> Result<String> {
    let env = BTreeMap::new();
    let remote = local::config_values(path, None, &format!("branch.{branch}.remote"), &env)?.pop();
    let merge = local::config_values(path, None, &format!("branch.{branch}.merge"), &env)?.pop();
    let (Some(remote), Some(merge)) = (remote, merge) else {
        return Ok(String::new());
    };
    if remote == "." {
        return Ok(merge);
    }
    for spec in local::config_values(path, None, &format!("remote.{remote}.fetch"), &env)? {
        if let Some((from, to)) = spec.trim_start_matches('+').split_once(':') {
            if let Some((pre, post)) = from.split_once('*') {
                if let Some(mid) = merge.strip_prefix(pre).and_then(|s| s.strip_suffix(post)) {
                    return Ok(to.replace('*', mid));
                }
            } else if from == merge {
                return Ok(to.to_owned());
            }
        }
    }
    Ok(String::new())
}
pub fn remote_request(path: &Path, a: &[&str]) -> Result<Vec<u8>> {
    if a.get(1) == Some(&"remove") {
        let name = a.get(2).context("삭제할 원격 이름이 없습니다.")?;
        git2::Repository::open(path)?.remote_delete(name)?;
        return Ok(Vec::new());
    }
    if a.len() == 1 {
        return Ok(remotes(path)?.join("\n").into_bytes());
    }
    if a.get(1) == Some(&"get-url") {
        let name = a.last().unwrap();
        let key = if a.contains(&"--push") {
            "pushurl"
        } else {
            "url"
        };
        let mut urls = local::config_values(
            path,
            None,
            &format!("remote.{name}.{key}"),
            &BTreeMap::new(),
        )?;
        if urls.is_empty() && key == "pushurl" {
            urls =
                local::config_values(path, None, &format!("remote.{name}.url"), &BTreeMap::new())?;
        }
        if urls.is_empty() {
            bail!("원격 주소가 없습니다.");
        }
        return Ok(urls.join("\n").into_bytes());
    }
    bail!("지원하지 않는 원격 설정 요청입니다.")
}
pub fn list(path: &Path, a: &[&str]) -> Result<Vec<u8>> {
    let repo = local::repo(path)?;
    let format = a
        .iter()
        .find_map(|s| s.strip_prefix("--format="))
        .unwrap_or("%(refname)");
    let prefixes = a
        .iter()
        .filter(|s| s.starts_with("refs/"))
        .copied()
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for reference in repo.references()?.all()? {
        let mut reference = reference.map_err(anyhow::Error::from_boxed)?;
        let name = reference.name().as_bstr().to_str()?.to_owned();
        if !prefixes.is_empty() && !prefixes.iter().any(|pre| name.starts_with(pre)) {
            continue;
        }
        let symbolic = reference
            .target()
            .try_name()
            .map(|n| n.as_bstr().to_string())
            .unwrap_or_default();
        let id = match reference.target().try_id() {
            Some(id) => id.to_owned(),
            None => reference.peel_to_id()?.detach(),
        };
        let object = repo.find_object(id)?;
        let kind = match object.kind {
            gix::objs::Kind::Commit => "commit",
            gix::objs::Kind::Tag => "tag",
            gix::objs::Kind::Tree => "tree",
            _ => "blob",
        };
        let upstream = if let Some(branch) = name.strip_prefix("refs/heads/") {
            upstream(path, branch)?
        } else {
            String::new()
        };
        rows.push((
            name.clone(),
            format
                .replace("%09", "\t")
                .replace("%(refname)", &name)
                .replace("%(objectname)", &id.to_string())
                .replace("%(symref)", &symbolic)
                .replace("%(objecttype)", kind)
                .replace("%(upstream)", &upstream),
        ));
    }
    rows.sort();
    Ok(rows
        .into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes())
}
pub fn update(path: &Path, a: &[&str]) -> Result<Vec<u8>> {
    let repo = local::repo(path)?;
    if a.get(1) == Some(&"-d") {
        let reference = repo.find_reference(a[2])?;
        if let Some(expected) = a.get(3)
            && reference.id().detach() != gix::ObjectId::from_hex(expected.as_bytes())?
        {
            bail!("참조가 변경되었습니다.");
        }
        reference.delete()?;
    } else {
        let expected = if let Some(old) = a.get(3) {
            PreviousValue::MustExistAndMatch(gix::refs::Target::Object(local::oid(&repo, old)?))
        } else {
            PreviousValue::Any
        };
        repo.reference(
            a[1],
            local::oid(&repo, a[2])?,
            expected,
            "feather-git: update",
        )?;
    }
    Ok(Vec::new())
}
pub fn branch(path: &Path, a: &[&str]) -> Result<Vec<u8>> {
    let repo = local::repo(path)?;
    let pos = a
        .iter()
        .position(|s| *s == "--")
        .map(|i| i + 1)
        .unwrap_or(1);
    let name = a.get(pos).context("브랜치 이름이 없습니다.")?;
    if let Some(up) = a.iter().find_map(|s| s.strip_prefix("--set-upstream-to=")) {
        let mut mapping = None;
        for remote in remotes(path)? {
            for spec in local::config_values(
                path,
                None,
                &format!("remote.{remote}.fetch"),
                &BTreeMap::new(),
            )? {
                if let Some((from, to)) = spec.trim_start_matches('+').split_once(':') {
                    let source = if let Some((pre, post)) = to.split_once('*') {
                        up.strip_prefix(pre)
                            .and_then(|v| v.strip_suffix(post))
                            .map(|v| from.replace('*', v))
                    } else {
                        (to == up).then(|| from.to_owned())
                    };
                    if let Some(source) = source {
                        mapping = Some((remote.clone(), source));
                    }
                }
            }
        }
        let (remote, merge) = mapping.context("원격 추적 매핑을 찾을 수 없습니다.")?;
        local::config_set(
            path,
            false,
            &format!("branch.{name}.remote"),
            &[remote],
            &BTreeMap::new(),
        )?;
        local::config_set(
            path,
            false,
            &format!("branch.{name}.merge"),
            &[merge],
            &BTreeMap::new(),
        )?;
    } else if a.contains(&"-d") {
        let reference = if a.contains(&"-r") {
            format!("refs/remotes/{name}")
        } else {
            format!("refs/heads/{name}")
        };
        if !a.contains(&"-r") {
            if String::from_utf8(worktrees(path)?)?
                .lines()
                .any(|s| s == format!("branch {reference}"))
            {
                bail!("사용 중인 브랜치입니다.");
            }
            let target = upstream(path, name)?;
            let target = if target.is_empty() { "HEAD" } else { &target };
            let id = local::oid(&repo, &reference)?;
            let tip = local::oid(&repo, target).or_else(|_| local::oid(&repo, "HEAD"))?;
            if repo.merge_base(id, tip)?.detach() != id {
                bail!("병합되지 않은 브랜치는 삭제하지 않습니다.");
            }
        }
        repo.find_reference(reference.as_str())?.delete()?;
        if !a.contains(&"-r") {
            for key in ["remote", "merge"] {
                local::config_set(
                    path,
                    false,
                    &format!("branch.{name}.{key}"),
                    &[],
                    &BTreeMap::new(),
                )?;
            }
        }
    } else {
        repo.reference(
            format!("refs/heads/{name}"),
            local::oid(&repo, a.get(pos + 1).unwrap_or(&"HEAD"))?,
            PreviousValue::MustNotExist,
            "branch: Created by feather-git",
        )?;
    }
    Ok(Vec::new())
}
pub fn tag(path: &Path, a: &[&str]) -> Result<Vec<u8>> {
    let repo = local::repo(path)?;
    let pos = a
        .iter()
        .position(|s| *s == "--")
        .map(|i| i + 1)
        .unwrap_or(2);
    let name = a.get(pos).context("태그 이름이 없습니다.")?;
    if a.contains(&"-d") {
        repo.find_reference(format!("refs/tags/{name}").as_str())?
            .delete()?;
    } else {
        repo.tag_reference(
            name,
            local::oid(&repo, a.get(pos + 1).unwrap_or(&"HEAD"))?,
            PreviousValue::MustNotExist,
        )?;
    }
    Ok(Vec::new())
}
pub fn worktrees(path: &Path) -> Result<Vec<u8>> {
    let repo = local::repo(path)?;
    let mut dirs = vec![repo.common_dir().to_owned()];
    if let Ok(entries) = std::fs::read_dir(repo.common_dir().join("worktrees")) {
        for entry in entries {
            dirs.push(entry?.path());
        }
    }
    let mut out = String::new();
    for dir in dirs {
        if let Ok(head) = std::fs::read_to_string(dir.join("HEAD"))
            && let Some(name) = head.trim().strip_prefix("ref: ")
        {
            out.push_str(&format!("branch {name}\n"));
        }
    }
    Ok(out.into_bytes())
}
