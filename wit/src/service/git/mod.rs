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

impl From<Repository> for GitRepository {
    fn from(r: Repository) -> Self {
        GitRepository { repo: r }
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
    use std::{
        fs::{File, create_dir_all},
        io::Write,
    };

    use git2::{Signature, Status};
    use tempfile::tempdir;

    use model::GitBranchType;

    use super::*;

    fn commit_with_signature(
        repo: &Repository,
        tree_id: Oid,
        message: &str,
        name: &str,
        email: &str,
        time: Option<i64>,
    ) -> Oid {
        let tree = repo
            .find_tree(tree_id)
            .unwrap_or_else(|e| panic!("find git tree failed: {e:?}"));
        let sig = match time {
            Some(time) => Signature::new(name, email, &Time::new(time, 0)),
            None => Signature::now(name, email),
        }
        .unwrap_or_else(|e| panic!("create git signature failed: {e:?}"));
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
            .unwrap_or_else(|e| panic!("create git commit failed: {e:?}"))
    }

    fn create_file_with_content<P: AsRef<Path>>(file_path: P, content: &str) {
        if let Some(parent) = file_path.as_ref().parent() {
            create_dir_all(parent).unwrap_or_else(|e| panic!("create parent dir failed: {e:?}"));
        }
        let mut file =
            File::create(file_path).unwrap_or_else(|e| panic!("create file failed: {e:?}"));
        write!(file, "{content}").unwrap_or_else(|e| panic!("write file failed: {e:?}"));
    }

    fn create_tag_for_commit(repo: &Repository, tag: &str, commit_id: Oid) -> Oid {
        let commit = repo
            .find_commit(commit_id)
            .unwrap_or_else(|e| panic!("find commit object failed: {e:?}"));
        repo.tag_lightweight(tag, commit.as_object(), false)
            .unwrap_or_else(|e| panic!("create git tag failed: {e:?}"))
    }

    fn initialize_git_repo<P: AsRef<Path>>(path: P) -> Repository {
        Repository::init(path).unwrap_or_else(|e| panic!("initialize git repo failed: {e:?}"))
    }

    fn set_git_head_to_branch(repo: &Repository, branch: &str) {
        repo.set_head(&format!("refs/heads/{branch}"))
            .unwrap_or_else(|e| panic!("set git head failed: {e:?}"));
    }

    fn write_index_tree(repo: &Repository, index_entry: &[&Path]) -> Oid {
        let mut index = repo
            .index()
            .unwrap_or_else(|e| panic!("get git index failed: {e:?}"));
        for &path in index_entry.iter() {
            index
                .add_path(path)
                .unwrap_or_else(|e| panic!("add file to git index failed: {e:?}"));
        }
        index
            .write_tree()
            .unwrap_or_else(|e| panic!("write git index failed: {e:?}"))
    }

    #[test]
    fn test_gather_status() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let file_name = "README.md";
        create_file_with_content(path.join(file_name), "git + web = wit\n");

        let repo: GitRepository = repo.into();
        let entries = repo.gather_status().unwrap_or_else(|e| {
            panic!("gather_status in git repo {path:?} should not fail: {e:?}")
        });

        assert_eq!(entries.len(), 1);
        let item = &entries[0];
        assert_eq!(item.path.to_string(), file_name);
        assert_eq!(item.status.0, Status::WT_NEW);
        assert_eq!(item.status_bits, 128);
    }

    #[test]
    fn test_get_blob() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let file_name = "README.md";
        let content = "git + web = wit\n";
        create_file_with_content(path.join(file_name), content);

        set_git_head_to_branch(&repo, "main");
        let tree_id = write_index_tree(&repo, &[Path::new(file_name)]);
        commit_with_signature(
            &repo,
            tree_id,
            "Initial commit",
            "wit",
            "wit@example.com",
            None,
        );

        let repo: GitRepository = repo.into();
        repo.get_blob(GitOid(Oid::zero()))
            .expect_err("get_blob(all_zero_oid) in git repo {path:?} is expected to fail");

        let entries = repo
            .list_tree(file_name)
            .unwrap_or_else(|e| panic!("list_tree in git repo {path:?} should not fail: {e:?}"));

        assert_eq!(entries.len(), 1);
        let item = &entries[0];

        let blob = repo
            .get_blob(item.id.clone())
            .unwrap_or_else(|e| panic!("get_blob in git repo {path:?} should not fail: {e:?}"));

