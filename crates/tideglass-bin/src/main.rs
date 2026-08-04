// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! tideGlass `UniBin` — sovereign drug repurposing service for biomeOS deployment.

mod capabilities;
mod cas_client;
mod data;
mod dispatch;
mod health;
mod server;

use std::process::ExitCode;

const DEFAULT_SOCKET: &str = "/run/tideglass/tideglass.sock";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    match parse_command(&args) {
        Command::Help => {
            print_usage();
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!(
                "{} {}",
                tideglass_core::PRIMAL_NAME,
                tideglass_core::VERSION
            );
            ExitCode::SUCCESS
        }
        Command::Capabilities => match print_capabilities() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("tideglass: {error}");
                ExitCode::FAILURE
            }
        },
        Command::Run { socket_path } => run_server(&socket_path),
    }
}

enum Command {
    Help,
    Version,
    Capabilities,
    Run { socket_path: String },
}

fn parse_command(args: &[String]) -> Command {
    if args.len() <= 1 || matches!(args[1].as_str(), "--help" | "-h" | "help") {
        return Command::Help;
    }

    match args[1].as_str() {
        "version" | "--version" | "-V" => Command::Version,
        "capabilities" => Command::Capabilities,
        "run" => Command::Run {
            socket_path: parse_socket_path(args),
        },
        _ => Command::Help,
    }
}

fn parse_socket_path(args: &[String]) -> String {
    if let Ok(path) = std::env::var("TIDEGLASS_SOCKET") {
        return path;
    }
    parse_socket_from_args(args)
}

fn parse_socket_from_args(args: &[String]) -> String {
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--socket" => {
                if let Some(path) = args.get(index + 1) {
                    return path.clone();
                }
                eprintln!("tideglass: --socket requires a path argument");
                std::process::exit(2);
            }
            "--" => break,
            other if other.starts_with("--socket=") => {
                return other.trim_start_matches("--socket=").to_owned();
            }
            _ => {}
        }
        index += 1;
    }

    DEFAULT_SOCKET.to_owned()
}

fn print_usage() {
    eprintln!(
        "\
tideGlass UniBin — sovereign drug repurposing service

Usage:
  tideglass run [--socket <path>]   Start UDS JSON-RPC server
  tideglass version                 Print version and exit
  tideglass capabilities            Print capabilities JSON and exit
  tideglass help                    Show this help message

Environment:
  TIDEGLASS_SOCKET    Default Unix socket path (overridden by --socket)

Default socket: {DEFAULT_SOCKET}"
    );
}

fn print_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(&capabilities::list())?;
    println!("{json}");
    Ok(())
}

fn run_server(socket_path: &str) -> ExitCode {
    eprintln!(
        "tideglass: starting {} v{} on {}",
        tideglass_core::PRIMAL_NAME,
        tideglass_core::VERSION,
        socket_path
    );

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("tideglass: failed to start tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };

    let module_data = runtime.block_on(async {
        if let Some(info) = tideglass_core::cas::discover_cas_socket() {
            let route_label = match info.routing {
                tideglass_core::cas::CasRouting::NeuralApi => "Neural API",
                tideglass_core::cas::CasRouting::Direct => "direct",
            };
            eprintln!(
                "tideglass: CAS discovered at {} ({})",
                info.path, route_label
            );
            let client = cas_client::CasClient::new(&info.path, info.routing);
            std::sync::Arc::new(data::load_from_cas(&client).await)
        } else {
            eprintln!("tideglass: no CAS socket found — running without CAS data");
            std::sync::Arc::new(data::ModuleData::default())
        }
    });

    if let Err(error) = runtime.block_on(server::run_server(socket_path, module_data)) {
        eprintln!("tideglass: server error: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_no_args_yields_help() {
        assert!(matches!(
            parse_command(&args(&["tideglass"])),
            Command::Help
        ));
    }

    #[test]
    fn parse_help_flag() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "--help"])),
            Command::Help
        ));
    }

    #[test]
    fn parse_short_help_flag() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "-h"])),
            Command::Help
        ));
    }

    #[test]
    fn parse_help_subcommand() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "help"])),
            Command::Help
        ));
    }

    #[test]
    fn parse_version_subcommand() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "version"])),
            Command::Version
        ));
    }

    #[test]
    fn parse_version_long_flag() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "--version"])),
            Command::Version
        ));
    }

    #[test]
    fn parse_version_short_flag() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "-V"])),
            Command::Version
        ));
    }

    #[test]
    fn parse_capabilities_subcommand() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "capabilities"])),
            Command::Capabilities
        ));
    }

    #[test]
    fn parse_unknown_subcommand_yields_help() {
        assert!(matches!(
            parse_command(&args(&["tideglass", "gibberish"])),
            Command::Help
        ));
    }

    #[test]
    fn socket_from_args_default() {
        let path = parse_socket_from_args(&args(&["tideglass", "run"]));
        assert_eq!(path, DEFAULT_SOCKET);
    }

    #[test]
    fn socket_from_args_flag() {
        let path =
            parse_socket_from_args(&args(&["tideglass", "run", "--socket", "/tmp/test.sock"]));
        assert_eq!(path, "/tmp/test.sock");
    }

    #[test]
    fn socket_from_args_equals_syntax() {
        let path = parse_socket_from_args(&args(&["tideglass", "run", "--socket=/tmp/eq.sock"]));
        assert_eq!(path, "/tmp/eq.sock");
    }

    #[test]
    fn socket_from_args_double_dash_stops_parsing() {
        let path = parse_socket_from_args(&args(&["tideglass", "run", "--", "--socket", "/x"]));
        assert_eq!(path, DEFAULT_SOCKET);
    }

    #[test]
    fn socket_from_args_ignores_unknown_flags() {
        let path = parse_socket_from_args(&args(&["tideglass", "run", "--verbose"]));
        assert_eq!(path, DEFAULT_SOCKET);
    }

    #[test]
    fn parse_run_yields_run_command() {
        let cmd = parse_command(&args(&["tideglass", "run"]));
        assert!(matches!(cmd, Command::Run { .. }));
    }

    #[test]
    fn print_usage_does_not_panic() {
        print_usage();
    }
}
