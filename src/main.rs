mod client;
mod server;

// Parse CLI arguments and run specified subcommand
fn main() {
    // TODO: how evaluate `name` and `version` on building?
    let cargo_toml = include_str!("../Cargo.toml");
    let cargo_parsed: toml::Value = toml::from_str(cargo_toml).unwrap();
    let package = cargo_parsed.get("package").unwrap();
    let name = package.get("name").unwrap().as_str().unwrap();
    let version = package.get("version").unwrap().as_str().unwrap();

    // TODO: how move to function without `temporary value created here`?
    // let yaml = &clap::YamlLoader::load_from_str(include_str!("./cli.yaml")).expect("Failed parse CLI configuration from YAML")[0];
    let yaml = clap::load_yaml!("./cli.yaml");
    let args = clap::App::from_yaml(yaml)
        .name(name)
        .version(version)
        .get_matches();

    match args.subcommand() {
        ("client", Some(sub_m)) => {
            client::main(sub_m);
        }
        ("server", Some(sub_m)) => {
            server::main(sub_m);
        }
        _ => {}
    }
}
