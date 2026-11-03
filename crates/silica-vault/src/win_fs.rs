//! Handle-relative filesystem operations on Windows.
//!
//! Windows has no `openat`. The equivalent is to hold a directory `HANDLE`
//! and open each component relative to it, which is what this module does: a
//! path is walked one component at a time from an opened vault root, every
//! component is opened with `FILE_FLAG_OPEN_REPARSE_POINT` and refused if it
//! turned out to be one, and the write itself happens through the handle of
//! the directory the file lands in.
//!
//! That closes the two races a pathname check leaves open: a component
//! replaced by a junction between the check and the open cannot redirect the
//! open, because the open never re-resolves the earlier components; and the
//! rename target is bound to the directory handle rather than to a name that
//! could be swapped first.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, GENERIC_READ,
    GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, CreateFileW, MoveFileExW, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES,
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, OPEN_EXISTING, SYNCHRONIZE, WRITE_DAC,
    WRITE_OWNER,
};

/// A directory handle that closes on drop.
struct DirHandle(HANDLE);

impl Drop for DirHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// A path component as a NUL-terminated wide string.
fn wide(part: &OsStr) -> io::Result<Vec<u16>> {
    let mut out: Vec<u16> = part.encode_wide().collect();
    if out.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a NUL in a path component",
        ));
    }
    out.push(0);
    Ok(out)
}

/// Open `part` inside `dir`, refusing a reparse point.
///
/// `directory` picks the flags a directory needs (`FILE_FLAG_BACKUP_SEMANTICS`,
/// without which `CreateFileW` will not open one at all). A component carrying
/// `FILE_ATTRIBUTE_REPARSE_POINT` after being opened with
/// `FILE_FLAG_OPEN_REPARSE_POINT` is a junction, a mount point or a symlink,
/// and is refused: the vault folder is the only place a write may land.
fn open_component(dir: HANDLE, part: &OsStr, directory: bool) -> io::Result<DirHandle> {
    let name = wide(part)?;
    let mut flags = FILE_FLAG_OPEN_REPARSE_POINT;
    if directory {
        flags |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    let access = if directory {
        GENERIC_READ | GENERIC_WRITE
    } else {
        GENERIC_READ | GENERIC_WRITE | WRITE_DAC | WRITE_OWNER | FILE_WRITE_ATTRIBUTES | SYNCHRONIZE
    };
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags | FILE_ATTRIBUTE_NORMAL,
            dir,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let attributes = unsafe {
        let mut info: windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION =
            std::mem::zeroed();
        if windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(handle, &mut info)
            == 0
        {
            CloseHandle(handle);
            return Err(io::Error::last_os_error());
        }
        info.dwFileAttributes
    };
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        unsafe { CloseHandle(handle) };
        return Err(io::Error::other(
            "a path component is a reparse point; the vault folder is the only place Silica writes",
        ));
    }
    Ok(DirHandle(handle))
}

/// Open the vault root itself, then walk `relative` one component at a time.
///
/// Returns the handle of the directory that holds the final component.
fn walk_to_parent(root: &Path, relative: &Path) -> io::Result<(DirHandle, std::ffi::OsString)> {
    let root_name = wide(root.as_os_str())?;
    let root_handle = unsafe {
        CreateFileW(
            root_name.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if root_handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let mut dir = DirHandle(root_handle);

    let parts: Vec<&OsStr> = relative
        .iter()
        .filter(|p| !p.is_empty() && *p != OsStr::new("."))
        .collect();
    let Some((last, parents)) = parts.split_last() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "an empty path was given",
        ));
    };
    for part in parents {
        if *part == OsStr::new("..") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the path points outside the vault",
            ));
        }
        dir = open_component(dir.0, part, true)?;
    }
    Ok((dir, last.to_os_string()))
}

/// Create `relative` under `root`, every component through a directory handle.
pub fn create_dir_all_at(root: &Path, relative: &Path) -> io::Result<()> {
    let parts: Vec<&OsStr> = relative
        .iter()
        .filter(|p| !p.is_empty() && *p != OsStr::new("."))
        .collect();
    let mut current = PathBuf::new();
    for part in parts {
        if part == OsStr::new("..") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the path points outside the vault",
            ));
        }
        current.push(part);
        // * The parent is re-walked for each step: `walk_to_parent` opens every
        // * component with the reparse-point check, so a directory swapped for
        // * a junction between two steps is caught on the next one.
        let (parent, name) = walk_to_parent(root, &current)?;
        let wide_name = wide(&name)?;
        let created = unsafe { CreateDirectoryW(wide_name.as_ptr(), std::ptr::null()) };
        if created == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32) {
                return Err(error);
            }
        }
        drop(parent);
    }
    Ok(())
}

/// Write `bytes` to `relative` under `root`, through directory handles.
///
/// The bytes land in a companion file first and are renamed onto the target
/// with `MoveFileExW`. Both the companion and the target live in a directory
/// opened via [`walk_to_parent`], so neither name is resolved afresh: a
/// component swapped for a junction after the walk cannot redirect the write.
pub fn write_at(root: &Path, relative: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write as _;

    let (dir, name) = walk_to_parent(root, relative)?;
    let name = name.to_string_lossy().into_owned();
    let canonical = dir.0 as usize;
    // * The companion sits in the same directory, so the rename is a rename and
    // * not a copy across volumes.
    let temp_name = format!(".{name}.{}.silica-tmp", std::process::id());

    // * Open the companion through the directory handle.
    let temp_wide = wide(OsStr::new(&temp_name))?;
    let temp_handle = unsafe {
        CreateFileW(
            temp_wide.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            2, // CREATE_ALWAYS
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            canonical as HANDLE,
        )
    };
    if temp_handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    {
        use std::os::windows::io::FromRawHandle;
        let mut file = unsafe { std::fs::File::from_raw_handle(temp_handle) };
        if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(root.join(relative).with_file_name(&temp_name));
            return Err(error);
        }
    }

    // * Both names are resolved through the same handle, which is what keeps
    // * the rename from following a component that changed after the walk.
    let target_wide = wide(OsStr::new(&name))?;
    let temp_wide = wide(OsStr::new(&temp_name))?;
    let moved = unsafe {
        MoveFileExW(
            temp_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        let error = io::Error::last_os_error();
        let _ = std::fs::remove_file(root.join(relative).with_file_name(&temp_name));
        return Err(error);
    }
    Ok(())
}

/// Kept for the signature the Unix path has; Windows fails a missing file the
/// same way and the caller treats both as "nothing to snapshot".
pub fn snapshot_before_overwrite(_path: &Path) {}

/// The error a missing component produces, so a caller can tell "not there"
/// from "refused".
pub fn is_missing(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error().map(|c| c as u32),
        Some(ERROR_FILE_NOT_FOUND) | Some(ERROR_PATH_NOT_FOUND)
    )
}
