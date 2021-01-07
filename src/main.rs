extern crate toml;
extern crate clap;

use std::env;
use std::io::prelude::*;
use std::fs::File;
use toml::{Value as Toml};
use std::process::Command;
use clap::{
    App,
    Arg,
};

fn main() -> Result<(), String> {
    let matched_args = App::new("cargo build-deps")
        .arg(Arg::with_name("build-deps"))
        .arg(Arg::with_name("features").long("features"))
        .arg(Arg::with_name("all-features").long("all-features"))
        .arg(Arg::with_name("release").long("release"))
        .arg(Arg::with_name("nightly").long("nightly"))
        .arg(Arg::with_name("target").long("target"))
        .get_matches();

    let is_release = matched_args.is_present("release");
    let is_all_features = matched_args.is_present("all-features");
    let is_nightly = matched_args.is_present("nightly");
    let target = match matched_args.value_of("target") {
        Some(value) => value,
        None => ""
    };
    let features = match matched_args.value_of("features") {
        Some(value) => value.split(" ").collect::<Vec<&str>>(),
        None => Vec::new(),
    };

    execute_command(Command::new("cargo").arg("update"))?;

    let cargo_toml = get_toml("Cargo.toml");
    let top_pkg_name = parse_package_name(&cargo_toml);

    let cargo_lock = get_toml("Cargo.lock");
    let deps = parse_deps(&cargo_lock, top_pkg_name)?;

    println!("building packages: {:?}", deps);

    for dep in deps {
        build_package(
            &dep, is_release, &target, is_nightly, features.clone(),
            is_all_features,
        )?
    }

    println!("done");
    Ok(())
}

fn get_toml(file_path: &str) -> Toml {
    let mut toml_file = File::open(file_path).unwrap();
    let mut toml_string = String::new();
    toml_file.read_to_string(&mut toml_string).unwrap();
    toml_string.parse().expect("failed to parse toml")
}

fn parse_package_name(toml: &Toml) -> &str {
    match toml {
        &Toml::Table(ref table) => {
            match table.get("package") {
                Some(&Toml::Table(ref table)) => {
                    match table.get("name") {
                        Some(&Toml::String(ref name)) => name,
                        _ => panic!("failed to parse name"),
                    }
                }
                _ => panic!("failed to parse package"),
            }
        }
        _ => panic!("failed to parse Cargo.toml: incorrect format"),
    }
}

fn cargo_lock_find_package<'a>(toml: &'a Toml, pkg_name: &str) -> Result<&'a Toml, String> {
    match toml.get("package") {
        Some(&Toml::Array(ref pkgs)) => {
            pkgs.iter().find(|pkg| {
                pkg.get("name").map_or(
                    false, |name| name.as_str().unwrap_or("") == pkg_name,
                )
            }).map_or(
                Err(format!("failed to find top package {}", pkg_name)),
                |x| Ok(x),
            )
        }
        _ => Err("failed to find packages in Cargo.lock".to_string()),
    }
}

fn crate_name_version(toml: &Toml, crate_name: &str) -> Result<String, String> {
    let value_pkg = cargo_lock_find_package(
        toml, crate_name,
    )?;
    let crate_version = value_pkg.get("version").map_or(
        Err(format!("Version not found for {}", crate_name)),
        |x| Ok(x),
    )?.as_str().map_or(
        Err(format!("Invalid version field for {}", crate_name)),
        |x| Ok(x),
    )?;
    Ok(format!("{}:{}", crate_name, crate_version))
}

fn parse_deps(toml: &Toml, top_pkg_name: &str) -> Result<Vec<String>, String> {
    match cargo_lock_find_package(toml, top_pkg_name)? {
        &Toml::Table(ref pkg) => {
            match pkg.get("dependencies") {
                Some(&Toml::Array(ref deps_toml_array)) => {
                    deps_toml_array.iter()
                        .map(|value| {
                            if let Some(crate_name) = value.as_str() {
                                crate_name_version(toml, crate_name)
                            } else {
                                Err("Empty dependency".to_string())
                            }
                        })
                        .collect()
                }
                _ => Err("error parsing dependencies table".to_string()),
            }
        }
        _ => Err("error parsing dependencies table".to_string()),
    }
}

fn build_package(
    pkg_name: &str, is_release: bool, target: &str, is_nightly: bool, features: Vec<&str>,
    is_all_features: bool,
) -> Result<(), String> {
    println!("building package: {:?}", pkg_name);

    let mut command = Command::new("cargo");

    let command_with_args = if is_nightly {
        command.arg("+nightly").arg("build")
    } else {
        command.arg("build")
    }.arg("-p").arg(pkg_name);

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

    let command_with_args = if is_all_features {
        command_with_args.arg("--all-features")
    } else if features.len() > 0 {
        command_with_args.arg("--features").arg(format!("\"{}\"", features.join(" ")))
    } else {
        command_with_args
    };

    execute_command(command_with_args)
}

fn execute_command(command: &mut Command) -> Result<(), String> {
    let mut child = command.envs(env::vars()).spawn()
        .map_err(|_| "failed to execute process".to_string())?;

    let exit_status = child.wait().expect("failed to run command");

    if !exit_status.success() {
        match exit_status.code() {
            Some(code) => Err(format!("Exited with status code: {}", code)),
            None => Err(format!("Process terminated by signal")),
        }
    } else {
        Ok(())
    }
}
