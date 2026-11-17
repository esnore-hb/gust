use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::objects::{Blob, Hash, Index};
use crate::store::{hash_blob, ObjectStore};

const META_DIR: &str = ".gust";
const IGNORED: [&str; 3] = [META_DIR, ".git", "target"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Untracked,
    Staged,
    Modified,
    Deleted,
}

pub struct WorkingTree {
    root: PathBuf,
    store: ObjectStore,
    index: Index,
    files: BTreeMap<String, FileState>,
}

impl WorkingTree {
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        let meta = root.join(META_DIR);
        fs::create_dir_all(meta.join("objects"))?;

        let mut tree = Self {
            store: ObjectStore::new(&meta),
            index: Index::default(),
            files: BTreeMap::new(),
            root,
        };
        tree.load_index()?;
        tree.refresh()?;
        Ok(tree)
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn files(&self) -> &BTreeMap<String, FileState> {
        &self.files
    }

    pub fn state_of(&self, path: &str) -> Option<FileState> {
        self.files.get(path).copied()
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        let mut on_disk = Vec::new();
        Self::scan(&self.root, &self.root, &mut on_disk)?;

        let mut files = BTreeMap::new();
        for path in &on_disk {
            let state = match self.index.entries.get(path) {
                None => FileState::Untracked,
                Some(staged) if *staged == self.hash_working_file(path)? => FileState::Staged,
                Some(_) => FileState::Modified,
            };
            files.insert(path.clone(), state);
        }
        for path in self.index.entries.keys() {
            files.entry(path.clone()).or_insert(FileState::Deleted);
        }
        self.files = files;
        Ok(())
    }

    pub fn add(&mut self, path: &str) -> io::Result<Hash> {
        let content = fs::read(self.root.join(path))?;
        let hash = self.store.write_blob(&Blob { content })?;
        self.index.entries.insert(path.to_string(), hash.clone());
        self.save_index()?;
        self.refresh()?;
        Ok(hash)
    }

    pub fn add_all(&mut self) -> io::Result<()> {
        let pending: Vec<(String, FileState)> = self
            .files
            .iter()
            .filter(|(_, s)| **s != FileState::Staged)
            .map(|(p, s)| (p.clone(), *s))
            .collect();
        for (path, state) in pending {
            if state == FileState::Deleted {
                self.index.entries.remove(&path);
            } else {
                let content = fs::read(self.root.join(&path))?;
                let hash = self.store.write_blob(&Blob { content })?;
                self.index.entries.insert(path, hash);
            }
        }
        self.save_index()?;
        self.refresh()
    }

    pub fn unstage(&mut self, path: &str) -> io::Result<()> {
        if self.index.entries.remove(path).is_none() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("{path} is not staged")));
        }
        self.save_index()?;
        self.refresh()
    }

    pub fn restore(&mut self, path: &str) -> io::Result<()> {
        let hash = self.index.entries.get(path).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("{path} is not tracked"))
        })?;
        let blob = self.store.read_blob(hash)?;
        let target = self.root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, blob.content)?;
        self.refresh()
    }

    fn hash_working_file(&self, path: &str) -> io::Result<Hash> {
        let content = fs::read(self.root.join(path))?;
        Ok(hash_blob(&Blob { content }))
    }

    fn scan(root: &Path, dir: &Path, out: &mut Vec<String>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if IGNORED.contains(&entry.file_name().to_string_lossy().as_ref()) {
                continue;
            }
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                Self::scan(root, &path, out)?;
            } else if kind.is_file() {
                let rel = path.strip_prefix(root).expect("path is under root");
                let rel: Vec<_> = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect();
                out.push(rel.join("/"));
            }
        }
        Ok(())
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(META_DIR).join("index")
    }

    fn save_index(&self) -> io::Result<()> {
        let data: String = self
            .index
            .entries
            .iter()
            .map(|(path, hash)| format!("{hash} {path}\n"))
            .collect();
        fs::write(self.index_path(), data)
    }

    fn load_index(&mut self) -> io::Result<()> {
        let data = match fs::read_to_string(self.index_path()) {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        for line in data.lines() {
            if let Some((hash, path)) = line.split_once(' ') {
                self.index.entries.insert(path.to_string(), hash.to_string());
            }
        }
        Ok(())
    }
}