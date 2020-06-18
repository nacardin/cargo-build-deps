extern crate clap;
extern crate toml;

use clap::{App, Arg};
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

    println!("Start building packages");

    for dependency in dependencies {
        build_package(&dependency, is_release, &target);
    }

    println!("Finished");
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

fn build_package(pkg_name: &str, is_release: bool, target: &str) {
    println!("Building package: {:?}", pkg_name);

    let mut command = Command::new("cargo");
    let command_with_args = command.arg("build").arg("-p").arg(pkg_name);

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