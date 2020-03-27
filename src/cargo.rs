use std::collections::HashSet;

use serde::Serialize;
use toml::value::{Table, Value};

use crate::errors::CargoPlayError;
use crate::opt::{Dependency, RustEdition};

#[derive(Clone, Debug, Serialize)]
struct CargoPackage {
    name: String,
    version: String,
    edition: String,
}

impl CargoPackage {
    fn new(name: String, edition: RustEdition) -> Self {
        Self {
            name: name.to_lowercase(),
            version: "0.1.0".into(),
            edition: edition.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CargoManifest {
    package: CargoPackage,
    #[serde(serialize_with = "toml::ser::tables_last")]
    dependencies: Table,
    #[serde(serialize_with = "toml::ser::tables_last")]
    dev_dependencies: Table,
}

fn deserialize_deps<F>(deps: &Vec<Dependency>, filter: F) -> Result<Table, CargoPlayError>
where
    F: Fn(&Dependency) -> Option<String>,
{
    let dependencies = deps
        .into_iter()
        .filter_map(filter)
        .map(|dep| dep.parse::<toml::Value>())
        .collect::<Result<Vec<toml::Value>, _>>()
        .map_err(CargoPlayError::from_serde)?;

    if dependencies.iter().any(|d| !d.is_table()) {
        return Err(CargoPlayError::ParseError("format error!".into()));
    }

    Ok(dependencies
        .into_iter()
        .map(|d| d.try_into::<Table>().unwrap().into_iter())
        .flatten()
        .collect())
}

impl CargoManifest {
    pub(crate) fn new(
        name: String,
        dependencies: Vec<Dependency>,
        edition: RustEdition,
    ) -> Result<Self, CargoPlayError> {
        let (dependencies, dev_dependencies): (Table, Table) = (
            deserialize_deps(&dependencies, |d| match d {
                Dependency::Build(dep) => Some(dep.clone()),
                _ => None,
            })?,
            deserialize_deps(&dependencies, |d| match d {
                Dependency::Test(dep) => Some(dep.clone()),
                _ => None,
            })?,
        );

        Ok(Self {
            package: CargoPackage::new(name, edition),
            dependencies,
            dev_dependencies,
        })
    }

    fn normalize_crate_name(name: &str) -> String {
        name.replace("-", "_")
    }

    fn normalized_dependencies(&self) -> HashSet<String> {
        self.dependencies
            .clone()
            .into_iter()
            .map(|(key, _)| Self::normalize_crate_name(&key))
            .collect()
    }

    pub(crate) fn add_infers(&mut self, infers: HashSet<String>) {
        let existing = self.normalized_dependencies();

        // we don't need to normalize crate name here (in filter) since it's impossible to have
        // dash in use statments.
        self.dependencies.extend(
            infers
                .into_iter()
                .filter(|key| !existing.contains(key))
                .map(|key| (key, Value::String("*".into()))),
        );
    }
}
