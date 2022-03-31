use globmatch;

use serde::Deserialize;
use std::path;

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

use crate::cli;

// TODO: UTF-8 restriction?
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JsonModel {
    pub paths: Vec<String>,
    pub blacklist: Option<Vec<String>>,
    pub style: Option<path::PathBuf>,
}

fn extract_err<T>(candidates: Vec<Result<T, String>>) -> eyre::Result<Vec<T>, eyre::Report> {
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
) -> eyre::Result<Vec<globmatch::Matcher<path::PathBuf>>, eyre::Report>
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
) -> eyre::Result<Vec<globmatch::GlobSet>, eyre::Report> {
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

pub fn run(data: cli::Data) -> eyre::Result<(), eyre::Report> {
    let match_case = if cfg!(windows) { false } else { true };

    // // TODO: prettify error: prepend "Error while loading json_file"
    // let style_json = match &cfg.style {
    //     None => None,
    //     Some(path) => {
    //         let mut full_path = path::PathBuf::from(json_root.as_path());
    //         full_path.push(path);
    //         Some(
    //             path_exists_or_err(full_path)
    //                 .wrap_err(format!("Invalid configuration for 'style''"))
    //                 .suggestion(format!(
    //                     "Check the format of the field 'style' in {}.",
    //                     json_file
    //                 ))?,
    //         )
    //     }
    // };

    // let paths: Vec<_> = data
    //     .json
    //     .paths
    //     .iter()
    //     .map(|p| data.json.root.join(p))
    //     .collect();

    // log::info!("joined paths {:?}", paths);

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
    let paths: Vec<_> = paths.into_iter().collect();
    log::info!(
        "paths \n{}",
        paths
            .iter()
            .map(|p| format!("{}", p.canonicalize().unwrap().to_string_lossy()))
            // .map(|p| format!("{}", p.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    );

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

    // TODO: remove duplicates

    // TODO: invoke command
    // https://stackoverflow.com/questions/21011330/how-do-i-invoke-a-system-command-and-capture-its-output
    // https://stackoverflow.com/questions/49218599/write-to-child-process-stdin-in-rust/49597789#49597789

    log::info!("success :)");
    Ok(())
}
