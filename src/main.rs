extern crate clap;
extern crate json;
extern crate semver;
extern crate toml;

use clap::{App, Arg};
use semver::Version;
use toml::Value as Toml;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

fn main() {
    let matched_args = App::new("cargo build-deps")
        .arg(Arg::with_name("build-deps"))
        .arg(Arg::with_name("release").long("release"))
        .arg(Arg::with_name("target").long("target"))
        .arg(Arg::with_name("skip-update").long("skip-update"))
        .get_matches();

    let is_release = matched_args.is_present("release");
    let target = match matched_args.value_of("target") {
        Some(value) => value,
        None => "",
    };

    if !matched_args.is_present("skip-update") {
        execute_command(Command::new("cargo").arg("update"));
    }

    let cargo_toml = get_toml("Cargo.toml");
    let dependencies = parse_dependencies(&cargo_toml);

    println!("    Start building packages");

    let cargo_metadata = json::parse(
        std::str::from_utf8(
            &Command::new("cargo")
                .arg("metadata")
                .envs(env::vars())
                .output()
                .expect("Couldn't run 'cargo metadata'")
                .stdout,
        )
        .expect("Couldn't get 'cargo metadata' output as utf8"),
    )
    .expect("Couldn't parse 'cargo metadata' output as JSON");

    for dependency in dependencies {
        build_package(&dependency, is_release, &target, &cargo_metadata);
    }

    println!("    Finished");
}

fn get_toml(file_path: &str) -> Toml {
    let mut toml_file = File::open(file_path).expect(&format!("{} is not available", file_path));
    let mut toml_string = String::new();
    toml_file
        .read_to_string(&mut toml_string)
        .expect("Can't read file");
    toml_string.parse().expect("Failed to parse toml")
}

fn parse_dependencies<'a>(toml: &'a Toml) -> Vec<String> {
    match toml.get("dependencies") {
        Some(&Toml::Table(ref pkgs)) => pkgs
            .iter()
            .map(|(name, value)| format_package(name, value))
            .collect(),
        _ => panic!("Failed to find dependencies in Cargo.toml"),
    }
}

fn format_package(name: &String, value: &Toml) -> String {
    match value {
        Toml::String(string) => format!("{}:{}", name, string.replace("\"", "")),
        Toml::Table(table) => {
            let value = match table.get("version") {
                Some(v) => v.to_string().replace("\"", ""),
                None => "".to_string(),
            };
            format!("{}:{}", name, value)
        }
        _ => panic!("Failed to format package-id"),
    }
}

fn build_package(pkg_name: &str, is_release: bool, target: &str, cargo_metadata: &json::JsonValue) {
    let mut split = pkg_name.split(':');
    let pkg_name = split.next().expect("Couldn't get package name");
    let mut pkg_version = split
        .next()
        .expect("Couldn't get package version")
        .to_string();

    // Cargo.toml allows non-semver dependencies.
    // This is an issue, because `cargo build -p <package>:<version> only allows semver format for the version.
    // Thanksfully, `cargo metadata` allows us to find the precise package version.
    if let Err(_) = Version::parse(&pkg_version) {
        println!(
            "    Getting package '{}' semver from 'cargo metadata'",
            pkg_name
        );
        pkg_version.clear();
        pkg_version.push_str(
            cargo_metadata["packages"]
                .members()
                // Only keep matching package names
                .filter(|e| e["name"] == pkg_name)
                // Only keep matching start of package version
                .filter(|e| {
                    e["version"]
                        .as_str()
                        .expect("Could't get version as str")
                        .starts_with(&pkg_version)
                })
                // At this point it's possible that there is more than one result,
                // but I'll handle this case if it causes me trouble in the future
                // Assume that the first one is the good one!
                .next()
                .expect(&format!(
                    "Couldn't find package {} in 'cargo metadata' output",
                    pkg_name
                ))["version"]
                .as_str()
                .expect("Couldn't get version as str"),
        );
    }

    println!("    Building package: {}:{}", pkg_name, pkg_version);

    let mut command = Command::new("cargo");
    let command_with_args = command
        .arg("build")
        .arg("-p")
        .arg(format!("{}:{}", pkg_name, pkg_version));

    let command_with_args = if is_release {
        command_with_args.arg("--release")
    } else {
        command_with_args
    };

    let command_with_args = if !target.is_empty() {
        command_with_args.arg("--target=".to_owned() + target)
    } else {
        command_with_args
    };

    execute_command(command_with_args);
}

fn execute_command(command: &mut Command) {
    let mut child = command
        .envs(env::vars())
        .spawn()
        .expect("Failed to execute process");

    let exit_status = child.wait().expect("Failed to run command");

    if !exit_status.success() {
        match exit_status.code() {
            Some(code) => panic!("Exited with status code: {}", code),
            None => panic!("Process terminated by signal"),
        }
    }
}
