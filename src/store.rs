use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};

use crate::objects::{Blob, Hash};

pub fn serialize_blob(blob: &Blob) -> Vec<u8> {
    let mut data = format!("blob {}\0", blob.content.len()).into_bytes();
    data.extend_from_slice(&blob.content);
    data
}

pub fn hash_blob(blob: &Blob) -> Hash {
    let mut hasher = Sha1::new();
    hasher.update(serialize_blob(blob));
    hex::encode(hasher.finalize().as_slice())
}

pub struct ObjectStore {
    objects_dir: PathBuf,
}

impl ObjectStore {
    pub fn new(gust_dir: &Path) -> Self {
        Self {
            objects_dir: gust_dir.join("objects"),
        }
    }

    fn path_for(&self, hash: &str) -> PathBuf {
        let (dir, file) = hash.split_at(2);
        self.objects_dir.join(dir).join(file)
    }

    /// Stores the blob (if not already present) and returns its hash.
    pub fn write_blob(&self, blob: &Blob) -> io::Result<Hash> {
        let hash = hash_blob(blob);
        let path = self.path_for(&hash);
        if path.exists() {
            return Ok(hash);
        }
        fs::create_dir_all(path.parent().expect("object path has a parent"))?;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serialize_blob(blob))?;
        fs::write(path, encoder.finish()?)?;
        Ok(hash)
    }

    pub fn read_blob(&self, hash: &str) -> io::Result<Blob> {
        let compressed = fs::read(self.path_for(hash))?;
        let mut raw = Vec::new();
        ZlibDecoder::new(&compressed[..]).read_to_end(&mut raw)?;

        let invalid =
            |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_string());
        let nul = raw
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| invalid("missing header terminator"))?;
        let header =
            std::str::from_utf8(&raw[..nul])
            .map_err(|_| invalid("header is not UTF-8"))?;
        let len: usize = header
            .strip_prefix("blob ")
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| invalid("not a blob object"))?;

        let content = raw[nul + 1..].to_vec();
        if content.len() != len {
            return Err(invalid("blob length mismatch"));
        }
        Ok(Blob { content })
    }
}
