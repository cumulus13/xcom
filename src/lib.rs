// File: src\lib.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-01-24
// Description: xcom - Windows File Operations Utility
// License: MIT

//! xcom - Windows File Operations Utility
//!
//! A professional utility for performing file copy and move operations
//! using Windows Shell APIs with comprehensive logging.

use chrono::Local;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use winapi::shared::windef::HWND;
#[cfg(windows)]
use winapi::um::shellapi::{SHFileOperationW, SHFILEOPSTRUCTW, FO_COPY, FO_MOVE, FOF_NOCONFIRMMKDIR};

const LOG_FILENAME: &str = "xcom.log";

/// Gets the path to the log file (always in exe directory)
pub fn get_log_path() -> PathBuf {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join(LOG_FILENAME);
        }
    }
    
    // Fallback: current directory
    PathBuf::from(LOG_FILENAME)
}

/// Writes a log entry with timestamp
pub fn logs(data: &str) {
    let log_path = get_log_path();
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let timestamp = Local::now().format("%d-%m-%Y %H:%M:%S");
        let _ = writeln!(file, "{} {}", timestamp, data);
    }
}

#[cfg(windows)]
fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn to_double_null_wide(paths: &[PathBuf]) -> Vec<u16> {
    let mut result = Vec::new();
    for path in paths {
        let path_str = path.to_string_lossy();
        result.extend(OsStr::new(path_str.as_ref()).encode_wide());
        result.push(0);
    }
    result.push(0); // Double null terminator
    result
}

/// File operation type
#[derive(Debug, Clone, Copy)]
pub enum FileOperation {
    Copy,
    Move,
}

impl FileOperation {
    fn as_str(&self) -> &'static str {
        match self {
            FileOperation::Copy => "COPY",
            FileOperation::Move => "MOVE",
        }
    }
}

/// Performs a Windows shell file operation (copy or move)
///
/// # Arguments
///
/// * `sources` - Vector of source file paths
/// * `dest` - Destination directory path
/// * `operation` - Type of operation (Copy or Move)
///
/// # Returns
///
/// * `Ok(true)` - Operation completed successfully
/// * `Ok(false)` - Operation was aborted by user
/// * `Err(String)` - Operation failed with error message
#[cfg(windows)]
pub fn win32_shell_operation(
    sources: Vec<PathBuf>,
    dest: &Path,
    operation: FileOperation,
) -> Result<bool, String> {
    unsafe {
        let src_wide = to_double_null_wide(&sources);
        let dest_wide = to_wide_string(&dest.to_string_lossy());

        let op_type = match operation {
            FileOperation::Copy => FO_COPY,
            FileOperation::Move => FO_MOVE,
        };

        let mut file_op = SHFILEOPSTRUCTW {
            hwnd: std::ptr::null_mut() as HWND,
            wFunc: op_type as u32,
            pFrom: src_wide.as_ptr(),
            pTo: dest_wide.as_ptr(),
            fFlags: FOF_NOCONFIRMMKDIR,
            fAnyOperationsAborted: 0,
            hNameMappings: std::ptr::null_mut(),
            lpszProgressTitle: std::ptr::null(),
        };

        let result = SHFileOperationW(&mut file_op);

        if file_op.fAnyOperationsAborted != 0 {
            return Ok(false);
        }

        if result != 0 {
            let error_msg = format!("SHFileOperation failed: 0x{:08x}", result);
            logs(&error_msg);
            return Err(error_msg);
        }

        Ok(true)
    }
}

#[cfg(not(windows))]
pub fn win32_shell_operation(
    _sources: Vec<PathBuf>,
    _dest: &Path,
    _operation: FileOperation,
) -> Result<bool, String> {
    Err("This utility is only supported on Windows".to_string())
}

/// Performs file operation on directory contents
///
/// # Arguments
///
/// * `path` - Source directory path (None = current directory)
/// * `dest` - Destination directory path
/// * `recursive` - Whether to include subdirectories recursively
/// * `operation` - Type of operation (Copy or Move)
pub fn perform_operation(
    path: Option<&Path>,
    dest: &Path,
    recursive: bool,
    operation: FileOperation,
) -> Result<(), String> {
    let source_path = path.unwrap_or_else(|| Path::new("."));
    
    let op_str = operation.as_str();
    logs(&format!(
        "{}: Path: {:?}, Dest: {:?}, Recursive: {}",
        op_str, source_path, dest, recursive
    ));

    if !recursive {
        let list_dir: Vec<PathBuf> = std::fs::read_dir(source_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect();

        let files_str: Vec<String> = list_dir
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let log_msg = format!(
            "{}: \"{}\" --> \"{}\"",
            op_str,
            files_str.join("; "),
            dest.display()
        );
        logs(&log_msg);

        match win32_shell_operation(list_dir, dest, operation) {
            Ok(_) => Ok(()),
            Err(e) => {
                logs(&e);
                Err(e)
            }
        }
    } else {
        let mut list_dir = Vec::new();

        for entry in WalkDir::new(source_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                list_dir.push(entry.path().to_path_buf());
            }
        }

        let files_str: Vec<String> = list_dir
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let log_msg = format!(
            "{}: \"{}\" --> \"{}\"",
            op_str,
            files_str.join("; "),
            dest.display()
        );
        logs(&log_msg);

        match win32_shell_operation(list_dir, dest, operation) {
            Ok(_) => Ok(()),
            Err(e) => {
                logs(&e);
                Err(e)
            }
        }
    }
}

/// Processes command-line arguments and performs operations
///
/// # Arguments
///
/// * `sources` - Vector of source paths/patterns
/// * `dest` - Destination directory path
/// * `operation` - Type of operation (Copy or Move)
pub fn process_sources(
    sources: Vec<String>,
    dest: &Path,
    operation: FileOperation,
) -> Result<(), String> {
    let mut all_paths = Vec::new();
    let mut has_wildcard = false;

    for source in &sources {
        if source == "*" {
            has_wildcard = true;
            // Get all files in current directory
            let list_dir: Vec<PathBuf> = std::fs::read_dir(".")
                .map_err(|e| format!("Failed to read directory: {}", e))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .collect();
            all_paths.extend(list_dir);
        } else if source.ends_with('*') {
            has_wildcard = true;
            let path = if source.len() > 1 {
                Path::new(&source[..source.len() - 1])
            } else {
                Path::new(".")
            };
            // Get all files in specified directory
            let list_dir: Vec<PathBuf> = std::fs::read_dir(path)
                .map_err(|e| format!("Failed to read directory: {}", e))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .collect();
            all_paths.extend(list_dir);
        } else {
            all_paths.push(PathBuf::from(source));
        }
    }

    // Process ALL files in ONE operation, just like Python!
    let files_str: Vec<String> = all_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let log_msg = format!(
        "{}: \"{}\" --> \"{}\"",
        operation.as_str(),
        files_str.join("; "),
        dest.display()
    );
    logs(&log_msg);

    match win32_shell_operation(all_paths, dest, operation) {
        Ok(_) => Ok(()),
        Err(e) => {
            logs(&e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_path() {
        let path = get_log_path();
        assert!(path.to_string_lossy().contains("xcom.log"));
    }

    #[test]
    fn test_file_operation_str() {
        assert_eq!(FileOperation::Copy.as_str(), "COPY");
        assert_eq!(FileOperation::Move.as_str(), "MOVE");
    }
}
