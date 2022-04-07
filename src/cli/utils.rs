use std::{fs, path};

use color_eyre::{eyre::eyre, eyre::WrapErr};

pub fn path_or_err<P>(path: P) -> eyre::Result<path::PathBuf>
where
    P: AsRef<path::Path>,
{
    let path_as_buf = path::PathBuf::from(path.as_ref());

    if !path_as_buf.exists() {
        return Err(eyre!("Path not found or permission denied"))
            .wrap_err(format!("'{}' is not a path", path_as_buf.to_string_lossy()));
    }

    Ok(path_as_buf)
}

pub fn file_or_err<P>(path: P) -> eyre::Result<path::PathBuf>
where
    P: AsRef<path::Path>,
{
    let path_as_buf = path::PathBuf::from(path.as_ref());

    if !path_as_buf.is_file() {
        return Err(eyre!("File not found or permission denied"))
            .wrap_err(format!("'{}' is not a file", path_as_buf.to_string_lossy()));
    }

    Ok(path_as_buf)
}

pub fn dir_or_err<P>(path: P) -> eyre::Result<path::PathBuf>
where
    P: AsRef<path::Path>,
{
    let path_as_buf = path::PathBuf::from(path.as_ref());
    let meta = fs::metadata(path.as_ref()).wrap_err(format!(
        "'{}' is not a directory",
        path_as_buf.to_string_lossy()
    ))?;

    if !meta.is_dir() {
        return Err(eyre!("Directory not found")).wrap_err(format!(
            "'{}' is not a directory",
            path_as_buf.to_string_lossy()
        ));
    }

    Ok(path_as_buf)
}

pub fn file_with_name<P>(path: P, name: &str) -> eyre::Result<path::PathBuf>
where
    P: AsRef<path::Path>,
{
    let buf = file_or_err(path.as_ref())?;
    let name_str = buf.to_string_lossy();

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

pub fn file_with_ext<P>(path: P, ext: &str, strict: bool) -> eyre::Result<path::PathBuf>
where
    P: AsRef<path::Path>,
{
    let buf = file_or_err(path.as_ref())?;
    let name = buf.to_string_lossy();

    let file_ext = path
        .as_ref()
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or(eyre!(format!(
            "Expected file with extension '{}', got file '{}'",
            ext, name
        )))?;

    let ext_minus = match ext.chars().next() {
        Some(c) if c == '.' && !strict => &ext[1..],
        _ => ext,
    };

    // if ext.starts_with(".") {
    //     &ext[1..]
    // }

    if ext_minus.to_lowercase() != file_ext.to_lowercase() {
        return Err(eyre!(format!(
            "Expected file extension '{}', got '{}'",
            ext_minus, file_ext
        )));
    }
    Ok(buf)
}

pub fn file_with_name_or_ext<P>(path: P, name_or_ext: &str) -> eyre::Result<path::PathBuf>
where
    P: AsRef<path::Path>,
{
    let buf = file_or_err(path.as_ref())?;

    let f_for_name = file_with_name(path.as_ref(), name_or_ext);
    let f_for_ext = file_with_ext(path.as_ref(), name_or_ext, false);

    match f_for_name {
        Ok(path) => Ok(path),
        Err(_) => match f_for_ext {
            Ok(path) => Ok(path),
            Err(_) => Err(eyre!(format!(
                "Expected file with name or extension '{}', got '{}'",
                name_or_ext,
                buf.to_string_lossy()
            ))),
        },
    }
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
