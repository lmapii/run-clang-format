// input
// https://github.com/mattgathu/duma/blob/master/tests/
// https://crates.io/crates/assert_cmd

use std::path;

use assert_cmd::Command;
use clap::crate_name;

fn cmd() -> Command {
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    cmd.env_clear();
    cmd.env_remove("PATH");
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
    if !cfg!(linux) {
        // TODO: cmd() does not seem to properly clear the path in linux
        cmd().arg(json.as_os_str()).assert().failure();
    }
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
fn invoke_json_command() {
    let combinations = vec![
        // path to command does not exist
        ("test-files/json/test-err-invalid-command.json", false),
        // path to command exists, but it is not an executable
        ("test-files/json/test-err-invalid-command-file.json", false),
        // command is not a path and an invalid executable name
        ("test-files/json/test-err-invalid-command-name.json", false),
        // valid command has been provided as path
        ("test-files/json/test-ok-style-and-command.json", true),
    ];

    for test in combinations.into_iter() {
        println!("checking {}", test.0);
        let json = crate_root_rel(test.0);
        // using command WITHOUT path
        run_cmd_and_assert(&mut cmd().arg(json.as_os_str()), test.1);
    }

    // test that also a valid executable name can be provided as command field (requires $PATH)
    let json = crate_root_rel("test-files/json/test-ok-style-and-command-name.json");
    run_cmd_and_assert(&mut cmd_with_path().arg(json.as_os_str()), true);
}

#[test]
fn invoke_json_glob() {
    // test that an invalid glob leads to an error
    let json = crate_root_rel("test-files/json/test-err-invalid-glob.json");
    run_cmd_and_assert(&mut cmd_with_path().arg(json.as_os_str()), false);
}

#[test]
fn invoke_arg_style() {
    // given: a valid .json configuration file
    let json = crate_root_rel("test-files/json/test-ok-style.json");

    // paired with an invalid --style parameter, leads to an error (overrides valid .json)
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

#[test]
fn invoke_arg_command() {
    // given: a valid .json configuration file
    let json = crate_root_rel("test-files/json/test-ok-style-and-command.json");

    // paired with an invalid --command parameter, leads to an error (overrides valid .json)
    run_cmd_and_assert(
        &mut cmd().arg(json.as_os_str()).arg("--command i/do/not/exist"),
        false,
    );

    // paired with an valid path as --command parameter, success
    run_cmd_and_assert(
        &mut cmd().arg(json.as_os_str()).arg(format!(
            "--command={}",
            crate_root_rel("artifacts/clang/clang-format").to_string_lossy()
        )),
        true,
    );

    // paired with an valid COMMAND as --command parameter, success
    run_cmd_and_assert(
        &mut cmd_with_path()
            .arg(json.as_os_str())
            .arg(format!("--command=clang-format")),
        true,
    );

    let json = crate_root_rel("test-files/json/test-err-invalid-command.json");
    // a valid --command parameter even overrides an invalid json configuration file
    run_cmd_and_assert(
        &mut cmd_with_path()
            .arg(json.as_os_str())
            .arg(format!("--command=clang-format")),
        true,
    );
}

#[test]
fn invoke_quiet() {
    fn assert_quiet(cmd: &mut Command, expect_quiet: bool) {
        let output = cmd.output().unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();

        println!("status: {}", output.status);
        println!("{}", stdout);
        println!("{}", stderr);

        if expect_quiet {
            assert_eq!(0, stdout.len());
            assert_eq!(0, stderr.len());
        } else {
            assert_ne!(0, stderr.len());
        }
    }

    assert_quiet(
        &mut cmd_with_path()
            .arg(crate_root_rel("test-files/json/test-ok-style.json").as_os_str())
            .arg("-vvvv")
            .arg("--quiet"),
        true,
    );

    assert_quiet(
        &mut cmd_with_path()
            .arg(crate_root_rel("test-files/json/test-err-empty.json").as_os_str())
            .arg("-vvvv")
            .arg("--quiet"),
        false,
    );
}
