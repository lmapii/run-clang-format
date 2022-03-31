mod logging;
mod utils;

use std::path;
use std::process;

use clap::{arg, crate_authors, crate_description, crate_name, crate_version};
#[allow(unused_imports)]
use color_eyre::{eyre::eyre, eyre::WrapErr, Help};

use schemars::{schema_for, JsonSchema};
use serde::Deserialize;

#[derive(Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JsonModel {
    pub paths: Vec<String>, // TODO: utf-8 ?
    pub blacklist: Option<Vec<String>>,
    pub style: Option<path::PathBuf>,

    #[serde(skip)]
    pub root: path::PathBuf,
    #[serde(skip)]
    pub name: String,
}

#[derive(Debug)]
pub struct Data {
    pub json: JsonModel,
    pub style: Option<path::PathBuf>,
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
                arg!(-s --style ... "Optional path to .clang-format file")
                    .allow_invalid_utf8(true)
                    .takes_value(true)
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
                    .about("Shows help and prints the schema used for <JSON>"),
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

    pub fn parse(self) -> eyre::Result<Data, eyre::Report> {
        if let Some(_) = self.matches.subcommand_matches("schema") {
            let _ = Builder::app().print_help();
            println!(
                "\n\nThe following schema is used for <JSON>:\n{}",
                JsonModel::schema(),
            );
            process::exit(0);
        }

        let json_path = path_for_key(&self.matches, "JSON")?;
        let style_path = path_for_key(&self.matches, "style")?;

        let json = JsonModel::load(&json_path).wrap_err("Invalid parameter <JSON>")?;

        let style = match self.matches.is_present("style") {
            false => None,
            true => {
                let path = utils::file_with_name(&style_path, ".clang-format")
                    .wrap_err(format!("Invalid parameter --style"))?;
                Some(path)
            }
        };

        Ok(Data { json, style })
    }
}

impl JsonModel {
    fn schema() -> String {
        let schema = schema_for!(JsonModel);
        serde_json::to_string_pretty(&schema).unwrap()
    }

    fn load<P>(path: P) -> eyre::Result<JsonModel, eyre::Report>
    where
        P: AsRef<path::Path>,
    {
        let json_path = utils::file_with_ext(path.as_ref(), "json")?;
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

pub fn path_for_key(
    matches: &clap::ArgMatches,
    key: &str,
) -> eyre::Result<path::PathBuf, eyre::Report> {
    let path = matches
        .value_of_os(key)
        .map(std::path::PathBuf::from)
        .ok_or(eyre!(format!(
            "Could not convert parameter {} to path",
            key
        )))?;

    utils::path_or_err(path)
}
