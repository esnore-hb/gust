use std::collections::BTreeMap;
use chrono::{DateTime, Utc};

pub type Hash = String;
pub type Timestamp = DateTime<Utc>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Blob,
    Tree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub kind: EntryKind,
    pub name: String,
    pub hash: Hash,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub tree: Hash,
    pub parent: Option<Hash>,
    pub message: String,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
    pub entries: BTreeMap<String, Hash>,
}