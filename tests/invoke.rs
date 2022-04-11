// input
// https://github.com/mattgathu/duma/blob/master/tests/
// https://crates.io/crates/assert_cmd

use std::path;

use assert_cmd::Command;
use clap::crate_name;

fn cmd() -> Command {
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    cmd.env_clear();
    cmd
}

fn cmd_with_path() -> Command {
    let mut cmd = cmd();
    cmd.env("PATH", crate_root().join("artifacts/clang"));
    cmd
}

fn crate_root() -> path::PathBuf {
    path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn crate_root_rel(path: &str) -> path::PathBuf {
    crate_root().join(path)
}

#[test]
fn invoke_subs() {
    // an empty command fails since <JSON> is required
    cmd().assert().failure();

    // sub-commands need no parameters.
    let empty_ok = vec!["help", "schema", "--version"];
    for arg in empty_ok.into_iter() {
        cmd().arg(arg).assert().success();
    }
}

fn run_cmd_and_assert(cmd: &mut Command, should_pass: bool) {
    let output = cmd.output().unwrap();

    println!("status: {}", output.status);
    println!("{}", String::from_utf8(output.stdout).unwrap());
    println!("{}", String::from_utf8(output.stderr).unwrap());

    assert_eq!(output.status.success(), should_pass);
}

#[test]
fn invoke_json_and_bin() {
    // empty .json file is not accepted
    let json = crate_root_rel("test-files/json/test-err-empty.json");
    cmd().arg(json.as_os_str()).assert().failure();

    let json = crate_root_rel("test-files/json/test-ok-empty-paths.json");
    // .json file with empty paths is accepted, but clang-format is not in the $PATH
    cmd().arg(json.as_os_str()).assert().failure();
    // as soon as we add the path to clang-format to $PATH the execution is successful
    cmd_with_path().arg(json.as_os_str()).assert().success();
}

#[test]
fn invoke_json_style() {
    let combinations = vec![
        // path to styleFile does not exist
        ("test-files/json/test-err-invalid-style-path.json", false),
        // path to styleFile exists, but this is not a style file
        ("test-files/json/test-err-invalid-style-file.json", false),
        // path to styleFile exists, file has name ".clang-format", but no 'styleRoot' exists
        ("test-files/json/test-err-no-root.json", false),
        // path to styleFile exists, file has name ".clang-format", but 'styleRoot' is an invalid path
        ("test-files/json/test-err-invalid-root.json", false),
        // path to styleFile exists, file has name ".clang-format", and 'styleRoot' exists
        ("test-files/json/test-ok-style.json", true),
        // path to styleFile exists, file has name "named.clang-format", and 'styleRoot' exists
        ("test-files/json/test-ok-style-named.json", true),
    ];

    for test in combinations.into_iter() {
        println!("checking {}", test.0);
        let json = crate_root_rel(test.0);
        run_cmd_and_assert(&mut cmd_with_path().arg(json.as_os_str()), test.1);
    }
}

#[test]
fn invoke_arg_style() {
    // given: a valid .json configuration file
    let json = crate_root_rel("test-files/json/test-ok-style.json");

    // paired with an invalid --style parameter, leads to an error (would override .json)
    run_cmd_and_assert(
        &mut cmd_with_path()
            .arg(json.as_os_str())
            .arg("--style i/do/not/exist.clang-format"),
        false,
    );

    // paired with an valid --style parameter, success
    run_cmd_and_assert(
        &mut cmd_with_path().arg(json.as_os_str()).arg(format!(
            "--style={}",
            crate_root_rel("test-files/clang-format/named.clang-format").to_string_lossy()
        )),
        true,
    );

    let json = crate_root_rel("test-files/json/test-err-invalid-style-file.json");
    // a valid --style parameter even overrides an invalid json configuration file
    run_cmd_and_assert(
        &mut cmd_with_path().arg(json.as_os_str()).arg(format!(
            "--style={}",
            crate_root_rel("test-files/clang-format/named.clang-format").to_string_lossy()
        )),
        true,
    );
}

// TODO: test that --quiet really does not output anything
