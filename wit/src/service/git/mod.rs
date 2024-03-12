mod error;
pub(crate) mod model;

use std::path::Path;

use git2::{ErrorClass, ErrorCode, Repository, TreeWalkMode, TreeWalkResult};
use time::{OffsetDateTime, UtcOffset};

pub(crate) use self::error::{GitError, GitResult};
use self::model::{
    GitBranch, GitCommit, GitIndex, GitReference, GitRemote, GitStatus, GitTag, GitTree,
    GitUpstream,
};

pub(crate) struct GitRepository {
    repo: Repository,
}

impl std::fmt::Debug for GitRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.repo.path())
    }
}

impl GitRepository {
    pub(crate) fn gather_status(&self) -> GitResult<Vec<GitStatus>> {
        Ok(self
            .repo
            .statuses(None)?
            .iter()
            .map(|s| GitStatus {
                path: s.path_bytes().into(),
                status: s.status().into(),
                status_bits: s.status().bits(),
            })
            .collect())
    }

    pub(crate) fn list_branch(&self) -> GitResult<Vec<GitBranch>> {
        Ok(self
            .repo
            .branches(None)?
            .flatten()
            .map(|(b, t)| GitBranch {
                kind: t.into(),
                name: b.get().name_bytes().into(),
                shorthand: b.name_bytes().unwrap_or_default().into(),
                target: b
                    .get()
                    .resolve()
                    .as_ref()
                    .unwrap_or(b.get())
                    .target()
                    .map(Into::into),
                upstream: b
                    .upstream()
                    .map(|u| {
                        Some(GitUpstream {
                            name: u.get().name_bytes().into(),
                            shorthand: u.name_bytes().unwrap_or_default().into(),
                            target: u
                                .get()
                                .resolve()
                                .as_ref()
                                .unwrap_or(u.get())
                                .target()
                                .map(Into::into),
                        })
                    })
                    .unwrap_or_default(),
            })
            .collect())
    }

    pub(crate) fn list_commit(&self) -> GitResult<Vec<GitCommit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        Ok(revwalk
            .flatten()
            .filter_map(|oid| {
                if let Ok(commit) = self.repo.find_commit(oid) {
                    Some(GitCommit {
                        author: commit.author().into(),
                        committer: commit.committer().into(),
                        id: commit.id().into(),
                        message: commit.message_bytes().into(),
                        time: OffsetDateTime::from_unix_timestamp(commit.time().seconds())
                            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
                            .to_offset(
                                UtcOffset::from_hms(
                                    (commit.time().offset_minutes() / 60) as i8,
                                    (commit.time().offset_minutes() % 60) as i8,
                                    0,
                                )
                                .unwrap_or(UtcOffset::UTC),
                            ),
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    pub(crate) fn list_index(&self) -> GitResult<Vec<GitIndex>> {
        Ok(self
            .repo
            .index()?
            .iter()
            .map(|i| GitIndex {
                ctime: i.ctime.seconds(),
                file_size: i.file_size,
                gid: i.gid,
                id: i.id.into(),
                mode: i.mode,
                mtime: i.mtime.seconds(),
                path: i.path.as_slice().into(),
                uid: i.uid,
            })
            .collect())
    }

    pub(crate) fn list_reference(&self) -> GitResult<Vec<GitReference>> {
        Ok(self
            .repo
            .references()?
            .flatten()
            .map(|r| GitReference {
                kind: r.kind().map(Into::into),
                name: r.name_bytes().into(),
                shorthand: r.shorthand_bytes().into(),
                target: r.resolve().unwrap_or(r).target().map(Into::into),
            })
            .collect())
    }

    pub(crate) fn list_remote(&self) -> GitResult<Vec<GitRemote>> {
        Ok(self
            .repo
            .remotes()?
            .iter_bytes()
            .map(|r| GitRemote {
                name: r.into(),
                url: if let Ok(remote) = self.repo.find_remote(&String::from_utf8_lossy(r)) {
                    remote.url_bytes().into()
                } else {
                    Default::default()
                },
            })
            .collect())
    }

    pub(crate) fn list_tag(&self) -> GitResult<Vec<GitTag>> {
        let mut vec = vec![];
        self.repo.tag_foreach(|id, name| {
            if let Ok(r) = self.repo.find_reference(&String::from_utf8_lossy(name)) {
                vec.push(GitTag {
                    name: name.into(),
                    shorthand: r.shorthand_bytes().into(),
                    target: id.into(),
                });
            }
            true
        })?;
        Ok(vec)
    }

    pub(crate) fn list_tree(&self) -> GitResult<Vec<GitTree>> {
        let mut vec = vec![];
        let commit = self.repo.head()?.peel_to_commit()?;
        commit
            .tree()
            .unwrap()
            .walk(TreeWalkMode::PreOrder, |root, entry| {
                vec.push(GitTree {
                    filemode: entry.filemode(),
                    id: entry.id().into(),
                    kind: entry.kind().map(Into::into),
                    name: entry.name_bytes().into(),
                    root: root.to_owned(),
                });
                TreeWalkResult::Ok
            })
            .unwrap_or_default();
        Ok(vec)
    }

    pub(crate) fn open<P>(path: P) -> GitResult<GitRepository>
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
            for item in entries.iter() {
                println!("{item:?}");
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
            for item in entries.iter() {
                println!("{item:?}");
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
            for item in entries.iter() {
                println!("{item:?}");
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
            for item in entries.iter() {
                println!("{item:?}");
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
            for item in entries.iter() {
                println!("{item:?}");
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
            for item in entries.iter() {
                println!("{item:?}");
            }
        }
    }

    #[test]
    fn test_list_tree() {
        let sample = [".."];
        for path in sample.into_iter() {
            let entries = GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_tree()
                .unwrap_or_else(|e| {
                    panic!("list_tree in git repo {path:?} should not fail: {e:?}")
                });
            for item in entries.iter() {
                println!("{item:?}");
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
                    } else {
                        panic!("{e}");
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
