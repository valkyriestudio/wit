use git2::{BranchType, ObjectType, Oid, ReferenceType, Signature, Status};
use serde::{Serialize, Serializer};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub(crate) struct GitBranch {
    pub(crate) kind: GitBranchType,
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) shorthand: MaybeLossyUtf8,
    pub(crate) target: Option<GitOid>,
    pub(crate) upstream: Option<GitUpstream>,
}

#[derive(Debug)]
pub(crate) struct GitBranchType(pub(crate) BranchType);

impl From<BranchType> for GitBranchType {
    fn from(t: BranchType) -> Self {
        GitBranchType(t)
    }
}

impl Serialize for GitBranchType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:?}", self.0).serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitCommit {
    pub(crate) author: GitSignature,
    pub(crate) committer: GitSignature,
    pub(crate) id: GitOid,
    pub(crate) message: MaybeLossyUtf8,
    pub(crate) time: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitIndex {
    pub(crate) ctime: i32,
    pub(crate) file_size: u32,
    pub(crate) gid: u32,
    pub(crate) id: GitOid,
    pub(crate) mode: u32,
    pub(crate) mtime: i32,
    pub(crate) path: MaybeLossyUtf8,
    pub(crate) uid: u32,
}

#[derive(Debug)]
pub(crate) struct GitObjectType(pub(crate) ObjectType);

impl From<ObjectType> for GitObjectType {
    fn from(t: ObjectType) -> Self {
        GitObjectType(t)
    }
}

impl Serialize for GitObjectType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:?}", self.0).serialize(serializer)
    }
}

#[derive(Debug)]
pub(crate) struct GitOid(pub(crate) Oid);

impl From<Oid> for GitOid {
    fn from(id: Oid) -> Self {
        GitOid(id)
    }
}

impl Serialize for GitOid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitReference {
    pub(crate) kind: Option<GitReferenceType>,
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) shorthand: MaybeLossyUtf8,
    pub(crate) target: Option<GitOid>,
}

#[derive(Debug)]
pub(crate) struct GitReferenceType(pub(crate) ReferenceType);

impl From<ReferenceType> for GitReferenceType {
    fn from(t: ReferenceType) -> Self {
        GitReferenceType(t)
    }
}

impl Serialize for GitReferenceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:?}", self.0).serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitRemote {
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) url: MaybeLossyUtf8,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitSignature {
    pub(crate) email: MaybeLossyUtf8,
    pub(crate) name: MaybeLossyUtf8,
}

impl From<Signature<'_>> for GitSignature {
    fn from(s: Signature<'_>) -> Self {
        GitSignature {
            email: s.email_bytes().into(),
            name: s.name_bytes().into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitStatus {
    pub(crate) path: MaybeLossyUtf8,
    pub(crate) status: GitStatusFlag,
    pub(crate) status_bits: u32,
}

#[derive(Debug)]
pub(crate) struct GitStatusFlag(pub(crate) Status);

impl From<Status> for GitStatusFlag {
    fn from(s: Status) -> Self {
        GitStatusFlag(s)
    }
}

impl Serialize for GitStatusFlag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0
            .iter_names()
            .map(|(s, _)| s)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitTag {
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) shorthand: MaybeLossyUtf8,
    pub(crate) target: GitOid,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitTree {
    pub(crate) filemode: i32,
    pub(crate) id: GitOid,
    pub(crate) kind: Option<GitObjectType>,
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) root: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitUpstream {
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) shorthand: MaybeLossyUtf8,
    pub(crate) target: Option<GitOid>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct MaybeLossyUtf8(pub(crate) String);

impl std::fmt::Display for MaybeLossyUtf8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&[u8]> for MaybeLossyUtf8 {
    fn from(bytes: &[u8]) -> Self {
        MaybeLossyUtf8(
            String::from_utf8(bytes.into())
                .map_err(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
                .expect("from_utf8_lossy() is infallible"),
        )
    }
}
