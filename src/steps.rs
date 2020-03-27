use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::vec::Vec;

use log::debug;
use pathdiff::diff_paths;

use crate::cargo::CargoManifest;
use crate::errors::CargoPlayError;
use crate::opt::{CargoAction, CargoProfile, Dependency, RustEdition};

pub fn parse_inputs(inputs: &[PathBuf]) -> Result<Vec<String>, CargoPlayError> {
    inputs
        .iter()
        .map(File::open)
        .map(|res| match res {
            Ok(mut fp) => {
                let mut buf = String::new();
                fp.read_to_string(&mut buf)?;
                Ok(buf)
            }
            Err(e) => Err(CargoPlayError::from(e)),
        })
        .collect()
}

// TODO split this into dev-dependencies and dependencies
// //# dev: criterion = "*"
// //# dev: flate2 = {}
// //# dev: tar = ""
pub fn extract_headers(files: &[String]) -> Vec<Dependency> {
    files
        .iter()
        .map(|file: &String| -> Vec<Dependency> {
            file.lines()
                .skip_while(|line| line.starts_with("#!") || line.is_empty())
                .take_while(|line| line.starts_with("//#"))
                .map(|line| line[3..].trim_start().into())
                .filter(|s: &String| !s.is_empty())
                .map(|s: String| Dependency::from(s))
                .collect()
        })
        .flatten()
        .collect()
}

pub fn temp_dir(name: PathBuf) -> PathBuf {
    let mut temp = PathBuf::new();
    temp.push(env::temp_dir());
    temp.push(name);
    temp
}

/// This function ignores the error intentionally.
pub fn rmtemp(temp: &PathBuf) {
    debug!("Cleaning temporary folder at: {:?}", temp);
    let _ = std::fs::remove_dir_all(temp);
}

pub fn mktemp(temp: &PathBuf) {
    debug!("Creating temporary building folder at: {:?}", temp);
    if std::fs::create_dir(temp).is_err() {
        debug!("Temporary directory already exists.");
    }
}

pub fn write_cargo_toml(
    dir: &PathBuf,
    name: String,
    dependencies: Vec<Dependency>,
    edition: RustEdition,
    infers: HashSet<String>,
) -> Result<(), CargoPlayError> {
    let mut manifest = CargoManifest::new(name, dependencies, edition)?;
    let mut cargo = File::create(dir.join("Cargo.toml"))?;

    manifest.add_infers(infers);

    cargo.write_all(&toml::to_vec(&manifest).map_err(CargoPlayError::from_serde)?)?;

    Ok(())
}

/// Copy all the passed in sources to the temporary directory. The first in the list will be
/// treated as main.rs.
pub fn copy_sources(temp: &PathBuf, sources: &[PathBuf]) -> Result<(), CargoPlayError> {
    let destination = temp.join("src");
    std::fs::create_dir_all(&destination)?;

    let mut files = sources.iter();
    let base = if let Some(first) = files.next() {
        let dst = destination.join("main.rs");
        debug!("Copying {:?} => {:?}", first, dst);
        std::fs::copy(first, dst)?;
        first.parent()
    } else {
        None
    };

    if let Some(base) = base {
        files
            .map(|file| -> Result<(), CargoPlayError> {
                let part = diff_paths(file, base)
                    .ok_or_else(|| CargoPlayError::DiffPathError(file.to_owned()))?;
                let dst = destination.join(part);

                // ensure the parent folder all exists
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(&parent);
                }

                debug!("Copying {:?} => {:?}", file, dst);
                std::fs::copy(file, dst).map(|_| ()).map_err(From::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
    }

    Ok(())
}

#[inline]
pub fn run_cargo_build(
    toolchain: Option<String>,
    project: &PathBuf,
    release: bool,
    cargo_option: Option<String>,
    program_args: &[String],
) -> Result<ExitStatus, CargoPlayError> {
    let profile = if release {
        CargoProfile::Release
    } else {
        CargoProfile::Debug
    };

    run_cargo_action(
        toolchain,
        project,
        CargoAction::Run(profile),
        cargo_option,
        program_args,
    )
}

#[inline]
pub fn run_cargo_test(
    toolchain: Option<String>,
    project: &PathBuf,
    cargo_option: Option<String>,
    program_args: &[String],
) -> Result<ExitStatus, CargoPlayError> {
    run_cargo_action(
        toolchain,
        project,
        CargoAction::Test,
        cargo_option,
        program_args,
    )
}

pub fn run_cargo_action(
    toolchain: Option<String>,
    project: &PathBuf,
    action: CargoAction,
    cargo_option: Option<String>,
    program_args: &[String],
) -> Result<ExitStatus, CargoPlayError> {
    let mut cargo = Command::new("cargo");

    if let Some(toolchain) = toolchain {
        cargo.arg(format!("+{}", toolchain));
    }

    match action {
        CargoAction::Run(CargoProfile::Release) => {
            cargo.arg("run").arg("--release");
        }
        CargoAction::Run(CargoProfile::Debug) => {
            cargo.arg("run");
        }
        CargoAction::Test => {
            cargo.arg("test");
        }
        // TODO : bench are unsupported for now
        _ => {}
    }

    cargo.arg("--manifest-path").arg(project.join("Cargo.toml"));

    if let Some(cargo_option) = cargo_option {
        cargo.args(cargo_option.split_ascii_whitespace());
    }

    cargo
        .arg("--")
        .args(program_args)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .map_err(From::from)
}

pub fn copy_project<T: AsRef<Path>, U: AsRef<Path>>(
    from: T,
    to: U,
) -> Result<ExitStatus, CargoPlayError> {
    let to = to.as_ref();

    if to.is_dir() {
        return Err(CargoPlayError::PathExistError(to.to_path_buf()));
    }

    Command::new("cp")
        .arg("-R")
        .arg(from.as_ref())
        .arg(&to)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .map(|x| {
            // At this point we are certain the `to` path exists
            println!(
                "Generated project at {}",
                to.canonicalize().unwrap().display()
            );
            x
        })
        .map_err(From::from)
}
