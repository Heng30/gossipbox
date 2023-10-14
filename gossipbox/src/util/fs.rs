use std::fs;
use std::io;
use std::path::Path;

const K: u64 = 1024;
const M: u64 = K * 1024;
const G: u64 = M * 1024;
const T: u64 = G * 1024;

pub fn dir_size(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut total_size: u64 = 0;
    let metadata = fs::metadata(path)?;

    if metadata.is_dir() {
        for entry in fs::read_dir(path)?.flatten() {
            let size = entry.metadata()?.len();
            total_size += size;
        }
    } else {
        total_size += metadata.len();
    }

    Ok(format!("{:.2}M", total_size as f64 / 1024.0 / 1024.0))
}

pub fn file_size(path: &Path) -> u64 {
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_file() {
                metadata.len()
            } else {
                0
            }
        }
        _ => 0,
    }
}

pub fn file_size_string(size: u64) -> String {
    if size > T {
        format!("{:.2}T", size / T)
    } else if size > G {
        format!("{:.2}G", size / G)
    } else if size > M {
        format!("{:.2}M", size / M)
    } else if size > K {
        format!("{:.2}K", size / K)
    } else {
        format!("{}B", size)
    }
}

pub fn remove_dir_files(path: &str) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn file_exist(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(md) => md.is_file(),
        _ => false,
    }
}
