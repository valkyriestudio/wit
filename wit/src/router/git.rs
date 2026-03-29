use askama::Template;
use axum::{
    Router,
    extract::{Path, State, path::ErrorKind, rejection::PathRejection},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};

use crate::service::git::{
    GitError, GitRepository,
    model::{GitBlob, GitBlobContent, GitIndex, GitObjectType, GitTree},
};

use super::{AppState, api::ApiError};

pub(crate) type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug)]
pub(crate) enum RenderError {
    ApiError(ApiError),
    TemplateError(askama::Error),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::ApiError(e) => write!(f, "ApiError: {e}"),
            RenderError::TemplateError(e) => write!(f, "TemplateError: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<askama::Error> for RenderError {
    fn from(e: askama::Error) -> Self {
        RenderError::TemplateError(e)
    }
}

impl From<GitError> for RenderError {
    fn from(e: GitError) -> Self {
        RenderError::ApiError(e.into())
    }
}

impl From<PathRejection> for RenderError {
    fn from(e: PathRejection) -> Self {
        RenderError::ApiError(e.into())
    }
}

impl From<RenderError> for (StatusCode, String) {
    fn from(e: RenderError) -> Self {
        match e {
            RenderError::ApiError(e) => e.into(),
            RenderError::TemplateError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {e}"),
            ),
        }
    }
}

impl IntoResponse for RenderError {
    fn into_response(self) -> Response {
        let (status, message) = self.into();
        let template = ErrorTemplate {
            code: status.into(),
            message,
        };
        match template.render() {
            Ok(body) => (status, Html(body)).into_response(),
            Err(e) => {
                let resp: (StatusCode, String) = RenderError::TemplateError(e).into();
                resp.into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    code: u16,
    message: String,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(hello))
        .route("/index", get(list_index))
        .route("/index/{*path}", get(list_index))
        .route("/tree", get(list_tree))
        .route("/tree/{*path}", get(list_tree))
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {}

#[derive(Template)]
#[template(path = "repo-index.html")]
struct RepoIndexTemplate {
    data: IndexView,
    segments: Vec<String>,
}

enum IndexView {
    Blob(GitBlob),
    Index(Vec<GitIndex>),
}

#[derive(Template)]
#[template(path = "repo-tree.html")]
struct RepoTreeTemplate {
    data: TreeView,
    segments: Vec<String>,
}

enum TreeView {
    Blob(GitBlob),
    Tree(Vec<GitTree>),
}

async fn hello() -> RenderResult<impl IntoResponse> {
    Ok(Html(HelloTemplate {}.render()?))
}

async fn list_index(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> RenderResult<impl IntoResponse> {
    let path = path.or_else(map_empty_segment_to_default)?.0;
    let repo = GitRepository::open(state.repo_root)?;
    let mut index = repo.list_index(&path)?;
    let segments = if path.is_empty() {
        vec![]
    } else {
        path.split('/').map(str::to_string).collect()
    };
    if index.len() == 1 {
        let entry = &index[0];
        let full_path = match entry {
            GitIndex::Directory(e) => &e.path,
            GitIndex::Entry(e) => &e.path,
        };
        if full_path.0.eq(&path) {
            let entry = index.swap_remove(0);
            if let GitIndex::Entry(e) = entry {
                let blob = repo.get_blob(e.id)?;
                return Ok(Html(
                    RepoIndexTemplate {
                        data: IndexView::Blob(blob),
                        segments,
                    }
                    .render()?,
                ));
            }
        }
    }
    Ok(Html(
        RepoIndexTemplate {
            data: IndexView::Index(index),
            segments,
        }
        .render()?,
    ))
}

async fn list_tree(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> RenderResult<impl IntoResponse> {
    let path = path.or_else(map_empty_segment_to_default)?.0;
    let repo = GitRepository::open(state.repo_root)?;
    let mut tree = repo.list_tree(&path)?;
    let segments = if path.is_empty() {
        vec![]
    } else {
        path.split('/').map(str::to_string).collect()
    };
    if tree.len() == 1 {
        let entry = &tree[0];
        if format!("{}{}", entry.root, entry.name).eq(&path) {
            let entry = tree.swap_remove(0);
            let blob = repo.get_blob(entry.id)?;
            return Ok(Html(
                RepoTreeTemplate {
                    data: TreeView::Blob(blob),
                    segments,
                }
                .render()?,
            ));
        }
    }
    Ok(Html(
        RepoTreeTemplate {
            data: TreeView::Tree(tree),
            segments,
        }
        .render()?,
    ))
}

fn map_empty_segment_to_default(r: PathRejection) -> Result<Path<String>, PathRejection> {
    match r {
        PathRejection::FailedToDeserializePathParams(ref e) => match e.kind() {
            ErrorKind::WrongNumberOfParameters { got, expected } if *got == 0 && *expected == 1 => {
                Ok(Path(Default::default()))
            }
            _ => Err(r),
        },
        _ => Err(r),
    }
}