        if let GitBlobContent::Text(s) = blob.content {
            assert_eq!(s.to_string(), content)
        } else {
            panic!("blob content should be text")
        }
        assert!(!blob.is_binary);
        assert!(blob.short_id.len() >= 7);
        assert_eq!(blob.size, content.len());
    }

    #[test]
    fn test_list_branch() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let branch = "main";
        set_git_head_to_branch(&repo, branch);
        let tree_id = write_index_tree(&repo, &[]);
        commit_with_signature(
            &repo,
            tree_id,
            "Initial commit",
            "wit",
            "wit@example.com",
            None,
        );

        let repo: GitRepository = repo.into();
        let entries = repo
            .list_branch()
            .unwrap_or_else(|e| panic!("list_branch in git repo {path:?} should not fail: {e:?}"));

        assert_eq!(entries.len(), 1);
        let item = &entries[0];
        assert!(matches!(item.kind, GitBranchType::Local));
        assert_eq!(item.name.to_string(), format!("refs/heads/{branch}"));
        assert_eq!(item.shorthand.to_string(), branch);
        assert!(item.target_short.len() >= 7);
        assert!(item.upstream.is_none());
    }

    #[test]
    fn test_list_commit() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let user_name = "wit";
        let user_email = "wit@example.com";
        let now = OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .unwrap_or_else(|e| panic!("round current time failed: {e:?}"));
        let commit_message = "Initial commit";
        set_git_head_to_branch(&repo, "main");
        let tree_id = write_index_tree(&repo, &[]);
        commit_with_signature(
            &repo,
            tree_id,
            commit_message,
            user_name,
            user_email,
            Some(now.unix_timestamp()),
        );

        let repo: GitRepository = repo.into();
        let entries = repo
            .list_commit()
            .unwrap_or_else(|e| panic!("list_commit in git repo {path:?} should not fail: {e:?}"));

        assert_eq!(entries.len(), 1);
        let item = &entries[0];
        assert_eq!(item.author.email.to_string(), user_email);
        assert_eq!(item.author.name.to_string(), user_name);
        assert_eq!(item.committer.email.to_string(), user_email);
        assert_eq!(item.committer.name.to_string(), user_name);
        assert_eq!(item.message.to_string(), commit_message);
        assert!(item.short_id.len() >= 7);
        assert_eq!(item.time, now);
    }

    #[test]
    fn test_list_index() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let file_list = [
            "dir01/dir11/file1",
            "dir01/dir12/dir21/file1",
            "dir01/dir12/dir22/file1",
            "dir01/dir12/dir23/file1",
            "dir01/dir12/dir23/file2",
            "dir01/dir12/dir23/file3",
            "dir01/dir12/file1",
            "dir01/file1",
            "dir02/dir11/file1",
            "dir02/dir12/file1",
            "dir02/file1",
            "file1",
            "file2",
            "file3",
        ];
        for &file_name in file_list.iter() {
            create_file_with_content(path.join(file_name), "");
        }

        set_git_head_to_branch(&repo, "main");
        write_index_tree(&repo, &file_list.map(Path::new));

        let repo: GitRepository = repo.into();
        let sample = [
            (Default::default(), 5, Default::default()),
            ("dir01/dir12/dir23", 3, "dir01/dir12/dir23/"),
            ("dir01/dir12/dir23/", 3, "dir01/dir12/dir23/"),
            ("dir01/file1", 1, "dir01/"),
        ];
        for (index, count, root) in sample.into_iter() {
            let entries = repo.list_index(index).unwrap_or_else(|e| {
                panic!("list_index in git repo {path:?} should not fail: {e:?}")
            });
            assert_eq!(entries.len(), count);
            for item in entries.iter() {
                let full_path = match item {
                    GitIndex::Directory(d) => &d.path,
                    GitIndex::Entry(e) => {
                        assert_eq!(e.file_size, 0);
                        &e.path
                    }
                };
                if !index.is_empty() {
                    assert!(
                        full_path.0.starts_with(root),
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
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let branch = "main";
        set_git_head_to_branch(&repo, branch);
        let tree_id = write_index_tree(&repo, &[]);
        commit_with_signature(
            &repo,
            tree_id,
            "Initial commit",
            "wit",
            "wit@example.com",
            None,
        );

        let repo: GitRepository = repo.into();
        let entries = repo.list_reference().unwrap_or_else(|e| {
            panic!("list_reference in git repo {path:?} should not fail: {e:?}")
        });

        assert_eq!(entries.len(), 1);
        let item = &entries[0];
        assert!(matches!(item.kind, Some(model::GitReferenceType::Direct)));
        assert_eq!(item.name.to_string(), format!("refs/heads/{branch}"));
        assert_eq!(item.shorthand.to_string(), branch);
        assert!(item.target_short.len() >= 7);
    }

    #[test]
    fn test_list_remote() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let remote_name = "example";
        let remote_url = "https://example.com/git/wit.git";
        repo.remote(remote_name, remote_url)
            .unwrap_or_else(|e| panic!("add git remote failed: {e:?}"));

        let repo: GitRepository = repo.into();
        let entries = repo
            .list_remote()
            .unwrap_or_else(|e| panic!("list_remote in git repo {path:?} should not fail: {e:?}"));

        assert_eq!(entries.len(), 1);
        let item = &entries[0];
        assert_eq!(item.name.to_string(), remote_name);
        assert_eq!(item.url.to_string(), remote_url);
    }

    #[test]
    fn test_list_tag() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let tag = "v0.0.0";
        set_git_head_to_branch(&repo, "main");
        let tree_id = write_index_tree(&repo, &[]);
        let commit_id = commit_with_signature(
            &repo,
            tree_id,
            "Initial commit",
            "wit",
            "wit@example.com",
            None,
        );
        create_tag_for_commit(&repo, tag, commit_id);

        let repo: GitRepository = repo.into();
        let entries = repo
            .list_tag()
            .unwrap_or_else(|e| panic!("list_tag in git repo {path:?} should not fail: {e:?}"));

        assert_eq!(entries.len(), 1);
        let item = &entries[0];
        assert_eq!(item.name.to_string(), format!("refs/tags/{tag}"));
        assert_eq!(item.shorthand.to_string(), tag);
        assert!(item.target_short.len() >= 7);
    }

    #[test]
    fn test_list_tree() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();
        let repo = initialize_git_repo(path);

        let file_list = [
            "dir01/dir11/file1",
            "dir01/dir12/dir21/file1",
            "dir01/dir12/dir22/file1",
            "dir01/dir12/dir23/file1",
            "dir01/dir12/dir23/file2",
            "dir01/dir12/dir23/file3",
            "dir01/dir12/file1",
            "dir01/file1",
            "dir02/dir11/file1",
            "dir02/dir12/file1",
            "dir02/file1",
            "file1",
            "file2",
            "file3",
        ];
        for &file_name in file_list.iter() {
            create_file_with_content(path.join(file_name), "");
        }

        set_git_head_to_branch(&repo, "main");
        let tree_id = write_index_tree(&repo, &file_list.map(Path::new));
        commit_with_signature(
            &repo,
            tree_id,
            "Initial commit",
            "wit",
            "wit@example.com",
            None,
        );

        let repo: GitRepository = repo.into();
        let sample = [
            (Default::default(), 5, Default::default()),
            ("dir01/dir12/dir23", 3, "dir01/dir12/dir23/"),
            ("dir01/dir12/dir23/", 3, "dir01/dir12/dir23/"),
            ("dir01/file1", 1, "dir01/"),
        ];
        for (tree, count, root) in sample.into_iter() {
            let entries = repo.list_tree(tree).unwrap_or_else(|e| {
                panic!("list_tree in git repo {path:?} should not fail: {e:?}")
            });
            assert_eq!(entries.len(), count);
            for item in entries.iter() {
                assert!(item.short_id.len() >= 7);
                assert_eq!(item.root, root, "unexpected root of tree entry");
            }
        }
    }

    #[test]
    fn test_open_repository() {
        let dir = tempdir().unwrap_or_else(|e| panic!("create tempdir failed: {e:?}"));
        let path = dir.path();

        GitRepository::open(path)
            .map_err(|e| {
                if let GitError::RepositoryNotFound(p) = e {
                    assert_eq!(p.file_name(), path.file_name());
                } else {
                    panic!("{e}");
                }
            })
            .expect_err("{path:?} should not be a valid git repo");

        initialize_git_repo(path);

        GitRepository::open(path)
            .unwrap_or_else(|e| panic!("{path:?} should be a valid git repo: {e:?}"));
    }
}
