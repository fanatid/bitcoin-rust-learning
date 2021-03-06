#[macro_use]
extern crate quick_error;

mod client;
mod server;

mod logger;
mod signals;

// Parse CLI arguments and run specified subcommand
fn main() {
    let cli_yaml = clap::load_yaml!("./cli.yaml");
    let args = clap::App::from_yaml(cli_yaml)
        .name(include_str!("./cli.yaml.name").trim())
        .version(include_str!("./cli.yaml.version").trim())
        .get_matches();

    let code = match args.subcommand() {
        ("client", Some(args)) => client::main(args),
        ("server", Some(args)) => server::main(args),
        _ => 1, // not possible, but we need to cover this arm
    };

    std::process::exit(code);
}
