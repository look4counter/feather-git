use gpui_http_client::{AsyncBody, HttpClient, Request, Response};
use sha2::{Digest, Sha256};
use std::{io::Read, sync::mpsc, time::Duration};

const MAX_BYTES: usize = 256 * 1024;
type Reply = anyhow::Result<Response<AsyncBody>>;
type Job = (String, futures::channel::oneshot::Sender<Reply>);

pub fn url(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return None;
    }
    Some(format!(
        "https://www.gravatar.com/avatar/{:x}?s=80&d=404&r=g",
        Sha256::digest(email.as_bytes())
    ))
}

fn repository_path(path: &str) -> Option<String> {
    let path = path.trim_matches('/');
    let parts: Vec<_> = path.split('/').collect();
    (parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && !matches!(*part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        }))
    .then(|| path.to_owned())
}

pub fn repository_from_remote(remote: &str) -> Option<String> {
    let normalized = remote
        .strip_prefix("git@github.com:")
        .map(|path| format!("ssh://git@github.com/{path}"));
    let remote = url::Url::parse(normalized.as_deref().unwrap_or(remote)).ok()?;
    if remote.host_str()? != "github.com"
        || !matches!(remote.scheme(), "https" | "http" | "ssh" | "git")
    {
        return None;
    }
    let path = remote.path().trim_matches('/');
    repository_path(path.strip_suffix(".git").unwrap_or(path))
}

pub fn web_repository_from_remote(remote: &str) -> Option<String> {
    let scp = remote
        .split_once(':')
        .filter(|(address, _)| address.contains('@') && !address.contains('/'))
        .map(|(address, path)| format!("ssh://{address}/{path}"));
    let mut remote = url::Url::parse(scp.as_deref().unwrap_or(remote)).ok()?;
    if remote.scheme() == "ssh" {
        let mut web = url::Url::parse("https://localhost/").ok()?;
        web.set_host(remote.host_str()).ok()?;
        web.set_path(remote.path());
        remote = web;
    }
    if !matches!(remote.scheme(), "https" | "http") || remote.host_str()? == "github.com" {
        return None;
    }
    remote.set_username("").ok()?;
    remote.set_password(None).ok()?;
    remote.set_query(None);
    remote.set_fragment(None);
    let path = remote.path().trim_matches('/');
    if path.split('/').count() < 2
        || !path.split('/').all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte))
        })
    {
        return None;
    }
    let path = format!("/{path}");
    remote.set_path(&path);
    Some(remote.into())
}

struct HostedRequest {
    repository: url::Url,
    sha: String,
    role: usize,
}

fn hosted_request(uri: &str) -> Option<HostedRequest> {
    let uri = url::Url::parse(uri).ok()?;
    if uri.origin()
        != url::Url::parse("https://avatar.feather.invalid/")
            .ok()?
            .origin()
    {
        return None;
    }
    let (sha, role) = uri.path().strip_prefix("/hosted/")?.split_once('/')?;
    if !valid_sha(sha) {
        return None;
    }
    let role = match role {
        "author" => 0,
        "committer" => 1,
        _ => return None,
    };
    let source = uri.query_pairs().find(|(key, _)| key == "repo")?.1;
    let repository = web_repository_from_remote(&source)?.parse().ok()?;
    Some(HostedRequest {
        repository,
        sha: sha.to_owned(),
        role,
    })
}

impl HostedRequest {
    fn endpoints(&self) -> Option<[(url::Url, Option<String>); 2]> {
        let (base_and_owner, repo) = self.repository.path().rsplit_once('/')?;
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        if matches!(repo, "" | "." | "..") {
            return None;
        }
        let (base, owner) = base_and_owner.rsplit_once('/')?;
        let mut gitea = self.repository.clone();
        gitea.set_path(&format!(
            "{base}/api/v1/repos/{owner}/{repo}/git/commits/{}",
            self.sha
        ));
        gitea.set_query(Some("stat=false&verification=false&files=false"));
        let base = base.strip_suffix("/scm").unwrap_or(base);
        let mut bitbucket = self.repository.clone();
        bitbucket.set_path(&format!(
            "{base}/rest/api/1.0/projects/{owner}/repos/{repo}/commits/{}",
            self.sha
        ));
        Some([(gitea, None), (bitbucket, Some(base.to_owned()))])
    }

