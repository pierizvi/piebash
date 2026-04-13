use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn piebash_command() -> (Command, TempDir) {
    let temp_home = TempDir::new().expect("temp home");
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("piebash"));

    cmd.env("HOME", temp_home.path())
        .env("USERPROFILE", temp_home.path());

    (cmd, temp_home)
}

#[test]
fn alias_expands_before_execution() {
    let (mut cmd, _temp_home) = piebash_command();

    cmd.write_stdin("alias ll='echo hi'\nll\nexit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("hi\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn quoted_operators_are_not_parsed_as_control_tokens() {
    let (mut cmd, _temp_home) = piebash_command();

    cmd.write_stdin("echo \"a;b\"\necho \"a|b\"\nexit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("a;b\n").and(predicate::str::contains("a|b\n")),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn builtin_pipeline_passes_output_to_wc() {
    let (mut cmd, _temp_home) = piebash_command();

    cmd.write_stdin("echo hi | wc\nexit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("       1        1        3"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn cargo_non_install_commands_use_external_cargo() {
    let (mut cmd, _temp_home) = piebash_command();

    cmd.write_stdin("cargo --version\nexit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("cargo ")
                .and(predicate::str::contains("runtime not found").not()),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn yes_is_registered_as_a_builtin() {
    let (mut cmd, _temp_home) = piebash_command();

    cmd.write_stdin("type yes\nexit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("yes is a shell builtin"))
        .stderr(predicate::str::is_empty());
}
