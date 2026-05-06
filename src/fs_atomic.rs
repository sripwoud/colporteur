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