    fn parse_profiles(
        &self,
        bytes: &[u8],
        bitbucket_base: Option<&str>,
        mut gitea_profile: impl FnMut(&url::Url) -> Option<String>,
    ) -> anyhow::Result<GitHubProfiles> {
        let data: serde_json::Value = serde_json::from_slice(bytes)?;
        let id = if bitbucket_base.is_some() {
            "id"
        } else {
            "sha"
        };
        anyhow::ensure!(
            data[id]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(&self.sha))
        );
        Ok(["author", "committer"].map(|role| {
            let user = &data[role];
            let image = if let Some(base) = bitbucket_base {
                let slug = user["slug"].as_str()?;
                if matches!(slug, "" | "." | "..") {
                    return None;
                }
                let mut image = self.repository.clone();
                image.set_path(&format!("{base}/users"));
                image.path_segments_mut().ok()?.extend([slug, "avatar.png"]);
                image.set_query(Some("s=80"));
                image
            } else {
                return user["avatar_url"]
                    .as_str()
                    .and_then(|image| self.same_origin_avatar(image))
                    .or_else(|| {
                        let login = user["login"]
                            .as_str()
                            .or_else(|| user["username"].as_str())?;
                        if login.is_empty() || matches!(login, "." | "..") {
                            return None;
                        }
                        let mut profile = self.repository.join("../").ok()?;
                        profile
                            .path_segments_mut()
                            .ok()?
                            .pop_if_empty()
                            .extend(["api", "v1", "users", login]);
                        gitea_profile(&profile)
                    });
            };
            Some(image.into())
        }))
    }

    fn same_origin_avatar(&self, path: &str) -> Option<String> {
        if path.trim().is_empty() {
            return None;
        }
        let image = self.repository.join(path).ok()?;
        (image.origin() == self.repository.origin()
            && image.username().is_empty()
            && image.password().is_none())
        .then(|| image.into())
    }
}

pub fn commit_url(
    repository: Option<&str>,
    sha: &str,
    committer: bool,
    email: &str,
) -> Option<String> {
    let role = if committer { "committer" } else { "author" };
    if let Some(repository) = repository.and_then(web_repository_from_remote)
        && valid_sha(sha)
    {
        let mut uri = url::Url::parse(&format!(
            "https://avatar.feather.invalid/hosted/{sha}/{role}"
        ))
        .ok()?;
        uri.query_pairs_mut().append_pair("repo", &repository);
        return Some(uri.into());
    }
    let fallback = url(email);
    let Some(repository) = repository.and_then(repository_path) else {
        return fallback;
    };
    if !valid_sha(sha) {
        return fallback;
    }
    let mut uri = format!("https://avatar.feather.invalid/github/{repository}/{sha}/{role}");
    if !email.trim().is_empty() {
        uri.push_str(&format!(
            "?email={:x}",
            Sha256::digest(email.trim().to_lowercase().as_bytes())
        ));
    }
    Some(uri)
}

fn valid_sha(sha: &str) -> bool {
    matches!(sha.len(), 40 | 64) && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
}

struct GitHubRequest {
    api: String,
    role: usize,
    profile_key: Option<String>,
    fallback: Option<String>,
}

