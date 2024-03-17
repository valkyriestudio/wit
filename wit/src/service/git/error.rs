use std::path::Path;

use git2::{ErrorClass, ErrorCode};

pub(crate) type GitResult<T> = Result<T, GitError>;

#[derive(Debug)]
pub(crate) enum GitError {
    ObjectNotFound(String),
    RepositoryNotFound(Box<Path>),
    Unhandled(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::ObjectNotFound(message) => write!(f, "ObjectNotFound: {message}"),
            GitError::RepositoryNotFound(path) => write!(f, "RepositoryNotFound: {:?}", path),
            GitError::Unhandled(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GitError {}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        match (e.class(), e.code()) {
            (ErrorClass::Odb, ErrorCode::NotFound) => GitError::ObjectNotFound(e.message().into()),
            _ => GitError::Unhandled(format!(
                "Unhandled {:?} {:?}: {}",
                e.class(),
                e.code(),
                e.message()
            )),
        }
    }
}
