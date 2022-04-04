use serde::Deserialize;
use std::{fs, path};

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

use crate::cli;
use crate::cmd;

mod globs;
mod resolve;

// TODO: UTF-8 restriction?
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JsonModel {
    pub paths: Vec<String>,
    pub blacklist: Option<Vec<String>>,
    pub style: Option<path::PathBuf>,
}

fn get_command(data: &cli::Data) -> eyre::Result<cmd::Runner> {
    let cmd_path = resolve::command(data);
    let cmd = cmd::Runner::new(&cmd_path);
    let version = cmd
        .get_version()
        .wrap_err(format!(
            "Failed to execute '{}'",
            cmd_path.to_string_lossy()
        ))
        .suggestion(format!(
            "Please make sure that the command '{}' exists or is in your search path",
            cmd_path.to_string_lossy()
        ))?;

    log::info!(
        "Using '{}', version {}",
        cmd_path.to_string_lossy(),
        version
    );

    Ok(cmd)
}

fn place_style_file(
    file_and_root: Option<(path::PathBuf, path::PathBuf)>,
) -> eyre::Result<Option<path::PathBuf>> {
    if file_and_root.is_none() {
        // in case no style file has been specified there's nothing to do
        return Ok(None);
    }

    // the style file `src` should be copied to the destination directory `dst`
    let (src_file, dst_root) = file_and_root.unwrap();
    let mut dst_file = path::PathBuf::from(dst_root.as_path());
    // by adding the filename of the style file we get the final name of the destination file
    dst_file.push(".clang-format");

    // it may happen that there is already a .clang-format file at the destination folder, e.g.,
    // because the user placed it there while working with an editor supporting `clang-format`.
    // in such a case we provide feedback by comparing the file contents and abort with an error
    // if they do not match.
    if dst_file.exists() {
        log::warn!(
            "Encountered existing style file {}",
            dst_file.to_string_lossy()
        );

        let content_src = fs::read_to_string(&src_file)
            .wrap_err(format!("Failed to read '{}'", dst_file.to_string_lossy()))?;
        let content_dst = fs::read_to_string(&dst_file.as_path())
            .wrap_err(format!("Failed to read '{}'", dst_file.to_string_lossy()))
            .wrap_err("Error while trying to compare existing style file")
            .suggestion(format!(
                "Please delete or fix the existing style file {}",
                dst_file.to_string_lossy()
            ))?;

        if content_src == content_dst {
            log::warn!(
                "Existing style file matches {}, skipping placement",
                src_file.to_string_lossy()
            );
            return Ok(None);
        }

        return Err(eyre::eyre!(
            "Existing style file {} does not match provided style file {}",
            dst_file.to_string_lossy(),
            src_file.to_string_lossy()
        )
        .suggestion(format!(
            "Please either delete the file {} or align the contents with {}",
            dst_file.to_string_lossy(),
            src_file.to_string_lossy()
        )));
    }

    log::info!(
        "Copying '{}' to '{}'",
        src_file.to_string_lossy(),
        dst_root.to_string_lossy()
    );
    // no file found at destination, copy the provided style file
    let _ = fs::copy(&src_file, &dst_file)
        .wrap_err(format!(
            "Failed to copy style file to {}",
            dst_root.to_string_lossy(),
        ))
        .suggestion(format!(
            "Please check the permissions for the folder {}",
            dst_root.to_string_lossy()
        ))?;

    Ok(Some(dst_file))
}

pub fn run(data: cli::Data) -> eyre::Result<()> {
    let style_and_root = resolve::style_and_root(&data)?;
    if let Some((style_file, style_root)) = &style_and_root {
        log::info!(
            "Using parameters from style file {}",
            style_file.to_string_lossy(),
        );
        log::info!("Placing to format root {}", style_root.to_string_lossy());
    }

    let match_case = !cfg!(windows);
    let candidates = globs::build_matchers(&data.json.paths, &data.json.root, match_case)
        .wrap_err("Error while parsing 'paths'")
        .suggestion(format!(
            "Check the format of the field 'paths' in the provided file '{}'.",
            data.json.name
        ))?;

    let blacklist_entries; // create binding that lives long enough
    let blacklist = match &data.json.blacklist {
        None => None,
        Some(paths) => {
            blacklist_entries = paths;
            Some(
                globs::build_glob_sets(blacklist_entries, match_case)
                    .wrap_err("Failed to compile patterns for 'paths'")
                    .suggestion(format!(
                        "Check the format of the field 'paths' in {}.",
                        data.json.name
                    ))?,
            )
        }
    };

    let paths = globs::match_paths(candidates, blacklist)
        .into_iter()
        .map(|p| p.canonicalize().unwrap());

    let cmd = get_command(&data)?;
    let style = place_style_file(style_and_root)?;

    let _style = scopeguard::guard(style, |path| {
        // ensure we delete the temporary style file at return or panic
        if let Some(path) = path {
            log::debug!("Deleting temporary file {}", path.to_string_lossy());
            let _ = fs::remove_file(path);
        }
    });

    // TODO: execute concurrently using multiple threads?
    log::info!("Formatting files...");
    for path in paths {
        log::info!("  + {}", path.to_string_lossy());
        let _ = cmd.format(path.as_path())
            .wrap_err(format!("Failed to format {}", path.to_string_lossy()))
            .suggestion("Please make sure that your style file matches the version of clang-format and that you have the necessary permissions to modify all files")?;
    }

    log::info!("success :)");
    Ok(())
}
