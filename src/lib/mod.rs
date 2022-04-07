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
    pub filter_post: Option<Vec<String>>,
    pub style: Option<path::PathBuf>,
}

fn log_pretty() -> bool {
    // fancy logging using indicatif is only done for log level "info". when debugging we
    // do not use a progress bar, if info is not enabled at all ("quiet") then the progress
    // is also not shown
    !log::log_enabled!(log::Level::Debug) && log::log_enabled!(log::Level::Info)
}

struct LogStep(u8);

impl LogStep {
    fn new() -> LogStep {
        LogStep(1)
    }

    fn next(&mut self) -> String {
        // TODO: the actual number of steps could be determined by a macro?
        let str = format!(
            "{}",
            console::style(format!("[ {:1}/5 ]", self.0)).bold().dim()
        );
        self.0 += 1;
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
        let src_name = src_file.to_string_lossy();
        let dst_name = dst_file.to_string_lossy();

        log::warn!("Encountered existing style file {}", dst_name);

        let content_src =
            fs::read_to_string(&src_file).wrap_err(format!("Failed to read '{}'", dst_name))?;
        let content_dst = fs::read_to_string(&dst_file.as_path())
            .wrap_err(format!("Failed to read '{}'", dst_name))
            .wrap_err("Error while trying to compare existing style file")
            .suggestion(format!(
                "Please delete or fix the existing style file {}",
                dst_name
            ))?;

        if content_src == content_dst {
            log::info!(
                "{} Existing style file matches {}, skipping placement",
                step.next(),
                src_name
            );
            return Ok(None);
        }

        return Err(eyre::eyre!(
            "Existing style file {} does not match provided style file {}",
            dst_name,
            src_name
        )
        .suggestion(format!(
            "Please either delete the file {} or align the contents with {}",
            dst_name, src_name
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

fn setup_jobs(jobs: Option<u8>) -> eyre::Result<()> {
    // configure rayon to use the specified number of threads (globally)
    if let Some(jobs) = jobs {
        let jobs = if jobs == 0 { 1u8 } else { jobs };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs.into())
            .build_global();

        if let Err(err) = pool {
            return Err(err)
                .wrap_err(format!("Failed to create thread pool of size {}", jobs))
                .suggestion("Please try to decrease the number of jobs");
        }
    };
    Ok(())
}

pub fn run(data: cli::Data) -> eyre::Result<()> {
    let start = std::time::Instant::now();

    log::info!(" ");
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

    let candidates =
        globs::build_matchers_from(&data.json.paths, &data.json.root, "paths", &data.json.name)?;
    let filter_pre =
        globs::build_glob_set_from(&data.json.filter_pre, "preFilter", &data.json.name)?;
    let filter_post =
        globs::build_glob_set_from(&data.json.filter_post, "postFilter", &data.json.name)?;

    let (paths, filtered) = globs::match_paths(candidates, filter_pre, filter_post);
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
    // binding for scope guard is not used, but an action needed when the variable goes out of scope
    let _style = scopeguard::guard(style, |path| {
        // ensure we delete the temporary style file at return or panic
        if let Some(path) = path {
            let str = format!("Cleaning up temporary file {}\n", path.to_string_lossy());
            let str = console::style(str).dim().italic();

            log::info!("{}", str);
            let _ = fs::remove_file(path);
        }
    });

    setup_jobs(data.jobs)?;
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

    // preparation for indicatif 0.17
    // pb.set_style(
    //     indicatif::ProgressStyle::with_template(if console::Term::stdout().size().1 > 80 {
    //         "{prefix:>12.cyan.bold} [{bar:26}] {pos}/{len} {wide_msg}"
    //     } else {
    //         "{prefix:>12.cyan.bold} [{bar:26}] {pos}/{len}"
    //     })
    //     .unwrap()
    //     .progress_chars("=> "),
    // );

    if log_pretty() {
        pb.set_prefix("Running");
    }

    let green_bold = console::Style::new().green().bold();

    let paths: Vec<_> = paths.collect();
    let result: eyre::Result<()> = paths.into_par_iter().try_for_each(|path| {
        let print_path = match &strip_root {
            None => &path,
            Some(strip) => path.strip_prefix(strip).unwrap()
        };

        if log_pretty() {
            pb.println(format!(
                "{:>12} {}",
                green_bold.apply_to("Formatting"),
                print_path.to_string_lossy(),
            ));
            pb.inc(1);
        } else {
            log::info!("  + {}", path.to_string_lossy());
        }

        let _ = cmd.format(&path)
            .wrap_err(format!("Failed to format {}", path.to_string_lossy()))
            .suggestion("Please make sure that your style file matches the version of clang-format and that you have the necessary permissions to modify all files")?;
        Ok(())
    });
    result?;

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

    log::info!(" "); // just an empty newline
    Ok(())
}
