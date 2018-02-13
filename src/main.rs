extern crate toml;

use std::env;
use std::io::prelude::*;
use std::fs::File;
use toml::Value as Toml;
use std::process::Command;

fn main() {
    execute_command(Command::new("cargo").arg("update"));

    let cargo_toml = get_toml("Cargo.toml");
    let top_pkg_name = parse_package_name(&cargo_toml);

    let cargo_lock = get_toml("Cargo.lock");
    let deps = parse_deps(&cargo_lock, top_pkg_name);

    println!("building packages: {:?}", deps);

    for dep in deps {
        build_package(&dep);
    }

    println!("done");
}

fn get_toml(file_path: &str) -> Toml {
    let mut toml_file = File::open(file_path).unwrap();
    let mut toml_string = String::new();
    toml_file.read_to_string(&mut toml_string).unwrap();
    toml_string.parse().expect("failed to parse toml")
}

fn parse_package_name(toml: &Toml) -> &str {
    match toml {
        &Toml::Table(ref table) => match table.get("package") {
            Some(&Toml::Table(ref table)) => match table.get("name") {
                Some(&Toml::String(ref name)) => name,
                _ => panic!("failed to parse name"),
            },
            _ => panic!("failed to parse package"),
        },
        _ => panic!("failed to parse Cargo.toml: incorrect format"),
    }
}

fn parse_deps<'a>(toml: &'a Toml, top_pkg_name: &str) -> Vec<String> {
    match toml.get("package") {
        Some(&Toml::Array(ref pkgs)) => {
            let top_pkg = pkgs.iter()
                .find(|pkg| pkg.get("name").unwrap().as_str().unwrap() == top_pkg_name);
            match top_pkg {
                Some(&Toml::Table(ref pkg)) => match pkg.get("dependencies") {
                    Some(&Toml::Array(ref deps_toml_array)) => deps_toml_array
                        .iter()
                        .map(|value| {
                            let mut value_parts = value.as_str().unwrap().split(" ");
                            format!(
                                "{}:{}",
                                value_parts
                                    .next()
                                    .expect("failed to parse name from depencency string"),
                                value_parts
                                    .next()
                                    .expect("failed to parse version from depencency string")
                            )
                        })
                        .collect(),
                    _ => panic!("error parsing dependencies table"),
                },
                _ => panic!("failed to find top package"),
            }
        }
        _ => panic!("failed to find packages in Cargo.lock"),
    }
}

fn build_package(pkg_name: &str) {
    println!("building package: {:?}", pkg_name);
    execute_command(Command::new("cargo").arg("build").arg("-p").arg(pkg_name).arg("--release"));
}

fn execute_command(command: &mut Command) {
    let mut child = command.envs(env::vars()).spawn().expect("failed to execute process");

    let exit_status = child.wait().expect("failed to run command");

    if !exit_status.success() {
        match exit_status.code() {
            Some(code) => panic!("Exited with status code: {}", code),
            None => panic!("Process terminated by signal"),
        }
    }
}
