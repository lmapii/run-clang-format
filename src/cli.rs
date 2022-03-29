use clap::{arg, crate_authors, crate_description, crate_name, crate_version};

use env_logger::fmt;
use std::io::Write;

pub fn build() -> clap::Command<'static> {
    let cmd = clap::Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            arg!(<JSON>)
                .help("Path/configuration as .json")
                // invalid UTF-8 characters must be allowed since we'll be using value_of_os
                // and paths do not necessarily only contain valid UTF-8 characters.
                .allow_invalid_utf8(true),
        )
        .arg(
            arg!(-s --style ... "Optional path to .clang-format file")
                .allow_invalid_utf8(true)
                .takes_value(true)
                .required(false),
        )
        .arg(
            arg!(-v --verbose ... "Verbosity, use -vv... for verbose output.")
                .multiple_values(false),
        )
        .arg(arg!(-q --quiet "Suppress all output except for errors; overrides -v"));

    cmd
}

fn log_level(matches: &clap::ArgMatches) -> log::Level {
    let lvl = if matches.is_present("quiet") {
        log::Level::Error
    } else {
        match matches.occurrences_of("verbose") {
            // ArgMatches::occurrences_of which will return 0 if the argument was not used at
            // runtime. This demo always displays error or warning messages, so by default -v is
            // always used. The --quiet option must be used to silence all.
            // _ => log::Level::Error,
            // _ => log::Level::Warn,
            0 | 1 => log::Level::Info,
            2 => log::Level::Debug,
            3 | _ => log::Level::Trace,
        }
    };
    lvl
}

pub fn setup(matches: &clap::ArgMatches) {
    let lvl = log_level(matches);

    env_logger::Builder::new()
        .format(move |f, record| {
            // Color::White renders as gray on black background terminals
            let mut s = f.style();
            let (lvl_str, s) = match record.level() {
                log::Level::Error => ("<e>", s.set_bold(true).set_color(fmt::Color::Red)),
                log::Level::Warn => ("<w>", s.set_bold(true).set_color(fmt::Color::Yellow)),
                log::Level::Info => ("<i>", s.set_bold(false).set_color(fmt::Color::White)),
                log::Level::Debug => ("<d>", s.set_bold(false).set_color(fmt::Color::Blue)),
                log::Level::Trace => ("<t>", s.set_bold(false).set_color(fmt::Color::Magenta)),
            };

            let (target, tstamp) = match lvl {
                l if l >= log::Level::Debug => (record.module_path(), f.timestamp_millis()),
                _ => (None, f.timestamp_seconds()),
            };

            write!(f, "{} {}", s.value(tstamp), s.value(lvl_str))?;
            if let Some(target) = target {
                write!(f, " {}", target)?;
            }
            writeln!(f, " {}", s.value(record.args()))
        })
        .filter_level(lvl.to_level_filter())
        .init();

    if lvl >= log::Level::Debug {
        std::env::set_var("RUST_SPANTRACE", "1");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    color_eyre::config::HookBuilder::default()
        .display_env_section(std::env::var("DISPLAY_LOCATION").is_ok())
        .install()
        .unwrap();
}
