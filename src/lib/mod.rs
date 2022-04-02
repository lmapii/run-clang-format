use serde::Deserialize;
use std::path;

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

// struct App<'a> {
//     candidates: Vec<globmatch::Matcher<'a, path::PathBuf>>,
//     blacklist: Option<Vec<globmatch::GlobSet<'a>>>,

//     style_file: Option<path::PathBuf>,
//     style_root: Option<path::PathBuf>,
// }

pub fn run(data: cli::Data) -> eyre::Result<()> {
    let (style_file, style_root) = resolve::style_and_root(&data)?;
    if let Some(style_file) = style_file {
        log::info!(
            "Using parameters from style file {}",
            style_file.to_string_lossy(),
        );
        log::info!(
            "Placing to format root {}",
            style_root.unwrap().to_string_lossy()
        );
    }

    let cmd_path = resolve::command(&data);
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

    let match_case = if cfg!(windows) { false } else { true };
    let candidates = globs::build_matchers(&data.json.paths, &data.json.root, match_case)
        .wrap_err("Error while parsing 'paths'")
        .suggestion(format!(
            "Check the format of the field 'paths' in the provided file '{}'.",
            data.json.name
        ))?;

    let blacklist_entries; // create binding that lives long enough
    let blacklist = match data.json.blacklist {
        None => None,
        Some(paths) => {
            blacklist_entries = paths;
            Some(
                globs::build_glob_sets(&blacklist_entries, match_case)
                    .wrap_err("Failed to compile patterns for 'paths'")
                    .suggestion(format!(
                        "Check the format of the field 'paths' in {}.",
                        data.json.name
                    ))?,
            )
        }
    };

    let paths = globs::match_paths(candidates, blacklist);

    log::info!("success :)");
    Ok(())
}
