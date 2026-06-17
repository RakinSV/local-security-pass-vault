//! Безопасные файловые операции. См. `.claude/rules/security.md` («Защита файлов»).

use crate::error::{Result, VaultError};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

/// Атомарная запись: tmp → fsync → rename. Гарантирует, что целевой файл
/// либо старый целиком, либо новый целиком — никогда «полузаписанный».
pub fn atomic_write(target: &Path, data: &[u8]) -> Result<()> {
    let tmp = target.with_extension("tmp");

    {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        // fsync — данные физически на диске до rename.
        file.sync_all()?;
    } // файл закрыт

    // rename атомарен: POSIX rename / Windows MoveFileExW(REPLACE_EXISTING).
    std::fs::rename(&tmp, target)?;
    Ok(())
}

/// Чтение файла с отказом при symlink. lstat (`symlink_metadata`) не следует по
/// ссылке, поэтому подмену через symlink ловим до открытия содержимого.
pub fn read_no_symlink(path: &Path) -> Result<Vec<u8>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(VaultError::NotFound),
        Err(e) => return Err(VaultError::from(e)),
    };
    if meta.file_type().is_symlink() {
        return Err(VaultError::SymlinkDetected(path.to_owned()));
    }

    // На Unix дополнительно открываем с O_NOFOLLOW (защита от TOCTOU между lstat
    // и open). На Windows полагаемся на проверку выше + отсутствие reparse-точки.
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .custom_flags(libc_o_nofollow())
            .open(path)?
    };
    #[cfg(not(unix))]
    let mut file = OpenOptions::new().read(true).open(path)?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

#[cfg(unix)]
fn libc_o_nofollow() -> i32 {
    // O_NOFOLLOW без зависимости от crate libc: значение стабильно на Linux/macOS.
    // Linux: 0o400000, macOS: 0x0100. Берём через std, если доступно, иначе константа.
    #[cfg(target_os = "linux")]
    {
        0o400000
    }
    #[cfg(target_os = "macos")]
    {
        0x0100
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        0
    }
}

/// Ограничивает права доступа к файлу до владельца (0600 на Unix; на Windows —
/// NTFS наследует ACL пользовательского профиля, доп. шаги — на уровне desktop).
pub fn restrict_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path; // На Windows ACL-ограничения выполняет desktop-слой.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_then_read() {
        let dir = std::env::temp_dir().join(format!("vp_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("vault.db");

        atomic_write(&target, b"hello").unwrap();
        assert_eq!(read_no_symlink(&target).unwrap(), b"hello");

        // перезапись — атомарна, читается новое
        atomic_write(&target, b"world!!").unwrap();
        assert_eq!(read_no_symlink(&target).unwrap(), b"world!!");
        // tmp удалён после rename
        assert!(!target.with_extension("tmp").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_missing_is_not_found() {
        let p = std::env::temp_dir().join("vp_definitely_missing_xyz.db");
        assert!(matches!(read_no_symlink(&p), Err(VaultError::NotFound)));
    }
}
