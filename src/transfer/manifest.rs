use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ManifestFile {
    pub path: String,
    pub size: u64,
    pub hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifest {
    pub files: Vec<ManifestFile>,
    pub root_hash: [u8; 32],
}

pub fn build_manifest(root: &Path) -> Result<Manifest, ManifestError> {
    let mut files = Vec::new();
    let mut all_hashes = Vec::new();

    let mut entries: Vec<_> = walkdir::WalkDir::new(root)
        .sort_by(|a, b| a.path().cmp(b.path()))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    entries.sort_by(|a, b| a.path().cmp(b.path()));

    for entry in &entries {
        let abs_path = entry.path();
        let rel_path = abs_path
            .strip_prefix(root)
            .map_err(|_| ManifestError::StripPrefix(abs_path.to_path_buf()))?;
        let size = std::fs::metadata(abs_path)
            .map_err(ManifestError::Io)?
            .len();

        let hash = compute_file_hash(abs_path)?;

        files.push(ManifestFile {
            path: rel_path.to_string_lossy().to_string(),
            size,
            hash,
        });
        all_hashes.extend_from_slice(&hash);
    }

    let root_hash = *blake3::hash(&all_hashes).as_bytes();

    Ok(Manifest { files, root_hash })
}

fn compute_file_hash(path: &Path) -> Result<[u8; 32], ManifestError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(ManifestError::Io)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(ManifestError::Io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to strip prefix from path: {0}")]
    StripPrefix(std::path::PathBuf),

    #[error("walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_manifest_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = build_manifest(dir.path()).unwrap();
        assert!(manifest.files.is_empty());
        assert_eq!(manifest.root_hash, *blake3::hash(&[]).as_bytes());
    }

    #[test]
    fn test_build_manifest_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();

        let manifest = build_manifest(dir.path()).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].path, "test.txt");
        assert_eq!(manifest.files[0].size, 5);
    }

    #[test]
    fn test_build_manifest_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"aaa").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"bbbb").unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("c.txt"), b"cc").unwrap();

        let manifest = build_manifest(dir.path()).unwrap();
        assert_eq!(manifest.files.len(), 3);
        assert_eq!(manifest.files[0].path, "a.txt");
        assert_eq!(manifest.files[1].path, "b.txt");
        assert_eq!(manifest.files[2].path, "sub/c.txt");
    }

    #[test]
    fn test_manifest_root_hash_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), b"data").unwrap();
        let m1 = build_manifest(dir.path()).unwrap();
        let m2 = build_manifest(dir.path()).unwrap();
        assert_eq!(m1.root_hash, m2.root_hash);
    }
}
