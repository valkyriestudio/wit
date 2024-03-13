use askama::Template;
use axum::{extract::Path, response::Html, routing::get, Router};

use crate::service::git::{model::GitTree, GitRepository};

pub(crate) fn router() -> Router {
    Router::new()
        .route("/", get(hello))
        .route("/tree", get(list_root_tree))
        .route("/tree/*path", get(list_tree))
}

async fn hello() -> Html<&'static str> {
    Html("<p>Hello, World!</p>")
}

#[derive(Template)]
#[template(path = "repo-tree.html")]
struct RepoTreeTemplate {
    data: Vec<GitTree>,
}

async fn list_root_tree() -> RepoTreeTemplate {
    let data = GitRepository::open(".")
        .unwrap()
        .list_tree(Default::default())
        .unwrap();
    RepoTreeTemplate { data }
}

async fn list_tree(Path(path): Path<String>) -> RepoTreeTemplate {
    let data = GitRepository::open(".").unwrap().list_tree(path).unwrap();
    RepoTreeTemplate { data }
}
