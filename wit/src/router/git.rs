use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};

use crate::service::git::{
    model::{GitBlob, GitBlobContent, GitIndex, GitObjectType, GitTree},
    GitError, GitRepository,
};

use super::AppState;

pub(crate) type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug)]
pub(crate) enum RenderError {
    Git(GitError),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Git(path) => write!(f, "GitError: {}", path),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<GitError> for RenderError {
    fn from(e: GitError) -> Self {
        RenderError::Git(e)
    }
}

impl IntoResponse for RenderError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            RenderError::Git(e) => match e {
                GitError::RepositoryNotFound(p) => (
                    StatusCode::NOT_FOUND,
                    format!("Git repository {p:?} not found"),
                ),
                GitError::Unhandled(_) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
            },
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
        .route("/index", get(list_root_index))
        .route("/index/*path", get(list_index))
        .route("/tree", get(list_root_tree))
        .route("/tree/*path", get(list_tree))
}

async fn hello() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width">
        <title>wit</title>
        <link href="/assets/daisyui.full.min.css" rel="stylesheet" type="text/css"/>
        <script src="/assets/tailwind.js"></script>
    </head>
    <body>
        <div class="container mx-auto my-2">
            <p><a class="link text-info text-xl" href="/git/index">index</a><p>
            <p><a class="link text-info text-xl" href="/git/tree">tree</a><p>
        </div>
    </body>
</html>
"#,
    )
}

#[derive(Template)]
#[template(path = "repo-index.html")]
struct RepoIndexTemplate {
    data: IndexView,
}

enum IndexView {
    Blob(GitBlob),
    Index(Vec<GitIndex>),
}

#[derive(Template)]
#[template(path = "repo-tree.html")]
struct RepoTreeTemplate {
    data: TreeView,
}

enum TreeView {
    Blob(GitBlob),
    Tree(Vec<GitTree>),
}

async fn list_root_index(State(state): State<AppState>) -> RenderResult<RepoIndexTemplate> {
    let index = GitRepository::open(state.repo_root)?.list_index()?;
    Ok(RepoIndexTemplate {
        data: IndexView::Index(index),
    })
}

async fn list_index(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> RenderResult<RepoIndexTemplate> {
    let repo = GitRepository::open(state.repo_root)?;
    let index = repo.list_index()?;
    if let Some(item) = index.iter().find(|i| i.path.0.eq(&path)) {
        let blob = repo.get_blob(item.id.clone())?;
        return Ok(RepoIndexTemplate {
            data: IndexView::Blob(blob),
        });
    }
    Ok(RepoIndexTemplate {
        data: IndexView::Index(index),
    })
}

async fn list_root_tree(State(state): State<AppState>) -> RenderResult<RepoTreeTemplate> {
    let tree = GitRepository::open(state.repo_root)?.list_tree(Default::default())?;
    Ok(RepoTreeTemplate {
        data: TreeView::Tree(tree),
    })
}

async fn list_tree(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> RenderResult<RepoTreeTemplate> {
    let repo = GitRepository::open(state.repo_root)?;
    let tree = repo.list_tree(&path)?;
    if tree.len() == 1 {
        let entry = &tree[0];
        if format!("{}{}", entry.root, entry.name).eq(&path) {
            let blob = repo.get_blob(entry.id.clone())?;
            return Ok(RepoTreeTemplate {
                data: TreeView::Blob(blob),
            });
        }
    }
    Ok(RepoTreeTemplate {
        data: TreeView::Tree(tree),
    })
}
