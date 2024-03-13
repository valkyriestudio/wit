use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::service::git::{
    model::{GitBranch, GitCommit, GitIndex, GitReference, GitRemote, GitStatus, GitTag, GitTree},
    GitError, GitRepository,
};

pub(crate) type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub(crate) enum ApiError {
    Git(GitError),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Git(path) => write!(f, "GitError: {}", path),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<GitError> for ApiError {
    fn from(e: GitError) -> Self {
        ApiError::Git(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Git(e) => match e {
                GitError::RepositoryNotFound(p) => (
                    StatusCode::NOT_FOUND,
                    format!("Git repository {p:?} not found"),
                ),
                GitError::Unhandled(_) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
            },
        };
        (status, Json(ErrorResponse { message })).into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

pub(crate) fn router() -> Router {
    Router::new()
        .route("/status", get(gather_status))
        .route("/branch", get(list_branch))
        .route("/commit", get(list_commit))
        .route("/index", get(list_index))
        .route("/reference", get(list_reference))
        .route("/remote", get(list_remote))
        .route("/tag", get(list_tag))
        .route("/tree", get(list_tree))
}

async fn gather_status() -> ApiResult<Json<Vec<GitStatus>>> {
    Ok(Json(GitRepository::open(".")?.gather_status()?))
}

async fn list_branch() -> ApiResult<Json<Vec<GitBranch>>> {
    Ok(Json(GitRepository::open(".")?.list_branch()?))
}

async fn list_commit() -> ApiResult<Json<Vec<GitCommit>>> {
    Ok(Json(GitRepository::open(".")?.list_commit()?))
}

async fn list_index() -> ApiResult<Json<Vec<GitIndex>>> {
    Ok(Json(GitRepository::open(".")?.list_index()?))
}

async fn list_reference() -> ApiResult<Json<Vec<GitReference>>> {
    Ok(Json(GitRepository::open(".")?.list_reference()?))
}

async fn list_remote() -> ApiResult<Json<Vec<GitRemote>>> {
    Ok(Json(GitRepository::open(".")?.list_remote()?))
}

async fn list_tag() -> ApiResult<Json<Vec<GitTag>>> {
    Ok(Json(GitRepository::open(".")?.list_tag()?))
}

async fn list_tree() -> ApiResult<Json<Vec<GitTree>>> {
    Ok(Json(
        GitRepository::open(".")?.list_tree(Default::default())?,
    ))
}
