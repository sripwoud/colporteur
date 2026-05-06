use std::path::Path;

use eyre::Context;

pub fn write_atomic(path: &Path, contents: &[u8]) -> eyre::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create directories for {}", path.display()))?;
    }

    let tmp_path = path.with_extension("tmp");

    std::fs::write(&tmp_path, contents)
        .wrap_err_with(|| format!("failed to write tmp file: {}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, path).wrap_err_with(|| {
        format!(
            "failed to rename {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "colporteur_test_fs_atomic_{name}_{}",
            std::process::id()
        ))
    }

    #[test]
    fn writes_content_and_creates_parent_dirs() {
        let dir = unique_dir("create_parents");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("out.bin");

        write_atomic(&path, b"hello").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert!(!path.with_extension("tmp").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn overwrites_existing_file() {
        let dir = unique_dir("overwrite");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.bin");
        std::fs::write(&path, b"old").unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(!path.with_extension("tmp").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_tmp_file_lingers_on_success() {
        let dir = unique_dir("no_lingering_tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.bin");

        write_atomic(&path, b"x").unwrap();

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "out.bin");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
