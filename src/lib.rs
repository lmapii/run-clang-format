use globmatch;

use serde::Deserialize;
use std::path;

use color_eyre::{eyre::eyre, eyre::Report, eyre::WrapErr, Help};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppCfg {
    pub paths: Vec<String>,
    pub blacklist: Option<Vec<String>>,
}

fn check_patterns<T>(candidates: &Vec<Result<T, String>>) -> Result<(), Report> {
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
    Ok(())
}

fn get_matchers<P>(
    globs: &Vec<String>,
    root: P,
    case_sensitive: bool,
) -> Result<Vec<globmatch::Matcher<path::PathBuf>>, Report>
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

    check_patterns(&candidates)?;
    let candidates = candidates.into_iter().flatten().collect();
    Ok(candidates)
}

fn get_globs(globs: &Vec<String>, case_sensitive: bool) -> Result<Vec<globmatch::GlobSet>, Report> {
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(&pattern)
                .case_sensitive(case_sensitive)
                .build_glob_set()
        })
        .collect();

    check_patterns(&candidates)?;
    let candidates = candidates.into_iter().flatten().collect();
    Ok(candidates)
}

pub fn run(matches: clap::ArgMatches) -> eyre::Result<(), Report> {
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

    let case_sensitive = if cfg!(windows) { false } else { true };

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

    let candidates = get_matchers(&json.paths, &json_root, case_sensitive)
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
                get_globs(&blacklist_entries, case_sensitive)
                    .wrap_err("Failed to compile patterns for 'paths'")
                    .suggestion(format!(
                        "Check the format of the entry 'paths' in {}.",
                        json_file
                    ))?,
            )
        }
    };

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
                let matches = patterns
                    .iter()
                    .try_for_each(|glob| match glob.is_match(path) {
                        true => None,      // path is a match, abort on first match in blacklist
                        false => Some(()), // path is not a match, continue with 'ok'
                    })
                    .is_some();
                if !matches {
                    log::warn!("filtered: {}", path.to_string_lossy());
                }
                matches
            }
        })
        .collect();

    let paths: Vec<_> = paths.into_iter().collect();
    log::info!(
        "paths \n{}",
        paths
            .iter()
            .map(|p| format!("{}", p.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    log::info!("success :)");
    Ok(())
}
