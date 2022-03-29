use globmatch;

use serde::Deserialize;
use std::path;

use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppCfg {
    pub paths: Vec<String>,
    pub blacklist: Option<Vec<String>>,
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

pub fn run(matches: clap::ArgMatches) -> eyre::Result<(), eyre::Report> {
    // use ensure! for parameter checks

    let json_path = matches
        .value_of_os("JSON")
        .map(std::path::PathBuf::from)
        .ok_or(eyre!("Could not convert parameter <JSON> to path"))?;

    let style_path = match matches.is_present("style") {
        false => None,
        true => Some(
            matches
                .value_of_os("style")
                .map(std::path::PathBuf::from)
                .ok_or(eyre!("Could not convert parameter --style to path"))?,
        ),
    };

    let match_case = if cfg!(windows) { false } else { true };

    let json_file = json_path.to_string_lossy();
    let f = std::fs::File::open(&json_path)
        .wrap_err(format!("Failed to open configuration file '{}'", json_file))?;

    let json: AppCfg = serde_json::from_reader(std::io::BufReader::new(f))
        .wrap_err(format!("Validation failed for '{}'", json_file))
        .suggestion(format!(
            "Ensure that '{}' is a valid .json file and contains all required fields.",
            json_file
        ))?;

    // TODO: print json schema in case of errors

    let json_root = path::PathBuf::from(json_path.canonicalize().unwrap().parent().unwrap());
    log::info!("parent folder of json = {}", json_root.to_string_lossy());

    // let paths: Vec<_> = json.paths.iter().map(|p| json_root.join(p)).collect();
    // log::info!("joined paths {:?}", paths);

    let candidates = build_matchers(&json.paths, &json_root, match_case)
        .wrap_err("Failed to compile patterns for 'paths'")
        .suggestion(format!(
            "Check the format of the entry 'paths' in {}.",
            json_file
        ))?;

    let blacklist_entries; // create binding that lives long enough
    let blacklist = match json.blacklist {
        None => None,
        Some(paths) => {
            blacklist_entries = paths;
            Some(
                build_glob_sets(&blacklist_entries, match_case)
                    .wrap_err("Failed to compile patterns for 'paths'")
                    .suggestion(format!(
                        "Check the format of the entry 'paths' in {}.",
                        json_file
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
                    .is_some(); // the value is "Some" if no match was encountered

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

    log::info!("success :)");
    Ok(())
}
