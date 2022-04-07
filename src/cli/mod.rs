use std::{path, process};

mod handlers;
mod logging;
pub mod utils;

use clap::{arg, crate_authors, crate_description, crate_name, crate_version};
#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;

#[derive(Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")] // removed: deny_unknown_fields
pub struct JsonModel {
    /// List of paths as globs
    pub paths: Vec<String>,
    /// List of globs to use for filtering (global blacklist)
    pub blacklist: Option<Vec<String>>,
    /// Optional path to a `.clang-format` style file (can be specified via --style)
    pub style_file: Option<path::PathBuf>,
    /// Optional path where the `.clang-format` file should be copied to while executing
    pub style_root: Option<path::PathBuf>,
    /// Optional path to the `clang-format` executable or command name
    pub command: Option<path::PathBuf>,

    #[serde(skip)]
    /// Parent directory of the Json file, used to resolve paths specified within
    pub root: path::PathBuf,
    #[serde(skip)]
    /// Lossy Json filename
    pub name: String,
}

#[derive(Debug)]
pub struct Data {
    /// Json input data
    pub json: JsonModel,
    /// Command-line override for the style file
    pub style: Option<path::PathBuf>,
    /// Command-line override for the clang-format executable
    pub command: Option<path::PathBuf>,
    /// Command-line parameter for the number of jobs to use for executing clang-format
    /// If `None` then all available jobs should be used, else the specified number of jobs.
    pub jobs: Option<u8>,
}

pub struct Builder {
    pub matches: clap::ArgMatches,
}

impl Builder {
    fn app() -> clap::Command<'static> {
        clap::Command::new(crate_name!())
            .arg_required_else_help(true)
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
                arg!(-s --style ... "Optional path to .clang-format style file. \
                                     Overrides <JSON> configuration")
                .allow_invalid_utf8(true)
                .takes_value(true)
                .required(false),
            )
            .arg(
                arg!(-c --command ... "Optional path to executable or clang-format command. \
                                       Overrides <JSON> configuration, defaults to `clang-format`")
                // .default_value("clang-format")
                .allow_invalid_utf8(true)
                .takes_value(true)
                .required(false),
            )
            .arg(
                arg!(-j --jobs ... "Optional parameter to define the number of jobs to use. \
                                    If provided without value (e.g., '-j') all available logical \
                                    cores are used. Maximum value is 255")
                .default_value("1")
                .takes_value(true)
                .min_values(0)
                .max_values(1)
                .required(false),
            )
            .arg(
                arg!(-v --verbose ... "Verbosity, use -vv... for verbose output.")
                    .global(true)
                    .multiple_values(false),
            )
            .arg(arg!(-q --quiet "Suppress all output except for errors; overrides -v"))
            .subcommand_negates_reqs(true)
            .subcommand(
                clap::Command::new("schema")
                    .about("Print the schema used for the <JSON> configuration file"),
            )
    }

    pub fn build() -> Builder {
        let cmd = Builder::app();
        let builder = Builder {
            matches: cmd.get_matches(),
        };
        logging::setup(&builder.matches);
        builder
    }

    pub fn parse(self) -> eyre::Result<Data> {
        if self.matches.subcommand_matches("schema").is_some() {
            // let _ = Builder::app().print_help();
            // println!(
            //     "\n\nThe following schema is used for <JSON>:\n{}",
            //     JsonModel::schema(),
            // );
            println!("{}", JsonModel::schema(),);
            process::exit(0);
        }

        let json_path = self.path_for_key("JSON", true)?;
        let json = JsonModel::load(&json_path).wrap_err("Invalid parameter <JSON>")?;

        let style = match self.matches.is_present("style") {
            false => None,
            true => {
                let style_path = self.path_for_key("style", true)?;
                let path = utils::file_with_name_or_ext(&style_path, ".clang-format")
                    .wrap_err("Invalid parameter --style")?;
                Some(path)
            }
        };

        // we're not yet validating the command here, since the same procedure is applied for the file
        let command = match self.matches.value_of_os("command") {
            None => None,
            Some(_) => Some(self.path_for_key("command", false)?),
        };

        // cannot use "and" since it is not lazily evaluated, and cannot use "and_then" nicely
        // since the question mark operator does not work in closures
        // let command = self
        //     .matches
        //     .value_of_os("command")
        //     .and_then(|_| Some(self.path_for_key("command", false)?));

        // unwrap is safe to call since jobs has a default value
        let jobs = {
            let mut val = self.matches.values_of("jobs").unwrap();
            if val.len() == 0 {
                None
            } else {
                let val: u8 = val
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(|_| eyre!("Invalid parameter --jobs"))
                    .suggestion("Please provide a number in the range [0 .. 255]")?;
                Some(val)
            }
        };

        Ok(Data {
            json,
            style,
            command,
            jobs,
        })
    }

    fn path_for_key(&self, key: &str, check_exists: bool) -> eyre::Result<path::PathBuf> {
        let path = self
            .matches
            .value_of_os(key)
            .map(std::path::PathBuf::from)
            .ok_or(eyre!(format!(
                "Could not convert parameter '{}' to path",
                key
            )))?;

        if check_exists {
            return utils::path_or_err(path);
        }
        Ok(path)
    }
}

impl JsonModel {
    fn schema() -> String {
        let schema = schema_for!(JsonModel);
        serde_json::to_string_pretty(&schema).unwrap()
    }

    fn load<P>(path: P) -> eyre::Result<JsonModel>
    where
        P: AsRef<path::Path>,
    {
        let json_path = utils::file_with_ext(path.as_ref(), "json", true)?;
        let json_name = json_path.to_string_lossy();

        let f = std::fs::File::open(path.as_ref())
            .wrap_err(format!("Failed to open provided JSON file '{}'", json_name))?;

        let mut json: JsonModel = serde_json::from_reader(std::io::BufReader::new(f))
            .wrap_err(format!("Validation failed for '{}'", json_name))
            .suggestion(format!(
        "Please make sure that '{}' is a valid .json file and the contents match the required schema.",
        json_name))?;

        json.root = path::PathBuf::from(json_path.canonicalize().unwrap().parent().unwrap());
        json.name = json_path.to_string_lossy().into();
        Ok(json)
    }
}
