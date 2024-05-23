use askama::Template;
use axum::{
    extract::{path::ErrorKind, rejection::PathRejection, Path, State},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::service::git::{
    model::{GitBlob, GitBlobContent, GitIndex, GitObjectType, GitTree},
    GitError, GitRepository,
};

use super::{api::ApiError, AppState};

pub(crate) type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug)]
pub(crate) enum RenderError {
    ApiError(ApiError),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::ApiError(e) => write!(f, "ApiError: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

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

impl IntoResponse for RenderError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            RenderError::ApiError(e) => e.into(),
        };
        (
            status,
            ErrorTemplate {
                code: status.into(),
                message,
            },
        )
            .into_response()
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
        .route("/index/*path", get(list_index))
        .route("/tree", get(list_tree))
        .route("/tree/*path", get(list_tree))
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

async fn hello() -> RenderResult<HelloTemplate> {
    Ok(HelloTemplate {})
}

async fn list_index(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> RenderResult<RepoIndexTemplate> {
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
                return Ok(RepoIndexTemplate {
                    data: IndexView::Blob(blob),
                    segments,
                });
            }
        }
    }
    Ok(RepoIndexTemplate {
        data: IndexView::Index(index),
        segments,
    })
}

async fn list_tree(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> RenderResult<RepoTreeTemplate> {
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
            return Ok(RepoTreeTemplate {
                data: TreeView::Blob(blob),
                segments,
            });
        }
    }
    Ok(RepoTreeTemplate {
        data: TreeView::Tree(tree),
        segments,
    })
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
