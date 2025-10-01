use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Metadata for a file entry surfaced to the UI pickers.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
    pub size: u64,
}

pub fn list_files_with_extension(dir: impl AsRef<Path>, extension: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    let dir_path = dir.as_ref();

    if let Ok(read_dir) = fs::read_dir(dir_path) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some(extension) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            };

            let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
            let size = metadata.len();
            let Some(name) = path
                .file_name()
                .and_then(|segment| segment.to_str())
                .map(|s| s.to_string())
            else {
                continue;
            };

            entries.push(FileEntry {
                name,
                path,
                modified,
                size,
            });
        }
    }

    entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    entries
}

pub fn list_csv_files(dir: impl AsRef<Path>) -> Vec<FileEntry> {
    list_files_with_extension(dir, "csv")
}

pub fn list_json_files(dir: impl AsRef<Path>) -> Vec<FileEntry> {
    list_files_with_extension(dir, "json")
}
