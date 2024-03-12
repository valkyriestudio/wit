use std::path::Path;

pub(crate) type GitResult<T> = Result<T, GitError>;

#[derive(Debug)]
pub(crate) enum GitError {
    RepositoryNotFound(Box<Path>),
    Unhandled(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::RepositoryNotFound(path) => write!(f, "RepositoryNotFound: {:?}", path),
            GitError::Unhandled(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for GitError {}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError::Unhandled(format!(
            "Unhandled {:?} {:?}: {}",
            e.class(),
            e.code(),
            e.message()
        ))
    }
}
