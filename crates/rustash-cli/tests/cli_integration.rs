use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

fn rustash_cmd() -> Command {
    Command::cargo_bin("rustash").unwrap()
}

#[test]
fn test_help_message() {
    let mut cmd = rustash_cmd();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A developer-first, multi-modal data stash.",
        ));
}

#[test]
fn test_stash_add_and_list() {
    let dir = tempdir().unwrap();
    let mut cmd = rustash_cmd();

    // Override home to isolate config
    cmd.env("HOME", dir.path());

    // Add a stash
    cmd.args([
        "stash",
        "add",
        "test_snippets",
        "--service-type",
        "Snippet",
        "--database-url",
        "sqlite:test.db",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Stash 'test_snippets' added."))
    .stdout(predicate::str::contains("set as the default"));

    // List stashes
    let mut list_cmd = rustash_cmd();
    list_cmd
        .env("HOME", dir.path())
        .arg("stash")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test_snippets"))
        .stdout(predicate::str::contains("(default)"));
}

#[test]
fn test_no_default_stash_error() {
    let dir = tempdir().unwrap();
    let mut cmd = rustash_cmd();

    // Override home to isolate config
    cmd.env("HOME", dir.path());

    // Try to list snippets without any stash configured
    cmd.arg("snippets")
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "No stash specified and no default_stash is set.",
        ));
}

#[test]
fn test_snippets_add_and_list() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("snippets.db");
    let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

    // Configure a default stash
    let mut setup_cmd = rustash_cmd();
    setup_cmd
        .env("HOME", dir.path())
        .args([
            "stash",
            "add",
            "default",
            "--service-type",
            "Snippet",
            "--database-url",
            &db_url,
        ])
        .assert()
        .success();

    // Add a snippet
    let mut add_cmd = rustash_cmd();
    add_cmd
        .env("HOME", dir.path())
        .args([
            "snippets",
            "add",
            "--title",
            "My First Snippet",
            "--content",
            "echo 'Hello, Rustash!'",
            "--tags",
            "rust,cli",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added snippet 'My First Snippet'"));

    // List snippets
    let mut list_cmd = rustash_cmd();
    list_cmd
        .env("HOME", dir.path())
        .args(["snippets", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("My First Snippet"))
        .stdout(predicate::str::contains("rust, cli"));
}

#[test]
fn test_cli_version() {
    let mut cmd = rustash_cmd();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rustash-cli"));
}
