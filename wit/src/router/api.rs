use axum::{
    Json, Router,
    extract::{Path, State, rejection::PathRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

use crate::service::git::{
    GitError, GitRepository,
    model::{
        GitBlob, GitBranch, GitCommit, GitIndex, GitOid, GitReference, GitRemote, GitStatus,
        GitTag, GitTree,
    },
};

use super::AppState;

pub(crate) type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub(crate) enum ApiError {
    Git(GitError),
    PathRejection(PathRejection),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Git(e) => write!(f, "GitError: {e}"),
            ApiError::PathRejection(e) => write!(f, "PathRejection: {e}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<GitError> for ApiError {
    fn from(e: GitError) -> Self {
        ApiError::Git(e)
    }
}

impl From<PathRejection> for ApiError {
    fn from(e: PathRejection) -> Self {
        ApiError::PathRejection(e)
    }
}

impl From<ApiError> for (StatusCode, String) {
    fn from(e: ApiError) -> Self {
        match e {
            ApiError::Git(e) => match e {
                GitError::ObjectNotFound(message) => (
                    StatusCode::NOT_FOUND,
                    format!("Git object not found: {message}"),
                ),
                GitError::RepositoryNotFound(p) => (
                    StatusCode::NOT_FOUND,
                    format!("Git repository {p:?} not found"),
                ),
                GitError::Unhandled(_) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
            },
            ApiError::PathRejection(e) => (StatusCode::BAD_REQUEST, format!("PathRejection: {e}")),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = self.into();
        (status, Json(ErrorResponse { message })).into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/statuses", get(gather_status))
        .route("/blobs/{id}", get(get_blob))
        .route("/branches", get(list_branch))
        .route("/commits", get(list_commit))
        .route("/indexes", get(list_index))
        .route("/references", get(list_reference))
        .route("/remotes", get(list_remote))
        .route("/tags", get(list_tag))
        .route("/trees", get(list_tree))
}

async fn gather_status(State(state): State<AppState>) -> ApiResult<Json<Vec<GitStatus>>> {
    Ok(Json(GitRepository::open(state.repo_root)?.gather_status()?))
}

async fn get_blob(
    State(state): State<AppState>,
    id: Result<Path<GitOid>, PathRejection>,
) -> ApiResult<Json<GitBlob>> {
    let id = id?.0;
    Ok(Json(GitRepository::open(state.repo_root)?.get_blob(id)?))
}

async fn list_branch(State(state): State<AppState>) -> ApiResult<Json<Vec<GitBranch>>> {
    Ok(Json(GitRepository::open(state.repo_root)?.list_branch()?))
}

async fn list_commit(State(state): State<AppState>) -> ApiResult<Json<Vec<GitCommit>>> {
    Ok(Json(GitRepository::open(state.repo_root)?.list_commit()?))
}

async fn list_index(State(state): State<AppState>) -> ApiResult<Json<Vec<GitIndex>>> {
    Ok(Json(
        GitRepository::open(state.repo_root)?.list_index(Default::default())?,
    ))
}

async fn list_reference(State(state): State<AppState>) -> ApiResult<Json<Vec<GitReference>>> {
    Ok(Json(
        GitRepository::open(state.repo_root)?.list_reference()?,
    ))
}

async fn list_remote(State(state): State<AppState>) -> ApiResult<Json<Vec<GitRemote>>> {
    Ok(Json(GitRepository::open(state.repo_root)?.list_remote()?))
}

async fn list_tag(State(state): State<AppState>) -> ApiResult<Json<Vec<GitTag>>> {
    Ok(Json(GitRepository::open(state.repo_root)?.list_tag()?))
}

async fn list_tree(State(state): State<AppState>) -> ApiResult<Json<Vec<GitTree>>> {
    Ok(Json(
        GitRepository::open(state.repo_root)?.list_tree(Default::default())?,
    ))
}
