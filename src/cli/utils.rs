use std::path;

use color_eyre::{eyre::eyre, eyre::WrapErr};

pub fn path_or_err(path: path::PathBuf) -> eyre::Result<path::PathBuf, eyre::Report> {
    if !path.exists() {
        return Err(eyre!("No such file or directory"))
            .wrap_err(format!("Failed to open file '{}'", path.to_string_lossy()));
    }
    Ok(path)
}

pub fn file_with_name<P>(path: P, name: &str) -> eyre::Result<path::PathBuf, eyre::Report>
where
    P: AsRef<path::Path>,
{
    let buf = path::PathBuf::from(path.as_ref());
    let name_str = buf.to_string_lossy();

    if !path.as_ref().is_file() {
        return Err(eyre!(format!("'{}' is not a file", name_str)));
    }

    let file_name = path
        .as_ref()
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or(eyre!(format!(
            "Expected file with name '{}', got '{}'",
            name, name_str
        )))?;

    if file_name.to_lowercase() != name.to_lowercase() {
        return Err(eyre!(format!(
            "Expected file with name '{}', got '{}'",
            name, name_str
        )));
    }
    Ok(buf)
}

pub fn file_with_ext<P>(path: P, ext: &str) -> eyre::Result<path::PathBuf, eyre::Report>
where
    P: AsRef<path::Path>,
{
    let buf = path::PathBuf::from(path.as_ref());
    let name = buf.to_string_lossy();

    if !path.as_ref().is_file() {
        return Err(eyre!(format!("'{}' is not a file", name)));
    }

    let file_ext = path
        .as_ref()
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or(eyre!(format!(
            "Expected file with extension '{}', got file '{}'",
            ext, name
        )))?;

    if ext.to_lowercase() != file_ext.to_lowercase() {
        return Err(eyre!(format!(
            "Expected file extension '{}', got '{}'",
            ext, file_ext
        )));
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path() {
        let path = path::Path::new("some/path/to/.clang-format");
        let file_name = path.file_name().and_then(std::ffi::OsStr::to_str).unwrap();

        assert_eq!(".clang-format", file_name.to_lowercase());
    }
}