fn github_request(uri: &str) -> Option<GitHubRequest> {
    let uri = url::Url::parse(uri).ok()?;
    if uri.scheme() != "https" || uri.host_str()? != "avatar.feather.invalid" {
        return None;
    }
    let parts: Vec<_> = uri.path_segments()?.collect();
    if parts.len() != 5 || parts[0] != "github" || !valid_sha(parts[3]) {
        return None;
    }
    let repository = repository_path(&format!("{}/{}", parts[1], parts[2]))?;
    let role = match parts[4] {
        "author" => 0,
        "committer" => 1,
        _ => return None,
    };
    let profile_key = uri
        .query_pairs()
        .find(|(key, _)| key == "email")
        .map(|(_, hash)| hash.into_owned())
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let fallback = profile_key
        .as_ref()
        .map(|hash| format!("https://www.gravatar.com/avatar/{hash}?s=80&d=404&r=g"));
    Some(GitHubRequest {
        api: format!(
            "https://api.github.com/repos/{repository}/commits/{}?per_page=1",
            parts[3]
        ),
        role,
        profile_key,
        fallback,
    })
}

fn allowed_image(uri: &str) -> bool {
    url::Url::parse(uri).is_ok_and(|uri| {
        if uri.scheme() != "https"
            || uri.port_or_known_default() != Some(443)
            || !uri.username().is_empty()
            || uri.password().is_some()
        {
            return false;
        }
        match uri.host_str() {
            Some("www.gravatar.com") => uri.path().strip_prefix("/avatar/").is_some_and(|hash| {
                hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            }),
            Some("avatars.githubusercontent.com") => uri.path().starts_with("/u/"),
            _ => false,
        }
    })
}

fn allowed(uri: &str) -> bool {
    allowed_image(uri) || github_request(uri).is_some() || hosted_request(uri).is_some()
}

type GitHubProfiles = [Option<String>; 2];

fn parse_profiles(bytes: &[u8]) -> anyhow::Result<GitHubProfiles> {
    let data: serde_json::Value = serde_json::from_slice(bytes)?;
    anyhow::ensure!(
        data.is_object() && data.get("author").is_some() && data.get("committer").is_some(),
        "GitHub 프로필 응답이 올바르지 않습니다."
    );
    Ok(["author", "committer"].map(|role| {
        data[role]["avatar_url"]
            .as_str()
            .filter(|uri| allowed_image(uri))
            .map(str::to_owned)
    }))
}

#[derive(Default)]
struct SessionCache {
    commits: std::collections::HashMap<String, Option<GitHubProfiles>>,
    profiles: std::collections::HashMap<String, Option<String>>,
    images: std::collections::HashMap<String, Option<Vec<u8>>>,
}

impl SessionCache {
    fn image(
        &mut self,
        uri: &str,
        fetch: &mut impl FnMut(&str) -> anyhow::Result<Vec<u8>>,
    ) -> Option<Vec<u8>> {
        self.images
            .entry(uri.to_owned())
            .or_insert_with(|| fetch(uri).ok())
            .clone()
    }
    fn resolve(
        &mut self,
        uri: &str,
        mut fetch: impl FnMut(&str) -> anyhow::Result<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        if let Some(request) = github_request(uri) {
            let profile = if let Some(cached) = request
                .profile_key
                .as_ref()
                .and_then(|key| self.profiles.get(key))
            {
                cached.clone()
            } else {
                let profiles = self.commits.entry(request.api.clone()).or_insert_with(|| {
                    fetch(&request.api)
                        .and_then(|bytes| parse_profiles(&bytes))
                        .ok()
                });
                let profile = profiles
                    .as_ref()
                    .and_then(|profiles| profiles[request.role].clone());
                if let Some(key) = request.profile_key {
                    self.profiles.insert(key, profile.clone());
                }
                profile
            };
            if let Some(uri) = profile
                && let Some(bytes) = self.image(&uri, &mut fetch)
            {
                return Ok(bytes);
            }
            if let Some(uri) = request.fallback
                && let Some(bytes) = self.image(&uri, &mut fetch)
            {
                return Ok(bytes);
            }
        } else if let Some(request) = hosted_request(uri) {
            if let Some(endpoints) = request.endpoints() {
                for (api, bitbucket_base) in endpoints {
                    let profiles = self.commits.entry(api.to_string()).or_insert_with(|| {
                        fetch(api.as_str())
                            .and_then(|bytes| {
                                request.parse_profiles(
                                    &bytes,
                                    bitbucket_base.as_deref(),
                                    |profile| {
                                        self.profiles
                                            .entry(profile.to_string())
                                            .or_insert_with(|| {
                                                let response = fetch(profile.as_str()).ok()?;
                                                let user: serde_json::Value =
                                                    serde_json::from_slice(&response).ok()?;
                                                request.same_origin_avatar(
                                                    user["avatar_url"].as_str()?,
                                                )
                                            })
                                            .clone()
                                    },
                                )
                            })
                            .ok()
                    });
                    if let Some(image) = profiles
                        .as_ref()
                        .and_then(|profiles| profiles[request.role].clone())
                        && let Some(bytes) = self.image(&image, &mut fetch)
                    {
                        return Ok(bytes);
                    }
                }
            }
        } else if allowed_image(uri)
            && let Some(bytes) = self.image(uri, &mut fetch)
        {
            return Ok(bytes);
        }
        anyhow::bail!("프로필 사진을 조회하지 못했습니다.")
    }
}

