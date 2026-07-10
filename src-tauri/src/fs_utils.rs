use crate::error::{AppError, AppResult};
use rand::RngCore;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PRIVATE_FILE_MODE: u32 = 0o600;
pub const PUBLIC_FILE_MODE: u32 = 0o644;

fn temporary_path(path: &Path) -> AppResult<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Custom("目标文件缺少父目录。".to_string()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Custom("目标文件名无效。".to_string()))?
        .to_string_lossy();

    for _ in 0..16 {
        let suffix = rand::thread_rng().next_u64();
        let candidate = parent.join(format!(".{}.tmp-{:016x}", file_name, suffix));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::Custom("无法分配安全的临时文件名。".to_string()))
}

pub fn atomic_write(path: &Path, data: &[u8], mode: u32) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Custom("目标文件缺少父目录。".to_string()))?;
    fs::create_dir_all(parent)?;

    let temp_path = temporary_path(path)?;
    let result = (|| -> AppResult<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(mode);
        }

        let mut file = options.open(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);

        #[cfg(unix)]
        fs::rename(&temp_path, path)?;

        #[cfg(windows)]
        replace_file_windows(&temp_path, path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        }

        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }

        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

#[cfg(windows)]
fn replace_file_windows(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn atomic_write_sets_private_permissions_and_replaces_content() {
        let path = std::env::temp_dir().join(format!(
            "cert-studio-atomic-write-{:016x}",
            rand::thread_rng().next_u64()
        ));

        atomic_write(&path, b"first", PRIVATE_FILE_MODE).unwrap();
        atomic_write(&path, b"second", PRIVATE_FILE_MODE).unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_file(path).unwrap();
    }
}
