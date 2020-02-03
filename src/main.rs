mod client;
mod server;

// Parse CLI arguments and run specified subcommand
fn main() {
    let cli_yaml = clap::load_yaml!("./cli.yaml");
    let args = clap::App::from_yaml(cli_yaml)
        .name(include_str!("./cli.yaml.name").trim())
        .version(include_str!("./cli.yaml.version").trim())
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
