use std::{fs, path};

#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;

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

struct LogStep {
    step: u8,
}

impl LogStep {
    fn new() -> LogStep {
        LogStep { step: 1 }
    }

    fn next(&mut self) -> String {
        // TODO: the actual number of steps could be determined by a macro?
        let str = format!(
            "{}",
            console::style(format!("[ {:1}/5 ]", self.step))
                .bold()
                .dim()
        );
        self.step += 1;
        if log_pretty() {
            str
        } else {
            "".to_string()
        }
    }
}

fn get_command(data: &cli::Data) -> eyre::Result<cmd::Runner> {
    let cmd_path = resolve::command(data);
    let mut cmd = cmd::Runner::new(&cmd_path);

    cmd.check()
        .wrap_err(format!(
            "Failed to execute '{}'",
            cmd_path.to_string_lossy()
        ))
        .suggestion(format!(
            "Please make sure that the command '{}' exists or is in your search path",
            cmd_path.to_string_lossy()
        ))?;

    Ok(cmd)
}

fn place_style_file(
    file_and_root: Option<(path::PathBuf, path::PathBuf)>,
    step: &mut LogStep,
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
            log::info!(
                "{} Existing style file matches {}, skipping placement",
                step.next(),
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
        "{} Copying style file to {}",
        step.next(),
        console::style(dst_file.to_string_lossy()).bold(),
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
    let start = std::time::Instant::now();

    log::info!("");
    let mut step = LogStep::new();

    let style_and_root = resolve::style_and_root(&data)?;
    if let Some((style_file, _)) = &style_and_root {
        log::info!(
            "{} Found style file {}",
            step.next(),
            console::style(style_file.to_string_lossy()).bold(),
        );
    } else {
        log::info!(
            "{} No style file specified, assuming .clang-format exists in the project tree",
            step.next()
        );
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

    let (paths, filtered) = globs::match_paths(candidates, blacklist);
    let paths = paths.into_iter().map(|p| p.canonicalize().unwrap());

    let filtered = if filtered.is_empty() {
        "".to_string()
    } else {
        format!(" (filtered {} paths)", filtered.len())
    };

    log::info!(
        "{} Found {} files for the provided path patterns{}",
        step.next(),
        console::style(paths.len()).bold(),
        filtered
    );

    let cmd = get_command(&data)?;
    log::info!(
        "{} Found clang-format version {} using command {}",
        step.next(),
        console::style(cmd.get_version().unwrap()).bold(),
        console::style(cmd.get_path().to_string_lossy()).bold(),
    );

    let strip_root = if let Some((_, style_root)) = &style_and_root {
        Some(path::PathBuf::from(style_root.as_path()))
    } else {
        None
    };

    let style = place_style_file(style_and_root, &mut step)?;

    let _style = scopeguard::guard(style, |path| {
        // ensure we delete the temporary style file at return or panic
        if let Some(path) = path {
            let str = format!("Cleaning up temporary file {}\n", path.to_string_lossy());
            let str = console::style(str).dim().italic();

            log::info!("{}", str);
            let _ = fs::remove_file(path);
        }
    });

    // configure rayon to use the specified number of threads (globally)
    if let Some(jobs) = data.jobs {
        let jobs = if jobs == 0 { 1u8 } else { jobs };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs.into())
            .build_global();

        if let Err(err) = pool {
            return Err(err)
                .wrap_err(format!("Failed to create thread pool of size {}", jobs))
                .suggestion("Please try to decrease the number of jobs");
        }
    }

    log::info!("{} Executing clang-format ...\n", step.next(),);

    let pb = indicatif::ProgressBar::new(paths.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(if console::Term::stdout().size().1 > 80 {
                "{prefix:>12.cyan.bold} [{bar:26}] {pos}/{len} {wide_msg}"
            } else {
                "{prefix:>12.cyan.bold} [{bar:26}] {pos}/{len}"
            })
            .progress_chars("=> "),
    );

    // pb.set_style(
    //     indicatif::ProgressStyle::with_template(if console::Term::stdout().size().1 > 80 {
    //         "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len} {wide_msg}"
    //     } else {
    //         "{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len}"
    //     })
    //     .unwrap()
    //     .progress_chars("=> "),
    // );

    if log_pretty() {
        pb.set_prefix("Running");
    }

    let green_bold = console::Style::new().green().bold();

    let paths: Vec<_> = paths.collect();
    let _: eyre::Result<()> = paths.into_par_iter().try_for_each(|path| {
        let print_path = if let Some(strip_path) = &strip_root {
            path.strip_prefix(strip_path).unwrap()
        } else {
            &path
        };

        if !log_pretty() {
            log::info!("  + {}", path.to_string_lossy());
        } else {
            pb.println(format!(
                "{:>12} {}",
                green_bold.apply_to("Formatting"),
                print_path.to_string_lossy(),
            ));
            pb.inc(1);
        }
        let _ = cmd.format(&path)
            .wrap_err(format!("Failed to format {}", path.to_string_lossy()))
            .suggestion("Please make sure that your style file matches the version of clang-format and that you have the necessary permissions to modify all files")?;
        Ok(())
    });

    let duration = start.elapsed();
    if log_pretty() {
        pb.finish();

        println!(
            "{:>12} in {}",
            green_bold.apply_to("Finished"),
            indicatif::HumanDuration(duration)
        );
    } else {
        log::info!("{} Finished in {:#?}", step.next(), duration);
    }

    log::info!(""); // just an empty newline
    Ok(())
}

fn log_pretty() -> bool {
    !log::log_enabled!(log::Level::Debug) && log::log_enabled!(log::Level::Info)
}
