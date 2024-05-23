use git2::{BranchType, ObjectType, Oid, ReferenceType, Signature, Status};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub(crate) struct GitBlob {
    pub(crate) content: GitBlobContent,
    pub(crate) id: GitOid,
    pub(crate) is_binary: bool,
    pub(crate) short_id: String,
    pub(crate) size: usize,
}

#[derive(Debug, Serialize)]
pub(crate) enum GitBlobContent {
    Binary(Vec<u8>),
    Text(MaybeLossyUtf8),
}

impl std::fmt::Display for GitBlobContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitBlobContent::Binary(data) => write!(f, "{data:X?}"),
            GitBlobContent::Text(data) => data.fmt(f),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitBranch {
    pub(crate) kind: GitBranchType,
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) shorthand: MaybeLossyUtf8,
    pub(crate) target: GitOid,
    pub(crate) target_short: String,
    pub(crate) upstream: Option<GitUpstream>,
}

#[derive(Debug, Serialize)]
pub(crate) enum GitBranchType {
    Local,
    Remote,
}

impl From<BranchType> for GitBranchType {
    fn from(t: BranchType) -> Self {
        match t {
            BranchType::Local => GitBranchType::Local,
            BranchType::Remote => GitBranchType::Remote,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct GitCommit {
    pub(crate) author: GitSignature,
    pub(crate) committer: GitSignature,
    pub(crate) id: GitOid,
    pub(crate) message: MaybeLossyUtf8,
    pub(crate) short_id: String,
    pub(crate) time: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub(crate) enum GitIndex {
    Directory(GitIndexDirectory),
    Entry(GitIndexEntry),
}

#[derive(Debug, Serialize)]
pub(crate) struct GitIndexDirectory {
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) path: MaybeLossyUtf8,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitIndexEntry {
    pub(crate) ctime: i32,
    pub(crate) file_size: u32,
    pub(crate) gid: u32,
    pub(crate) id: GitOid,
    pub(crate) mode: u32,
    pub(crate) mtime: i32,
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) path: MaybeLossyUtf8,
    pub(crate) short_id: String,
    pub(crate) uid: u32,
}

impl From<GitIndexDirectory> for GitIndex {
    fn from(d: GitIndexDirectory) -> Self {
        GitIndex::Directory(d)
    }
}

impl From<GitIndexEntry> for GitIndex {
    fn from(e: GitIndexEntry) -> Self {
        GitIndex::Entry(e)
    }
}

impl std::fmt::Display for GitIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitIndex::Directory(data) => write!(f, "{data:?}"),
            GitIndex::Entry(data) => write!(f, "{data:?}"),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) enum GitObjectType {
    Any,
    Blob,
    Commit,
    Tag,
    Tree,
}

impl From<ObjectType> for GitObjectType {
    fn from(t: ObjectType) -> Self {
        match t {
            ObjectType::Any => GitObjectType::Any,
            ObjectType::Blob => GitObjectType::Blob,
            ObjectType::Commit => GitObjectType::Commit,
            ObjectType::Tag => GitObjectType::Tag,
            ObjectType::Tree => GitObjectType::Tree,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GitOid(pub(crate) Oid);

impl std::fmt::Display for GitOid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Oid> for GitOid {
    fn from(id: Oid) -> Self {
        GitOid(id)
    }
}

struct GitOidVisitor;

impl<'de> Visitor<'de> for GitOidVisitor {
    type Value = GitOid;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a hex-formatted string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Oid::from_str(value) {
            Ok(oid) => Ok(GitOid(oid)),
            Err(e) => Err(E::custom(format!(
                "{:?} {:?}: {}",
                e.class(),
                e.code(),
                e.message()
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for GitOid {
    fn deserialize<D>(deserializer: D) -> Result<GitOid, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(GitOidVisitor)
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
    pub(crate) target: GitOid,
    pub(crate) target_short: String,
}

#[derive(Debug, Serialize)]
pub(crate) enum GitReferenceType {
    Direct,
    Symbolic,
}

impl From<ReferenceType> for GitReferenceType {
    fn from(t: ReferenceType) -> Self {
        match t {
            ReferenceType::Direct => GitReferenceType::Direct,
            ReferenceType::Symbolic => GitReferenceType::Symbolic,
        }
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
    pub(crate) target_short: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitTree {
    pub(crate) filemode: i32,
    pub(crate) id: GitOid,
    pub(crate) kind: Option<GitObjectType>,
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) root: String,
    pub(crate) short_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitUpstream {
    pub(crate) name: MaybeLossyUtf8,
    pub(crate) shorthand: MaybeLossyUtf8,
    pub(crate) target: GitOid,
    pub(crate) target_short: String,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct MaybeLossyUtf8(pub(crate) String);

impl std::fmt::Display for MaybeLossyUtf8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for MaybeLossyUtf8 {
    fn from(s: String) -> Self {
        MaybeLossyUtf8(s)
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
