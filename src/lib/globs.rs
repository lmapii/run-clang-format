use std::path;

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

fn extract_patterns<T>(candidates: Vec<Result<T, String>>) -> eyre::Result<Vec<T>> {
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

fn wrap_result<T>(result: eyre::Result<T>, field: &str, file: &str) -> eyre::Result<T> {
    result
        .wrap_err(format!("Error while parsing '{}'", field))
        .suggestion(format!(
            "Check the format of the field '{}' in the provided file '{}'.",
            field, file
        ))
}

pub fn build_matchers_from<'a, P>(
    globs: &'a [String],
    root: P,
    field: &str,
    file: &str,
) -> eyre::Result<Vec<globmatch::Matcher<'a, path::PathBuf>>>
where
    P: AsRef<path::Path>,
{
    wrap_result(build_matchers(globs, root), field, file)
}

fn build_matchers<P>(
    globs: &[String],
    root: P,
) -> eyre::Result<Vec<globmatch::Matcher<path::PathBuf>>>
where
    P: AsRef<path::Path>,
{
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(pattern)
                .case_sensitive(!cfg!(windows))
                .build(root.as_ref())
        })
        .collect();

    let candidates = extract_patterns(candidates)?;
    Ok(candidates)
}

pub fn build_glob_set_from<'a>(
    filter: &'a Option<Vec<String>>,
    field: &str,
    file: &str,
) -> eyre::Result<Option<Vec<globmatch::GlobSet<'a>>>> {
    wrap_result(build_glob_set(filter), field, file)
}

fn build_glob_set(filter: &Option<Vec<String>>) -> eyre::Result<Option<Vec<globmatch::GlobSet>>> {
    let filter = match filter {
        None => None,
        Some(paths) => Some(glob_sets(paths, !cfg!(windows))?),
    };
    Ok(filter)
}

fn glob_sets(globs: &[String], case_sensitive: bool) -> eyre::Result<Vec<globmatch::GlobSet>> {
    let candidates: Vec<Result<_, String>> = globs
        .iter()
        .map(|pattern| {
            globmatch::Builder::new(pattern)
                .case_sensitive(case_sensitive)
                .build_glob_set()
        })
        .collect();

    let candidates = extract_patterns(candidates)?;
    Ok(candidates)
}

pub fn match_paths<P>(
    candidates: Vec<globmatch::Matcher<P>>,
    filter: Option<Vec<globmatch::GlobSet>>,
    filter_post: Option<Vec<globmatch::GlobSet>>,
) -> (Vec<path::PathBuf>, Vec<path::PathBuf>)
where
    P: AsRef<path::Path>,
{
    let mut filtered = vec![];

    let paths = candidates
        .into_iter()
        .flat_map(|m| {
            m.into_iter()
                .filter_entry(|path| {
                    match &filter {
                        // yield all entries if no pattern have been provided
                        // but try_for_each yields all elements for an empty vector (see test)
                        // Some(patterns) if patterns.is_empty() => true,
                        // Some(patterns) if !patterns.is_empty() => {
                        Some(patterns) => {
                            let do_filter = patterns
                                .iter()
                                .try_for_each(|glob| match glob.is_match(path) {
                                    true => None,      // path is a match, abort on first match
                                    false => Some(()), // path is not a match, continue with 'ok'
                                })
                                .is_none(); // the value remains "Some" if no match was encountered
                            !do_filter
                        }
                        _ => !globmatch::is_hidden_entry(path), // yield entries that are not hidden
                    }
                })
                .flatten()
                .collect::<Vec<_>>()
        })
        .filter(|path| path.as_path().is_file()) // accept only files
        .filter(|path| match &filter_post {
            None => true,
            Some(patterns) => {
                let do_filter = patterns
                    .iter()
                    .try_for_each(|glob| match glob.is_match(path) {
                        true => None,      // path is a match, abort on first match in filter_post
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

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_foreach() {
        let items = vec![0u8, 1u8, 2u8];
        let filter: Vec<u8> = vec![];

        // show that an empty filter list yields all elements
        let filter_zero: Vec<_> = items
            .iter()
            .filter(|item| {
                let do_filter = filter
                    .iter()
                    .try_for_each(|filter_item| {
                        if *filter_item == **item {
                            None // abort on first match
                        } else {
                            Some(()) // no match, continue
                        }
                    })
                    .is_none(); // the value remains "Some" if no match was encountered
                !do_filter
            })
            .cloned()
            .collect();

        assert_eq!(filter_zero, items);
    }
}
