mod error;
pub(crate) mod model;

use std::{collections::HashSet, path::Path};

use git2::{
    Blob, Branch, Commit, ErrorClass, ErrorCode, IndexEntry, Object, ObjectType, Oid, Reference,
    Repository, Time, Tree, TreeEntry, TreeWalkMode, TreeWalkResult,
};
use time::{OffsetDateTime, UtcOffset};

pub(crate) use self::error::{GitError, GitResult};
use self::model::{
    GitBlob, GitBlobContent, GitBranch, GitCommit, GitIndex, GitIndexDirectory, GitIndexEntry,
    GitOid, GitReference, GitRemote, GitStatus, GitTag, GitTree, GitUpstream, MaybeLossyUtf8,
};

pub(crate) struct GitRepository {
    repo: Repository,
}

impl std::fmt::Debug for GitRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitRepository {{ repo: {:?} }}", self.repo.path())
    }
}

trait IdGetter {
    fn get_id(&self) -> GitOid;
}

impl IdGetter for Branch<'_> {
    fn get_id(&self) -> GitOid {
        self.get().get_id()
    }
}

impl IdGetter for Reference<'_> {
    fn get_id(&self) -> GitOid {
        self.resolve()
            .as_ref()
            .unwrap_or(self)
            .target()
            .unwrap_or(Oid::zero())
            .into()
    }
}

trait IntoDateTime {
    fn datetime(&self) -> OffsetDateTime;
}

impl IntoDateTime for Time {
    fn datetime(&self) -> OffsetDateTime {
        let offset = match (
            i8::try_from(self.offset_minutes() / 60),
            i8::try_from(self.offset_minutes() % 60),
        ) {
            (Ok(h), Ok(m)) => UtcOffset::from_hms(h, m, 0).unwrap_or(UtcOffset::UTC),
            _ => UtcOffset::UTC,
        };
        OffsetDateTime::from_unix_timestamp(self.seconds())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
            .to_offset(offset)
    }
}

trait ShortIdGetter {
    fn get_short_id(&self) -> String;
}

impl ShortIdGetter for Blob<'_> {
    fn get_short_id(&self) -> String {
        self.as_object().get_short_id()
    }
}

impl ShortIdGetter for Branch<'_> {
    fn get_short_id(&self) -> String {
        self.get().get_short_id()
    }
}

impl ShortIdGetter for Commit<'_> {
    fn get_short_id(&self) -> String {
        self.as_object().get_short_id()
    }
}

impl ShortIdGetter for Object<'_> {
    fn get_short_id(&self) -> String {
        self.short_id()
            .map(|s| s.as_str().map(str::to_string).unwrap_or_default())
            .unwrap_or_default()
    }
}

