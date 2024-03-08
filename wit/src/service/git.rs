use std::path::Path;

use git2::{
    BranchType, ErrorClass, ErrorCode, ReferenceType, Repository, Status, TreeWalkMode,
    TreeWalkResult,
};

type GitResult<T> = Result<T, GitError>;

#[derive(Debug)]
enum GitError {
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
        GitError::Unhandled(format!("{:?} {:?}: {}", e.class(), e.code(), e.message()))
    }
}

struct GitRepository {
    repo: Repository,
}

impl std::fmt::Debug for GitRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.repo.path())
    }
}

impl GitRepository {
    fn gather_status(&self) -> GitResult<Vec<(Option<String>, Status)>> {
        Ok(self
            .repo
            .statuses(None)?
            .iter()
            .map(|s| (s.path().map(str::to_string), s.status()))
            .collect())
    }

    fn list_branch(&self) -> GitResult<Vec<(Option<String>, BranchType)>> {
        Ok(self
            .repo
            .branches(None)?
            .flatten()
            .map(|(b, t)| (b.name().unwrap_or_default().map(str::to_string), t))
            .collect())
    }

    fn list_commit(&self) -> GitResult<Vec<()>> {
        let commit = self.repo.head()?.peel_to_commit()?;
        println!(
            "{:?} {} {} {} {:?}",
            commit.time(),
            commit.id(),
            commit.author(),
            commit.committer(),
            commit.message(),
        );
        commit
            .tree()
            .unwrap()
            .walk(TreeWalkMode::PreOrder, |root, entry| {
                println!("{} {:?}", root, entry.name());
                TreeWalkResult::Ok
            })
            .unwrap_or_default();
        Ok(vec![])
    }

    fn list_index(&self) -> GitResult<Vec<()>> {
        for e in self.repo.index()?.iter() {
            println!("{e:?}");
        }
        Ok(vec![])
    }

    fn list_reference(&self) -> GitResult<Vec<(Option<String>, Option<ReferenceType>)>> {
        Ok(self
            .repo
            .references()?
            .flatten()
            .map(|r| (r.name().map(str::to_string), r.kind()))
            .collect())
    }

    fn list_remote(&self) -> GitResult<Vec<Option<String>>> {
        Ok(self
            .repo
            .remotes()?
            .iter()
            .map(|r| r.map(str::to_string))
            .collect())
    }

    fn list_tag(&self) -> GitResult<Vec<Option<String>>> {
        Ok(self
            .repo
            .tag_names(None)?
            .iter()
            .map(|t| t.map(str::to_string))
            .collect())
    }

    fn open<P>(path: P) -> GitResult<GitRepository>
    where
        P: AsRef<Path>,
    {
        let path: Box<Path> = path.as_ref().into();
        Repository::open(&path)
            .map(|r| GitRepository { repo: r })
            .map_err(|e| match (e.class(), e.code()) {
                (ErrorClass::Os | ErrorClass::Repository, ErrorCode::NotFound) => {
                    GitError::RepositoryNotFound(path.clone())
                }
                _ => e.into(),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn test_gather_status() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .gather_status()
                .unwrap_or_else(|e| {
                    panic!("gather_status in git repo {path:?} should not fail: {e:?}")
                });
            for (path, status) in entries.iter() {
                println!("{path:?}, {status:?}");
            }
        }
    }

    #[test]
    fn test_list_branch() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_branch()
                .unwrap_or_else(|e| {
                    panic!("list_branch in git repo {path:?} should not fail: {e:?}")
                });
            for branch in entries.iter() {
                println!("{branch:?}");
            }
        }
    }

    #[test]
    fn test_list_commit() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_commit()
                .unwrap_or_else(|e| {
                    panic!("list_commit in git repo {path:?} should not fail: {e:?}")
                });
            for commit in entries.iter() {
                println!("{commit:?}");
            }
        }
    }

    #[test]
    fn test_list_index() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_index()
                .unwrap_or_else(|e| {
                    panic!("list_index in git repo {path:?} should not fail: {e:?}")
                });
            for item in entries.iter() {
                println!("{item:?}");
            }
        }
    }

    #[test]
    fn test_list_reference() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_reference()
                .unwrap_or_else(|e| {
                    panic!("list_reference in git repo {path:?} should not fail: {e:?}")
                });
            for reference in entries.iter() {
                println!("{reference:?}");
            }
        }
    }

    #[test]
    fn test_list_remote() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_remote()
                .unwrap_or_else(|e| {
                    panic!("list_remote in git repo {path:?} should not fail: {e:?}")
                });
            for remote in entries.iter() {
                println!("{remote:?}");
            }
        }
    }

    #[test]
    fn test_list_tag() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_tag()
                .unwrap_or_else(|e| panic!("list_tag in git repo {path:?} should not fail: {e:?}"));
            for tag in entries.iter() {
                println!("{tag:?}");
            }
        }
    }

    #[test]
    fn test_open_repository() {
        let sample = [(".", None), ("wit", Some(OsStr::new("wit")))];
        for (path, exptcted) in sample.into_iter() {
            GitRepository::open(path)
                .map_err(|e| {
                    if let GitError::RepositoryNotFound(p) = e {
                        assert_eq!(p.file_name(), exptcted);
                    }
                })
                .expect_err("{path:?} should not be a valid git repo");
        }

        let sample = [".."];
        for path in sample.into_iter() {
            GitRepository::open(path)
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"));
        }
    }
}
