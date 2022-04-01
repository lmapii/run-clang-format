use globmatch;

use serde::Deserialize;
use std::path;

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

use crate::cli::{self, utils};

// TODO: UTF-8 restriction?
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JsonModel {
    pub paths: Vec<String>,
    pub blacklist: Option<Vec<String>>,
    pub style: Option<path::PathBuf>,
}

fn extract_err<T>(candidates: Vec<Result<T, String>>) -> eyre::Result<Vec<T>> {
    let failures: Vec<_> = candidates
        .iter()
        .filter_map(|f| match f {
            Ok(_) => None,
            Err(e) => Some(e),
        })
        .collect();

    if failures.len() > 0 {
        eyre::bail!(
            "Failed to compile patterns: \n{}",
            failures
                .iter()
                .map(|err| format!("{}", err))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    Ok(candidates.into_iter().flatten().collect())
}

fn build_matchers<P>(
    globs: &Vec<String>,
    root: P,
    case_sensitive: bool,
) -> eyre::Result<Vec<globmatch::Matcher<path::PathBuf>>>
where
    P: AsRef<path::Path>,
{
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(&pattern)
                .case_sensitive(case_sensitive)
                .build(root.as_ref())
        })
        .collect();

    let candidates = extract_err(candidates)?;
    Ok(candidates)
}

fn build_glob_sets(
    globs: &Vec<String>,
    case_sensitive: bool,
) -> eyre::Result<Vec<globmatch::GlobSet>> {
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(&pattern)
                .case_sensitive(case_sensitive)
                .build_glob_set()
        })
        .collect();

    let candidates = extract_err(candidates)?;
    Ok(candidates)
}

fn resolve_style(data: &cli::Data) -> eyre::Result<eyre::Result<path::PathBuf>> {
    let style_json = match &data.json.style_file {
        None => None,
        Some(path) => {
            let mut full_path = path::PathBuf::from(data.json.root.as_path());
            full_path.push(path);
            Some(
                utils::file_with_name(full_path, ".clang-format")
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
                "Style file must either be specified as parameter or within the configuration file"
            )),
            // style defined as CLI parameter but not in the .json configuration file
            Some(s_cli) => Ok(path::PathBuf::from(s_cli.as_path())),
        },
        Some(s_cfg) => match &data.style {
            // style defined in the .json configuration file but not as CLI parameter
            None => Ok(path::PathBuf::from(s_cfg.as_path())),
            // style defined in both, the .json configuration file and as CLI parameter
            Some(s_cli) => {
                log::info!("Override detected:\nStyle file '{}' specified in '{}'\nis overridden by the command line parameter: '{}'",
                s_cfg.to_string_lossy(), data.json.name, s_cli.as_path().to_string_lossy());
                Ok(path::PathBuf::from(s_cli.as_path()))
            }
        },
    };

    Ok(style)
}

// TODO: corner case: when applying clang-format we should search through ROOT and deterime
// a common root folder where to place the .clang-format
// but we can also search recursively if this would be overridden by any other .clang-format file
// within this tree -> we're ignoring that
// can also define a root in the format.json that defines where to place the clang-format file

pub fn run(data: cli::Data) -> eyre::Result<()> {
    let match_case = if cfg!(windows) { false } else { true };
    let style_file = resolve_style(&data)?;

    let style_root = match data.json.style_root {
        None => None,
        Some(path) => {
            let mut full_path = path::PathBuf::from(data.json.root.as_path());
            full_path.push(path);
            Some(
                utils::dir_or_err(full_path.as_path())
                    .wrap_err("Invalid configuration for 'styleRoot'")
                    .suggestion("Please make sure that 'styleRoot' is a valid directory and check the access permissions")?,
            )
        }
    };

    let copy_style;
    if style_root.is_none() && style_file.is_err() {
        // scenario: no root folder and no style file specified, simply run clang-format
        // and assume that there is a .clang-format file in the root folder of all files
        copy_style = false;
    } else if style_root.is_some() && style_file.is_ok() {
        // scenario: root folder and style file have been specified. it is necessary to copy
        // the style file to the root folder before executing clang-format
        copy_style = true;
    } else if style_root.is_some() && style_file.is_err() {
        // unsupported scenario: root specified but missing style file
        return Err(style_file.unwrap_err().wrap_err(
            "A valid style file must be specified for configurations with the field 'styleRoot'",
        )).suggestion("Specify the style file using the command line parameter or the field 'styleRoot' within the configuration file.");
    } else {
        // unsupported scenario: style file specified but missing root folder
        return Err(eyre::eyre!("Missing root folder configuration",)
            .wrap_err(format!(
                "Found style file '{}' but could not find root folder configuration",
                style_file.unwrap().to_string_lossy()
            ))
            .suggestion("Please add the field 'styleRoot' to your configuration file."));
    }

    if let Ok(style_file) = style_file {
        log::info!(
            "Using parameters from style file {}",
            style_file.to_string_lossy()
        );
    }
    println!("Style file will be copied: {}", copy_style);

    let candidates = build_matchers(&data.json.paths, &data.json.root, match_case)
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
                build_glob_sets(&blacklist_entries, match_case)
                    .wrap_err("Failed to compile patterns for 'paths'")
                    .suggestion(format!(
                        "Check the format of the field 'paths' in {}.",
                        data.json.name
                    ))?,
            )
        }
    };

    let mut filtered = vec![];

    let paths: Vec<_> = candidates
        .into_iter()
        .map(|m| {
            m.into_iter()
                .filter_entry(|p| !globmatch::is_hidden_entry(p))
                .flatten()
                .collect::<Vec<_>>()
        })
        .flatten()
        .filter(|path| path.as_path().is_file()) // accept only files
        .filter(|path| match &blacklist {
            None => true,
            Some(patterns) => {
                let do_filter = !patterns
                    .iter()
                    .try_for_each(|glob| match glob.is_match(path) {
                        true => None,      // path is a match, abort on first match in blacklist
                        false => Some(()), // path is not a match, continue with 'ok'
                    })
                    .is_some(); // the value remains "Some" if no match was encountered
                if do_filter {
                    filtered.push(path::PathBuf::from(path));
                }
                !do_filter
            }
        })
        .collect();

    // TODO: canonicalize() is inefficient for a pretty print since it does access the fs
    let mut paths: Vec<_> = paths.into_iter().collect();
    paths.sort_unstable();
    paths.dedup();

    log::info!(
        "paths \n{}",
        paths
            .iter()
            .map(|p| format!("{}", p.canonicalize().unwrap().to_string_lossy()))
            // .map(|p| format!("{}", p.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    filtered.sort_unstable();
    filtered.dedup();

    if filtered.len() > 0 {
        log::warn!(
            "filtered \n{}",
            filtered
                .iter()
                .map(|p| format!("{}", p.canonicalize().unwrap().to_string_lossy()))
                // .map(|p| format!("{}", p.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    // TODO: invoke command
    // https://stackoverflow.com/questions/21011330/how-do-i-invoke-a-system-command-and-capture-its-output
    // https://stackoverflow.com/questions/49218599/write-to-child-process-stdin-in-rust/49597789#49597789

    log::info!("success :)");
    Ok(())
}