impl ShortIdGetter for Reference<'_> {
    fn get_short_id(&self) -> String {
        self.peel_to_commit()
            .map(|c| c.as_object().get_short_id())
            .unwrap_or_default()
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

    pub(crate) fn get_blob(&self, oid: GitOid) -> GitResult<GitBlob> {
        Ok(self.repo.find_blob(oid.0).map(|b| {
            let content = match b.is_binary() {
                true => GitBlobContent::Binary(b.content().into()),
                false => GitBlobContent::Text(b.content().into()),
            };
            GitBlob {
                content,
                id: b.id().into(),
                is_binary: b.is_binary(),
                short_id: b.get_short_id(),
                size: b.size(),
            }
        })?)
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
                target: b.get_id(),
                target_short: b.get_short_id(),
                upstream: b
                    .upstream()
                    .map(|u| {
                        Some(GitUpstream {
                            name: u.get().name_bytes().into(),
                            shorthand: u.name_bytes().unwrap_or_default().into(),
                            target: u.get_id(),
                            target_short: u.get_short_id(),
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
            .filter_map(|id| {
                self.repo
                    .find_commit(id)
                    .map(|c| {
                        Some(GitCommit {
                            author: c.author().into(),
                            committer: c.committer().into(),
                            id: c.id().into(),
                            message: c.message_bytes().into(),
                            short_id: c.get_short_id(),
                            time: c.time().datetime(),
                        })
                    })
                    .unwrap_or_default()
            })
            .collect())
    }

    pub(crate) fn list_index(&self, path: &str) -> GitResult<Vec<GitIndex>> {
        let path = path.strip_suffix('/').unwrap_or(path);
        let depth = if path.is_empty() {
            0
        } else {
            path.split('/').count()
        };
        let convert_to_index_entry = |entry: IndexEntry, path: MaybeLossyUtf8| GitIndexEntry {
            ctime: entry.ctime.seconds(),
            file_size: entry.file_size,
            gid: entry.gid,
            id: entry.id.into(),
            mode: entry.mode,
            mtime: entry.mtime.seconds(),
            name: path
                .0
                .split('/')
                .last()
                .map(str::to_string)
                .unwrap_or_default()
                .into(),
            path,
            short_id: self
                .repo
                .find_blob(entry.id)
                .map(|o| o.get_short_id())
                .unwrap_or_default(),
            uid: entry.uid,
        };
        let mut set = HashSet::<String>::new();
        Ok(self
            .repo
            .index()?
            .iter()
            .filter_map(|entry| {
                let full_path: MaybeLossyUtf8 = entry.path.as_slice().into();
                if full_path.0.eq(path) {
                    Some(convert_to_index_entry(entry, full_path).into())
                } else if path.is_empty() || full_path.0.starts_with(&format!("{path}/")) {
                    let components = full_path.0.splitn(depth + 2, '/').collect::<Vec<&str>>();
                    if components.len() == depth + 1 {
                        Some(convert_to_index_entry(entry, full_path).into())
                    } else {
                        let path = components[0..depth + 1].join("/");
                        if set.contains(&path) {
                            None
                        } else {
                            set.insert(path.clone());
                            Some(
                                GitIndexDirectory {
                                    name: components[depth].to_owned().into(),
                                    path: path.into(),
                                }
                                .into(),
                            )
                        }
                    }
                } else {
                    None
                }
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
                target: r.get_id(),
                target_short: r.get_short_id(),
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
                url: self
                    .repo
                    .find_remote(&String::from_utf8_lossy(r))
                    .map(|r| r.url_bytes().into())
                    .unwrap_or_default(),
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
                    target_short: r.get_short_id(),
                });
            }
            true
        })?;
        Ok(vec)
    }

    pub(crate) fn list_tree(&self, path: &str) -> GitResult<Vec<GitTree>> {
        let path = path.strip_suffix('/').unwrap_or(path);
        let commit = self.repo.head()?.peel_to_commit()?;
        let root = commit.tree()?;
        let convert_to_tree = |entry: &TreeEntry<'_>, root: &str| -> GitTree {
            GitTree {
                filemode: entry.filemode(),
                id: entry.id().into(),
                kind: entry.kind().map(Into::into),
                name: entry.name_bytes().into(),
                root: root.into(),
                short_id: entry
                    .to_object(&self.repo)
                    .map(|o| o.get_short_id())
                    .unwrap_or_default(),
            }
        };
        let collect_tree = |tree: Tree<'_>, root: &str| -> Vec<_> {
            tree.iter()
                .map(|entry| convert_to_tree(&entry, root))
                .collect()
        };
        if path.is_empty() {
            let vec = collect_tree(root, "");
            return Ok(vec);
        }
        let mut vec = vec![];
        root.walk(TreeWalkMode::PreOrder, |root, entry| {
            let curr = format!("{root}{}", entry.name().unwrap_or_default());
            if path.eq(&curr) {
                if let Some(ObjectType::Tree) = entry.kind() {
                    if let Ok(tree) = self.repo.find_tree(entry.id()) {
                        vec = collect_tree(tree, &format!("{curr}/"));
                    }
                } else {
                    vec.push(convert_to_tree(entry, root));
                }
                return TreeWalkResult::Abort;
            }
            if let Some(ObjectType::Tree) = entry.kind() {
                if !path.starts_with(&format!("{curr}/")) {
                    return TreeWalkResult::Skip;
                }
            }
            TreeWalkResult::Ok
        })
        .unwrap_or_default();
        Ok(vec)
    }

    pub(crate) fn open<P>(path: P) -> GitResult<GitRepository>
    where
        P: AsRef<Path>,
    {
        Repository::open(&path)
            .map(|r| GitRepository { repo: r })
            .map_err(|e| match (e.class(), e.code()) {
                (ErrorClass::Os | ErrorClass::Repository, ErrorCode::NotFound) => {
                    GitError::RepositoryNotFound(path.as_ref().into())
                }
                _ => e.into(),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use git2::Oid;

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
    fn test_get_blob() {
        let sample = [".."];
        for path in sample.into_iter() {
            GitRepository::open("..")
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .get_blob(GitOid(Oid::zero()))
                .expect_err("get_blob(all zero Oid) in git repo {path:?} is expected to fail");
        }
        let sample = [("..", "LICENSE")];
        for (path, tree) in sample.into_iter() {
            let repo = GitRepository::open(path)
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"));
            let entries = repo.list_tree(tree).unwrap_or_else(|e| {
                panic!("list_tree in git repo {path:?} should not fail: {e:?}")
            });
            for item in entries.into_iter() {
                let blob = repo.get_blob(item.id).unwrap_or_else(|e| {
                    panic!("get_blob in git repo {path:?} should not fail: {e:?}")
                });
                println!("{blob:?}");
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
        let sample = [
            ("..", Default::default(), Default::default()),
            ("..", "wit", String::from("wit/")),
            ("..", "wit/", String::from("wit/")),
            ("..", "wit/src/main.rs", String::from("wit/src/")),
        ];
        for (path, index, root) in sample.into_iter() {
            let entries = GitRepository::open(path)
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_index(index)
                .unwrap_or_else(|e| {
                    panic!("list_index in git repo {path:?} should not fail: {e:?}")
                });
            for item in entries.iter() {
                println!("{item:?}");
                let full_path = match item {
                    GitIndex::Directory(e) => &e.path,
                    GitIndex::Entry(e) => &e.path,
                };
                if !index.is_empty() {
                    assert!(
                        full_path.0.starts_with(&root),
                        "unexpected root of index entry"
                    );
                }
                assert_eq!(
                    full_path.0.split('/').count(),
                    root.split('/').count(),
                    "unexpected root of index entry"
                )
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
        let sample = [
            ("..", Default::default(), Default::default()),
            ("..", "wit", String::from("wit/")),
            ("..", "wit/", String::from("wit/")),
            ("..", "wit/src/main.rs", String::from("wit/src/")),
        ];
        for (path, tree, root) in sample.into_iter() {
            let entries = GitRepository::open(path)
                .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"))
                .list_tree(tree)
                .unwrap_or_else(|e| {
                    panic!("list_tree in git repo {path:?} should not fail: {e:?}")
                });
            for item in entries.iter() {
                println!("{item:?}");
                assert_eq!(item.root, root, "unexpected root of tree entry");
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
