use crate::{
    graph::GraphBuilder,
    model::{Commit, RefTab, Reference},
};
use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use std::{collections::HashMap, path::PathBuf, sync::mpsc};

pub const PAGE_SIZE: usize = 200;

#[derive(Debug)]
pub struct CommitPage {
    pub commits: Vec<Commit>,
    pub has_more: bool,
}

#[derive(Clone, Debug)]
pub struct HistorySession {
    requests: mpsc::Sender<mpsc::SyncSender<Result<CommitPage>>>,
}

impl HistorySession {
    pub fn start(
        path: PathBuf,
        tips: Vec<gix::ObjectId>,
        references: &[Reference],
    ) -> Result<Self> {
        let mut branches: HashMap<String, Vec<String>> = HashMap::new();
        for reference in references {
            if matches!(reference.kind, RefTab::Local | RefTab::Remote) {
                branches
                    .entry(reference.target.clone())
                    .or_default()
                    .push(reference.name.clone());
            }
        }
        let (requests, receiver) = mpsc::channel::<mpsc::SyncSender<Result<CommitPage>>>();
        std::thread::Builder::new()
            .name("commit-pages".into())
            .spawn(move || {
                let run = || -> Result<()> {
                    let repo = crate::git::open(&path)?;
                    let mut walk = gix::traverse::commit::topo::Builder::new(&repo.objects)
                        .with_tips(tips)
                        .build()?
                        .peekable();
                    let mut graph = GraphBuilder::default();
                    let mut pending = Vec::with_capacity(PAGE_SIZE);
                    for reply in &receiver {
                        let result = (|| -> Result<CommitPage> {
                            while pending.len() < PAGE_SIZE {
                                let Some(entry) = walk.peek() else { break };
                                let entry =
                                    entry.as_ref().map_err(|error| anyhow::anyhow!("{error}"))?;
                                let id = entry.id;
                                let commit = repo.find_commit(id)?;
                                let decoded = commit.decode()?;
                                let time = decoded.committer()?.time()?;
                                pending.push(Commit {
                                    id: id.to_string(),
                                    title: decoded
                                        .message
                                        .to_str_lossy()
                                        .lines()
                                        .next()
                                        .unwrap_or("")
                                        .to_owned(),
                                    author: decoded.author()?.name.to_str_lossy().into_owned(),
                                    time: time.seconds,
                                    time_offset: time.offset,
                                    parent_ids: commit
                                        .parent_ids()
                                        .map(|id| id.to_string())
                                        .collect(),
                                    branches: branches.remove(&id.to_string()).unwrap_or_default(),
                                    ..Commit::default()
                                });
                                walk.next();
                            }
                            let mut commits =
                                std::mem::replace(&mut pending, Vec::with_capacity(PAGE_SIZE));
                            graph.append(&mut commits);
                            Ok(CommitPage {
                                commits,
                                has_more: walk.peek().is_some(),
                            })
                        })();
                        if reply.send(result).is_err() {
                            break;
                        }
                    }
                    Ok(())
                };
                if let Err(error) = run()
                    && let Ok(reply) = receiver.recv()
                {
                    let _ = reply.send(Err(error));
                }
            })?;
        Ok(Self { requests })
    }

    /// UI가 아닌 백그라운드 실행기에서 호출한다.
    pub fn next_page(&self) -> Result<CommitPage> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.requests
            .send(reply)
            .context("커밋 탐색 세션이 종료되었습니다")?;
        receiver
            .recv()
            .context("커밋 페이지 응답이 중단되었습니다")?
    }
}
