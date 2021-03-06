use std::fs;

fn main() {
    generate_cli_name_version();
}

fn generate_cli_name_version() {
    let cargo_toml = include_str!("./Cargo.toml");
    let cargo: toml::Value = toml::from_str(cargo_toml).expect("Failed to parse Cargo.toml");
    let package = cargo
        .get("package")
        .expect("Cargo.toml should have `package` section");

    let name = package
        .get("name")
        .and_then(|x| x.as_str())
        .expect("Field get String field `name` from `package` section");
    fs::write("./src/cli.yaml.name", name).expect("Failed to create CLI file with name");

    let version = package
        .get("version")
        .and_then(|x| x.as_str())
        .expect("Field get String field `version` from `package` section");
    fs::write("./src/cli.yaml.version", version).expect("Failed to create CLI file with version");
}
