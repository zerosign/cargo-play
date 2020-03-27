use std::convert::Infallible;
use std::ffi::{OsStr, OsString};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::vec::Vec;
use structopt::StructOpt;

use crate::errors::CargoPlayError;

#[derive(Debug, PartialEq)]
pub enum Dependency {
    Build(String),
    Test(String),
}

impl FromStr for Dependency {
    type Err = Infallible;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Ok(From::from(String::from(raw)))
    }
}

impl From<String> for Dependency {
    fn from(line: String) -> Self {
        // check string "dev:" if first string not "dev:"
        // then it should be build time dependency,
        // however if dev: is not the first then return an error
        // we don't need to check whether the next package definition are correct or not
        // since it's already being checked by cargo itself
        println!("line: {}", &line[0..4]);

        match &line[0..4] {
            "dev:" => Dependency::Test(line[4..].trim_start().into()),
            _ => Dependency::Build(String::from(line)),
        }
    }
}

#[derive(Debug)]
pub enum RustEdition {
    E2015,
    E2018,
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = "cargo profile could be `release`, `debug` or `profile`")]
pub enum CargoProfile {
    Release,
    Debug,
    Profile,
}

impl FromStr for CargoProfile {
    type Err = CargoPlayError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "release" => Ok(CargoProfile::Release),
            "debug" => Ok(CargoProfile::Debug),
            "profile" => Ok(CargoProfile::Profile),
            _ => Err(Self::Err::InvalidCargoProfile(String::from(raw))),
        }
    }
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = "cargo action, could be either `run` or `test`")]
pub enum CargoAction {
    // release or not
    Run(CargoProfile),
    Test,
}

impl FromStr for CargoAction {
    type Err = CargoPlayError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.starts_with("run") {
            Ok(CargoAction::Run(CargoProfile::from_str(&raw[4..])?))
        } else if raw.starts_with("test") {
            Ok(CargoAction::Test)
        } else {
            Err(Self::Err::InvalidCargoAction(String::from(raw)))
        }
    }
}

impl Default for CargoAction {
    fn default() -> Self {
        Self::Run(CargoProfile::Debug)
    }
}

impl FromStr for RustEdition {
    type Err = CargoPlayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "2018" {
            Ok(RustEdition::E2018)
        } else if s == "2015" {
            Ok(RustEdition::E2015)
        } else {
            Err(CargoPlayError::InvalidEdition(s.into()))
        }
    }
}

impl Into<String> for RustEdition {
    fn into(self) -> String {
        match self {
            RustEdition::E2015 => "2015".into(),
            RustEdition::E2018 => "2018".into(),
        }
    }
}

impl Default for RustEdition {
    fn default() -> Self {
        RustEdition::E2018
    }
}

#[derive(Debug, StructOpt, Default)]
#[structopt(
    name = "cargo-play",
    about = "Run your Rust program without Cargo.toml"
)]
pub struct Opt {
    #[structopt(short = "d", long = "debug", hidden = true)]
    pub debug: bool,
    #[structopt(short = "c", long = "clean")]
    /// Rebuild the cargo project without the cache from previous run
    pub clean: bool,
    #[structopt(short = "t", long = "toolchain", hidden = true)]
    pub toolchain: Option<String>,
    #[structopt(
        parse(try_from_os_str = "osstr_to_abspath"),
        raw(required = "true", validator = "file_exist")
    )]
    /// Paths to your source code files
    pub src: Vec<PathBuf>,
    #[structopt(
        short = "e",
        long = "edition",
        default_value = "2018",
        raw(possible_values = r#"&["2015", "2018"]"#)
    )]
    /// Specify Rust edition
    pub edition: RustEdition,
    #[structopt(long = "cached", hidden = true)]
    pub cached: bool,
    #[structopt(long = "cargo-action")]
    /// Cargo action
    pub cargo_action: Option<CargoAction>,
    #[structopt(long = "cargo-option")]
    /// Custom flags passing to cargo
    pub cargo_option: Option<String>,
    #[structopt(long = "save")]
    /// Generate a Cargo project based on inputs
    pub save: Option<PathBuf>,
    /// [experimental] Automatically infers dependency
    #[structopt(long = "infer", short = "i")]
    pub infer: bool,
    #[structopt(multiple = true, last = true)]
    /// Arguments passed to the underlying program
    pub args: Vec<String>,
}

impl Opt {
    #[allow(unused)]
    /// Convenient constructor for testing
    pub fn with_files<I: AsRef<Path>>(src: Vec<I>) -> Self {
        Opt {
            src: src
                .into_iter()
                .filter_map(|x| std::fs::canonicalize(x).ok())
                .collect(),
            ..Default::default()
        }
    }

    /// Generate a string of hash based on the path passed in
    pub fn src_hash(&self) -> String {
        let mut hash = sha1::Sha1::new();
        let mut srcs = self.src.clone();

        srcs.sort();

        for file in srcs.into_iter() {
            hash.update(file.to_string_lossy().as_bytes());
        }

        base64::encode_config(&hash.digest().bytes()[..], base64::URL_SAFE_NO_PAD)
    }

    pub fn temp_dirname(&self) -> PathBuf {
        format!("cargo-play.{}", self.src_hash()).into()
    }

    fn with_toolchain(mut self, toolchain: Option<String>) -> Self {
        self.toolchain = toolchain;
        self
    }

    pub fn parse(args: Vec<String>) -> Result<Self, ()> {
        if args.len() < 2 {
            Self::clap().print_help().unwrap_or(());
            return Err(());
        }

        let with_cargo = args[1] == "play";
        let mut args = args.into_iter();

        if with_cargo {
            args.next();
        }

        let toolchain = args
            .clone()
            .find(|x| x.starts_with('+'))
            .map(|s| String::from_iter(s.chars().skip(1)));

        Ok(Opt::from_iter(args.filter(|x| !x.starts_with('+'))).with_toolchain(toolchain))
    }
}

/// Convert `std::ffi::OsStr` to an absolute `std::path::PathBuf`
fn osstr_to_abspath(v: &OsStr) -> Result<PathBuf, OsString> {
    if let Ok(r) = PathBuf::from(v).canonicalize() {
        Ok(r)
    } else {
        Err(v.into())
    }
}

/// structopt compataible function to check whether a file exists
fn file_exist(v: String) -> Result<(), String> {
    let p = PathBuf::from(v);
    if !p.is_file() {
        Err(format!("input file does not exist: {:?}", p))
    } else {
        Ok(())
    }
}
