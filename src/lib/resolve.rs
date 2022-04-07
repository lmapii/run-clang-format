use std::path;

use crate::cli::{self, utils};

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

fn resolve_style_file(data: &cli::Data) -> eyre::Result<eyre::Result<path::PathBuf>> {
    let style_json = match &data.json.style_file {
        None => None,
        Some(path) => {
            let mut full_path = path::PathBuf::from(data.json.root.as_path());
            full_path.push(path);
            Some(
                utils::file_with_name_or_ext(full_path, ".clang-format")
                    .wrap_err("Invalid configuration for 'styleFile''")
                    .suggestion(format!(
                        "Check the content of the field 'styleFile' in {}.",
                        data.json.name
                    ))?,
            )
        }
    };

    let style = match style_json {
        None => match &data.style {
            None => Err(eyre::eyre!(
                "Style file must either be specified as command-line parameter or within the configuration file"
            )),
            // style defined as CLI parameter but not in the .json configuration file
            Some(s_cli) => Ok(path::PathBuf::from(s_cli.as_path()).canonicalize().unwrap()),
        },
        Some(s_cfg) => match &data.style {
            // style defined in the .json configuration file but not as CLI parameter
            None => Ok(path::PathBuf::from(s_cfg.as_path()).canonicalize().unwrap()),
            // style defined in both, the .json configuration file and as CLI parameter
            Some(s_cli) => {
                log::info!("Override detected:\nStyle file '{}' specified in '{}'\nis overridden by the command line parameter: '{}'",
                s_cfg.to_string_lossy(), data.json.name, s_cli.as_path().to_string_lossy());
                Ok(path::PathBuf::from(s_cli.as_path()).canonicalize().unwrap())
            }
        },
    };

    Ok(style)
}

pub fn style_and_root(data: &cli::Data) -> eyre::Result<Option<(path::PathBuf, path::PathBuf)>> {
    let style_file = resolve_style_file(data)?;
    let style_root = match &data.json.style_root {
        None => None,
        Some(path) => {
            let path = if path.is_absolute() {
                path::PathBuf::from(path.as_path())
            } else {
                let mut full_path = path::PathBuf::from(data.json.root.as_path());
                full_path.push(path);
                full_path
            };
            Some(
                utils::dir_or_err(path.as_path())
                    .wrap_err("Invalid configuration for 'styleRoot'")
                    .suggestion("Please make sure that 'styleRoot' is a valid directory and check the access permissions")?
                    .canonicalize()
                    .unwrap(),
            )
        }
    };

    if let Err(style_err) = style_file {
        match style_root {
            // scenario: no root folder and no style file specified, simply run clang-format
            // and assume that there is a .clang-format file in the root folder of all files
            None => Ok(None),
            // unsupported scenario: root specified but missing style file
            Some(_) =>
                Err(style_err.wrap_err(
                        "A valid style file must be specified for configurations with the field 'styleRoot'",
                    )).suggestion("Specify the style file using the command line parameter or the field 'styleRoot' within the configuration file.")
        }
    } else {
        match style_root {
            // scenario: root folder and style file have been specified. it is necessary to copy
            // the style file to the root folder before executing clang-format
            Some(style_root) => Ok(Some((style_file.unwrap(), style_root))),
            // unsupported scenario: style file specified but missing root folder
            None => Err(eyre::eyre!("Missing root folder configuration",)
                .wrap_err(format!(
                    "Found style file '{}' but could not find root folder configuration",
                    style_file.unwrap().to_string_lossy()
                ))
                .suggestion("Please add the field 'styleRoot' to your configuration file.")),
        }
    }
}

pub fn command(data: &cli::Data) -> path::PathBuf {
    match &data.json.command {
        None => match &data.command {
            // use default value if not specified in configuration file nor as parameter
            None => path::PathBuf::from("clang-format"),
            // cmd defined as CLI parameter but not in the .json configuration file
            Some(cmd_cli) => path::PathBuf::from(cmd_cli.as_path()),
        },
        Some(cmd_cfg) => match &data.command {
            // cmd defined in the .json configuration file but not as CLI parameter
            None => path::PathBuf::from(cmd_cfg.as_path()),
            // cmd defined in both, the .json configuration file and as CLI parameter
            Some(cmd_cli) => {
                log::info!("Override detected:\nCommand '{}' specified in '{}'\nis overridden by the command line parameter: '{}'",
                cmd_cfg.to_string_lossy(), data.json.name, cmd_cli.as_path().to_string_lossy());
                path::PathBuf::from(cmd_cli.as_path())
            }
        },
    }
}
