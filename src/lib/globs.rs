use std::path;

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

fn extract_err<T>(candidates: Vec<Result<T, String>>) -> eyre::Result<Vec<T>> {
    let failures: Vec<_> = candidates
        .iter()
        .filter_map(|f| match f {
            Ok(_) => None,
            Err(e) => Some(e),
        })
        .collect();

    if !failures.is_empty() {
        eyre::bail!(
            "Failed to compile patterns: \n{}",
            failures
                .iter()
                .map(|err| err.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    Ok(candidates.into_iter().flatten().collect())
}

pub fn build_matchers<P>(
    globs: &[String],
    root: P,
    case_sensitive: bool,
) -> eyre::Result<Vec<globmatch::Matcher<path::PathBuf>>>
where
    P: AsRef<path::Path>,
{
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(pattern)
                .case_sensitive(case_sensitive)
                .build(root.as_ref())
        })
        .collect();

    let candidates = extract_err(candidates)?;
    Ok(candidates)
}

pub fn build_glob_sets(
    globs: &[String],
    case_sensitive: bool,
) -> eyre::Result<Vec<globmatch::GlobSet>> {
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(pattern)
                .case_sensitive(case_sensitive)
                .build_glob_set()
        })
        .collect();

    let candidates = extract_err(candidates)?;
    Ok(candidates)
}

pub fn match_paths<P>(
    candidates: Vec<globmatch::Matcher<P>>,
    blacklist: Option<Vec<globmatch::GlobSet>>,
) -> (Vec<path::PathBuf>, Vec<path::PathBuf>)
where
    P: AsRef<path::Path>,
{
    let mut filtered = vec![];

    let paths = candidates
        .into_iter()
        .flat_map(|m| {
            m.into_iter()
                .filter_entry(|p| !globmatch::is_hidden_entry(p))
                .flatten()
                .collect::<Vec<_>>()
        })
        .filter(|path| path.as_path().is_file()) // accept only files
        .filter(|path| match &blacklist {
            None => true,
            Some(patterns) => {
                let do_filter = patterns
                    .iter()
                    .try_for_each(|glob| match glob.is_match(path) {
                        true => None,      // path is a match, abort on first match in blacklist
                        false => Some(()), // path is not a match, continue with 'ok'
                    })
                    .is_none(); // the value remains "Some" if no match was encountered
                if do_filter {
                    filtered.push(path::PathBuf::from(path));
                }
                !do_filter
            }
        });

    let mut paths: Vec<_> = paths.collect();
    paths.sort_unstable();
    paths.dedup();

    log::debug!(
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

    if !filtered.is_empty() {
        log::debug!(
            "filtered \n{}",
            filtered
                .iter()
                .map(|p| format!("{}", p.canonicalize().unwrap().to_string_lossy()))
                // .map(|p| format!("{}", p.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    (paths, filtered)
}