fn bounded_bytes(reader: impl Read) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() <= MAX_BYTES,
        "사진 응답이 제한 크기를 초과했습니다."
    );
    Ok(bytes)
}

pub struct AvatarClient {
    sender: mpsc::SyncSender<Job>,
}

impl AvatarClient {
    pub fn new() -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(8);
        std::thread::Builder::new()
            .name("avatar-images".into())
            .spawn(move || {
                let client = avatar_http::blocking::Client::builder()
                    .timeout(Duration::from_secs(3))
                    .redirect(avatar_http::redirect::Policy::custom(|attempt| {
                        let original = attempt.previous().first();
                        let public = original.is_some_and(|url| {
                            url.host_str() == Some("api.github.com") || allowed_image(url.as_str())
                        });
                        if attempt.previous().len() > 3 {
                            attempt.error("too many avatar redirects")
                        } else if public
                            || original.is_some_and(|url| url.origin() == attempt.url().origin())
                        {
                            attempt.follow()
                        } else {
                            attempt.error("cross-origin avatar redirect")
                        }
                    }))
                    .user_agent("FeatherGit/1.0")
                    .build();
                let mut cache = SessionCache::default();
                while let Ok((uri, reply)) = receiver.recv() {
                    if reply.is_canceled() {
                        continue;
                    }
                    let result = (|| {
                        let client = client
                            .as_ref()
                            .map_err(|_| anyhow::anyhow!("사진 HTTP 초기화 실패"))?;
                        let bytes = cache.resolve(&uri, |uri| {
                            let request = client.get(uri);
                            let request = if uri.starts_with("https://api.github.com/") {
                                request.header("Accept", "application/vnd.github+json")
                            } else {
                                request
                            };
                            let response = request
                                .send()
                                .and_then(|response| response.error_for_status())
                                .map_err(|_| anyhow::anyhow!("사진 조회 실패"))?;
                            bounded_bytes(response)
                        })?;
                        Ok(Response::builder()
                            .status(200)
                            .body(AsyncBody::from(bytes))?)
                    })();
                    let _ = reply.send(result);
                }
            })?;
        Ok(Self { sender })
    }
}

