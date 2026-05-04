// Minimal Electron asar archive reader.
//
// asar layout (verified against /Applications/Codex.app/...):
//   bytes 0..4        : u32 = inner pickle size (always 4)
//   bytes 4..8        : u32 = total header size (= rest-of-header byte count)
//   bytes 8..12       : u32 = string-pickle payload size (= header_size - 4)
//   bytes 12..16      : u32 = JSON string length
//   bytes 16..16+len  : JSON string (UTF-8)
//   bytes 8+header_size..  : concatenated file payloads
//
// JSON header lists files as { "files": { name: { files | offset, size } } }
// where `offset` and `size` are strings (yes, strings) of decimal byte counts
// relative to the start of the data section.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct AsarHeader {
    pub files: HashMap<String, AsarEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AsarEntry {
    Dir {
        files: HashMap<String, AsarEntry>,
        #[serde(default, rename = "unpacked")]
        _unpacked: Option<bool>,
    },
    File {
        size: u64,
        // `offset` is missing for files that live in app.asar.unpacked/.
        #[serde(default)]
        offset: Option<String>,
        #[serde(default, rename = "executable")]
        _executable: Option<bool>,
        #[serde(default, rename = "unpacked")]
        _unpacked: Option<bool>,
        #[serde(default, rename = "integrity")]
        _integrity: Option<serde_json::Value>,
    },
}

pub struct AsarReader {
    file: File,
    pub header: AsarHeader,
    pub data_offset: u64,
}

impl AsarReader {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buf4 = [0u8; 4];

        // Pickle: read u32 = size of next pickle (always 4 — it just holds a u32).
        file.read_exact(&mut buf4)?;
        // Pickle: read u32 = header_size value.
        file.read_exact(&mut buf4)?;
        let header_size = u32::from_le_bytes(buf4) as usize;

        let mut header_buf = vec![0u8; header_size];
        file.read_exact(&mut header_buf)?;

        // header_buf layout:
        //   [0..4]  inner-pickle size (= header_size - 4)
        //   [4..8]  JSON string length
        //   [8..]   JSON bytes (padded to 4 alignment)
        if header_buf.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "asar header too short",
            ));
        }
        let json_len = u32::from_le_bytes([
            header_buf[4],
            header_buf[5],
            header_buf[6],
            header_buf[7],
        ]) as usize;
        if 8 + json_len > header_buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "asar JSON length out of range",
            ));
        }
        let json_bytes = &header_buf[8..8 + json_len];
        let header: AsarHeader = serde_json::from_slice(json_bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        let data_offset = 8 + header_size as u64;
        Ok(Self {
            file,
            header,
            data_offset,
        })
    }

    pub fn read_file(&mut self, path: &str) -> std::io::Result<Vec<u8>> {
        let entry = lookup(&self.header.files, path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asar entry not found: {}", path),
            )
        })?;
        let (size, offset) = match entry {
            AsarEntry::File {
                size,
                offset: Some(offset),
                ..
            } => {
                let off: u64 = offset.parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad offset")
                })?;
                (*size, off)
            }
            AsarEntry::File { offset: None, .. } => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "asar entry is unpacked (lives in app.asar.unpacked/)",
                ))
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "asar entry is not a file",
                ))
            }
        };
        self.file
            .seek(SeekFrom::Start(self.data_offset + offset))?;
        let mut buf = vec![0u8; size as usize];
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }
}

fn lookup<'a>(
    tree: &'a HashMap<String, AsarEntry>,
    path: &str,
) -> Option<&'a AsarEntry> {
    let mut current = tree;
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for (i, part) in parts.iter().enumerate() {
        let entry = current.get(*part)?;
        if i + 1 == parts.len() {
            return Some(entry);
        }
        match entry {
            AsarEntry::Dir { files, .. } => {
                current = files;
            }
            _ => return None,
        }
    }
    None
}

/// Walks the archive collecting every file path (slash-separated).
pub fn list_files(header: &AsarHeader) -> Vec<String> {
    let mut out = vec![];
    walk("", &header.files, &mut out);
    out
}

fn walk(prefix: &str, tree: &HashMap<String, AsarEntry>, out: &mut Vec<String>) {
    for (name, entry) in tree {
        let full = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };
        match entry {
            AsarEntry::Dir { files, .. } => walk(&full, files, out),
            AsarEntry::File { .. } => out.push(full),
        }
    }
}
