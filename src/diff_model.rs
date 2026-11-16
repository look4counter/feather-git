// 이전 프로젝트의 비교 원본과 차이 모델을 재사용합니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    Empty,
    Object(gix::ObjectId, u16),
    Worktree,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    pub path: Vec<u8>,
    pub label: String,
    pub status: String,
    pub staged: bool,
    pub before: Content,
    pub after: Content,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub kind: char,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct DetailIdentity {
    pub row: usize,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub worktree: Option<Vec<u8>>,
    pub index_state: Vec<(u8, gix::ObjectId, u32)>,
    pub identities: Vec<DetailIdentity>,
    pub note: String,
    pub lines: Vec<DiffLine>,
    pub eof_rows: std::collections::BTreeSet<usize>,
    pub empty_eof: bool,
    pub conflict: Option<crate::conflict::Preview>,
    pub conflict_resolved: bool,
    pub partial: Option<crate::partial_stage::Preview>,
}

impl Diff {
    pub fn conflict_at_row(&self, row: usize) -> Option<usize> {
        self.conflict
            .as_ref()?
            .region_for_line(self.lines.get(row)?.new?)
    }
}

#[derive(Clone, Debug)]
pub struct CommitDetail {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub author: String,
    pub author_email: String,
    pub author_time: i64,
    pub author_offset: i32,
    pub committer: String,
    pub committer_email: String,
    pub commit_time: i64,
    pub commit_offset: i32,
    pub message: String,
}