impl HttpClient for AvatarClient {
    fn type_name(&self) -> &'static str {
        "AvatarClient"
    }
    fn user_agent(&self) -> Option<&gpui_http_client::http::HeaderValue> {
        None
    }
    fn proxy(&self) -> Option<&url::Url> {
        None
    }
    fn send(&self, request: Request<AsyncBody>) -> futures::future::BoxFuture<'static, Reply> {
        let (reply, result) = futures::channel::oneshot::channel();
        let uri = request.uri().to_string();
        let accepted = request.method() == gpui_http_client::Method::GET && allowed(&uri);
        let queued = accepted && self.sender.try_send((uri, reply)).is_ok();
        Box::pin(async move {
            anyhow::ensure!(queued, "사진 요청을 처리할 수 없습니다.");
            result
                .await
                .map_err(|_| anyhow::anyhow!("사진 작업이 종료되었습니다."))?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gitea_user_avatar_is_reused_across_commits() {
        let repository = "https://git.example.test:8443/dev/team/repo.git";
        let profile = "https://git.example.test:8443/dev/api/v1/users/writer";
        let image = "https://git.example.test:8443/avatars/photo?size=512";
        let mut cache = SessionCache::default();
        let mut requests = Vec::new();
        for sha in ["a".repeat(40), "b".repeat(40)] {
            let uri = commit_url(Some(repository), &sha, false, "writer@example.test").unwrap();
            assert_eq!(
                cache
                    .resolve(&uri, |url| {
                        requests.push(url.to_owned());
                        if url.contains("/git/commits/") {
                            return Ok(serde_json::to_vec(&serde_json::json!({
                                "sha": sha,
                                "author": {"login": "writer"},
                                "committer": {"username": "writer"}
                            }))?);
                        }
                        if url == profile {
                            return Ok(br#"{"avatar_url":"/avatars/photo?size=512"}"#.to_vec());
                        }
                        assert_eq!(url, image);
                        Ok(b"avatar".to_vec())
                    })
                    .unwrap(),
                b"avatar"
            );
        }
        assert_eq!(requests.len(), 4);
        assert_eq!(
            requests
                .iter()
                .filter(|url| url.as_str() == profile)
                .count(),
            1
        );
        assert_eq!(
            requests.iter().filter(|url| url.as_str() == image).count(),
            1
        );
    }

    #[test]
    fn ambiguous_remote_tries_gitea_then_bitbucket_and_shares_commit_results() {
        let sha = "a".repeat(40);
        let remote = "https://user:password@git.example.test:8443/dev/scm/team/repo.git";
        let author = commit_url(Some(remote), &sha, false, "private@example.test").unwrap();
        let committer = commit_url(Some(remote), &sha, true, "other@example.test").unwrap();
        assert!(!author.contains("password"));
        assert!(!author.contains("private"));
        let mut cache = SessionCache::default();
        let mut requests = Vec::new();
        let mut fetch = |url: &str| {
            requests.push(url.to_owned());
            if url.contains("/api/v1/") {
                return Ok(serde_json::to_vec(&serde_json::json!({
                    "sha": sha,
                    "author": {"avatar_url": "https://elsewhere.example.test/image"},
                    "committer": {"avatar_url": "/avatars/committer"}
                }))?);
            }
            if url.contains("/rest/api/") {
                return Ok(serde_json::to_vec(&serde_json::json!({
                    "id": sha,
                    "author": {"slug": "writer"},
                    "committer": {"name": "Raw Git name"}
                }))?);
            }
            Ok(url.as_bytes().to_vec())
        };
        assert_eq!(
            cache.resolve(&author, &mut fetch).unwrap(),
            b"https://git.example.test:8443/dev/users/writer/avatar.png?s=80"
        );
        assert_eq!(
            cache.resolve(&committer, &mut fetch).unwrap(),
            b"https://git.example.test:8443/avatars/committer"
        );
        cache.resolve(&author, &mut fetch).unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].contains("/dev/scm/api/v1/repos/team/repo/git/commits/"));
        assert!(requests[1].contains("/dev/rest/api/1.0/projects/team/repos/repo/commits/"));
    }

    #[test]
    fn failed_hosted_probes_are_cached_without_gravatar() {
        let remote =
            web_repository_from_remote("ssh://git@internal.example.test:7999/TEAM/repo.git")
                .unwrap();
        assert_eq!(remote, "https://internal.example.test/TEAM/repo.git");
        assert_eq!(
            web_repository_from_remote("git@internal.example.test:TEAM/repo.git").as_deref(),
            Some(remote.as_str())
        );
        let uri = commit_url(
            Some(&remote),
            &"b".repeat(40),
            false,
            "private@example.test",
        )
        .unwrap();
        let mut cache = SessionCache::default();
        let mut count = 0;
        for _ in 0..2 {
            assert!(
                cache
                    .resolve(&uri, |url| {
                        assert!(url.starts_with("https://internal.example.test/"));
                        count += 1;
                        anyhow::bail!("unavailable")
                    })
                    .is_err()
            );
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn gravatar_bytes_and_invalid_api_results_are_cached() {
        let mut cache = SessionCache::default();
        let image = url("person@example.invalid").unwrap();
        let mut calls = 0;
        let mut fetch = |_: &str| {
            calls += 1;
            Ok(b"photo".to_vec())
        };
        assert_eq!(cache.resolve(&image, &mut fetch).unwrap(), b"photo");
        assert_eq!(cache.resolve(&image, &mut fetch).unwrap(), b"photo");
        assert_eq!(calls, 1);
        let parsed = parse_profiles(
            br#"{"author":null,"committer":{"avatar_url":"https://evil.test/photo"}}"#,
        )
        .unwrap();
        assert_eq!(parsed, [None, None]);
        assert!(parse_profiles(b"not-json").is_err());
        assert!(parse_profiles(br#"{"message":"API rate limit exceeded"}"#).is_err());
    }

    #[test]
    fn github_remote_and_request_parsing_keep_credentials_and_email_out() {
        for remote in [
            "git@github.com:owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
            "https://token@github.com/owner/repo.git",
        ] {
            assert_eq!(
                repository_from_remote(remote).as_deref(),
                Some("owner/repo")
            );
        }
        for remote in [
            "https://github.com.evil.test/owner/repo",
            "https://gitlab.com/owner/repo",
            "https://github.com/owner/repo/tree/main",
        ] {
            assert!(repository_from_remote(remote).is_none());
        }
        assert_eq!(
            repository_from_remote("git@github.com:owner/repo.git.git").as_deref(),
            Some("owner/repo.git")
        );
        let sha = "a".repeat(40);
        let uri = commit_url(Some("owner/repo"), &sha, true, "Person@example.invalid").unwrap();
        assert!(!uri.contains("Person") && !uri.contains('@'));
        let request = github_request(&uri).unwrap();
        assert_eq!(request.role, 1);
        assert!(
            request
                .api
                .starts_with("https://api.github.com/repos/owner/repo/commits/")
        );
        assert_eq!(request.fallback, url("person@example.invalid"));
        assert_eq!(
            commit_url(None, &sha, false, "person@example.invalid"),
            url("person@example.invalid")
        );
    }

    #[test]
    fn github_profiles_commit_and_images_are_cached_until_restart() {
        let mut cache = SessionCache::default();
        let mut calls = Vec::new();
        let data = br#"{"author":{"avatar_url":"https://avatars.githubusercontent.com/u/1"},"committer":{"avatar_url":"https://avatars.githubusercontent.com/u/2"}}"#;
        let mut fetch = |uri: &str| {
            calls.push(uri.to_owned());
            Ok(if uri.contains("api.github.com") {
                data.to_vec()
            } else {
                uri.as_bytes().to_vec()
            })
        };
        let author = commit_url(
            Some("owner/repo"),
            &"a".repeat(40),
            false,
            "author@example.invalid",
        )
        .unwrap();
        let committer = commit_url(
            Some("owner/repo"),
            &"a".repeat(40),
            true,
            "committer@example.invalid",
        )
        .unwrap();
        assert_eq!(
            cache.resolve(&author, &mut fetch).unwrap(),
            b"https://avatars.githubusercontent.com/u/1"
        );
        assert_eq!(
            cache.resolve(&committer, &mut fetch).unwrap(),
            b"https://avatars.githubusercontent.com/u/2"
        );
        cache.resolve(&author, &mut fetch).unwrap();
        let next = commit_url(
            Some("other/repo"),
            &"b".repeat(40),
            false,
            "AUTHOR@example.invalid",
        )
        .unwrap();
        cache.resolve(&next, &mut fetch).unwrap();
        assert_eq!(calls.len(), 3); // API 1회, 서로 다른 사진 2회
        assert_eq!(cache.commits.len(), 1);
        assert!(SessionCache::default().images.is_empty());
    }

    #[test]
    fn github_and_photo_errors_fall_back_and_are_negatively_cached() {
        let mut cache = SessionCache::default();
        let uri = commit_url(
            Some("owner/repo"),
            &"a".repeat(40),
            false,
            "author@example.invalid",
        )
        .unwrap();
        let mut calls = Vec::new();
        let mut fetch = |uri: &str| {
            calls.push(uri.to_owned());
            if uri.contains("api.github.com") {
                return Ok(br#"{"author":{"avatar_url":"https://avatars.githubusercontent.com/u/1"},"committer":null}"#.to_vec());
            }
            if uri.contains("gravatar.com") {
                return Ok(b"gravatar".to_vec());
            }
            anyhow::bail!("사진 오류")
        };
        assert_eq!(cache.resolve(&uri, &mut fetch).unwrap(), b"gravatar");
        assert_eq!(cache.resolve(&uri, &mut fetch).unwrap(), b"gravatar");
        assert_eq!(calls.len(), 3);
        let mut failed = SessionCache::default();
        let mut failures = 0;
        let mut fetch = |_: &str| {
            failures += 1;
            anyhow::bail!("오프라인")
        };
        assert!(failed.resolve(&uri, &mut fetch).is_err());
        assert!(failed.resolve(&uri, &mut fetch).is_err());
        assert_eq!(failures, 2); // API와 Gravatar 각각 1회
    }

    #[test]
    fn unlinked_github_users_use_gravatar_and_reuse_email_mapping() {
        let mut cache = SessionCache::default();
        let mut count = 0;
        let mut fetch = |uri: &str| {
            count += 1;
            Ok(if uri.contains("api.github.com") {
                br#"{"author":null,"committer":null}"#.to_vec()
            } else {
                b"fallback".to_vec()
            })
        };
        for sha in ["a".repeat(40), "b".repeat(40)] {
            let uri = commit_url(Some("owner/repo"), &sha, false, "same@example.invalid").unwrap();
            assert_eq!(cache.resolve(&uri, &mut fetch).unwrap(), b"fallback");
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn photo_queue_is_bounded_and_rejections_do_not_block() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let client = AvatarClient { sender };
        let request = || {
            Request::get(url("person@example.invalid").unwrap())
                .body(AsyncBody::empty())
                .unwrap()
        };
        let first = client.send(request());
        let overflow = client.send(request());
        assert!(futures::executor::block_on(overflow).is_err());
        let (_, reply) = receiver.try_recv().unwrap();
        assert!(
            reply
                .send(Ok(Response::builder()
                    .status(404)
                    .body(AsyncBody::empty())
                    .unwrap()))
                .is_ok()
        );
        assert_eq!(futures::executor::block_on(first).unwrap().status(), 404);
        let invalid = client.send(
            Request::get("https://example.com/photo")
                .body(AsyncBody::empty())
                .unwrap(),
        );
        assert!(futures::executor::block_on(invalid).is_err());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn email_is_normalized_hashed_and_not_sent_in_plain_text() {
        let image = url(" MyEmailAddress@example.com ").unwrap();
        assert_eq!(image, url("myemailaddress@example.com").unwrap());
        assert!(image.contains("84059b07d4be67b806386c0aad8070a23f18836bbaae342275dc0a83414c32ee"));
        assert!(!image.contains('@'));
        assert!(url("  ").is_none());
        assert!(allowed(&image));
        assert!(!allowed("https://example.com/avatar/test"));
        assert!(!allowed(&image.replace("https:", "http:")));
    }
    #[test]
    fn image_response_size_is_bounded() {
        assert_eq!(bounded_bytes(&b"photo"[..]).unwrap(), b"photo");
        assert!(bounded_bytes(std::io::repeat(1).take(MAX_BYTES as u64 + 1)).is_err());
    }
}
