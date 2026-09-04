use md5::{Digest, Md5};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::core::errors::{HashRenameError, Result};

const BUFFER_SIZE: usize = 4096;

/// Trait for file hashing - allows future extension to SHA-256 etc.
pub trait Hasher: Send + Sync {
    fn hash(&self, path: &Path) -> Result<String>;
}

/// MD5 hasher implementation using streaming reads.
pub struct Md5Hasher;

impl Md5Hasher {
    pub fn new() -> Self {
        Md5Hasher
    }
}

impl Default for Md5Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Md5Hasher {
    fn hash(&self, path: &Path) -> Result<String> {
        let mut file = File::open(path).map_err(|e| HashRenameError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut hasher = Md5::new();
        let mut buffer = [0u8; BUFFER_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| HashRenameError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
}

// Simple hex encoding without external dependency
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hash_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let hasher = Md5Hasher::new();
        let hash = hasher.hash(&path).unwrap();
        // MD5 of empty string
        assert_eq!(hash, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_hash_small_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("hello.txt");
        fs::write(&path, "hello world").unwrap();

        let hasher = Md5Hasher::new();
        let hash = hasher.hash(&path).unwrap();
        assert_eq!(hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }

    #[test]
    fn test_hash_deterministic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.bin");
        fs::write(&path, b"\x00\x01\x02\x03\x04").unwrap();

        let hasher = Md5Hasher::new();
        let hash1 = hasher.hash(&path).unwrap();
        let hash2 = hasher.hash(&path).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_binary_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("binary.bin");
        let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
        fs::write(&path, &data).unwrap();

        let hasher = Md5Hasher::new();
        let hash = hasher.hash(&path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32); // MD5 hex is always 32 chars
    }

    #[test]
    fn test_hash_nonexistent_file() {
        let hasher = Md5Hasher::new();
        let result = hasher.hash(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex::encode([0u8, 1u8, 255u8]), "0001ff");
        assert_eq!(hex::encode(b"hello"), "68656c6c6f");
    }
}
