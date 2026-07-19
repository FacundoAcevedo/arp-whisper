mod config;
mod network;
use log::{LevelFilter, Log, Metadata, Record};
use std::env;
use std::io::{self, Write};
use std::process;
use std::sync::OnceLock;

use ini::Ini;

use crate::config::{interface_name, load_and_validate, logging_level};
use crate::network::{parse_host, respond_arp_queries, Host, NetworkError};

struct StderrLogger;

static LOGGER: OnceLock<StderrLogger> = OnceLock::new();

const USAGE: &str = "Usage:\n  arp-whisper <CONFIG_PATH>\n  arp-whisper --validate-config <CONFIG_PATH>\n  arp-whisper --version";

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run(String),
    ValidateConfig(String),
    Version,
}

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn init_logger(level: LevelFilter) {
    let logger = LOGGER.get_or_init(|| StderrLogger);
    log::set_logger(logger).expect("logger already initialized");
    log::set_max_level(level);
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = match parse_command(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("Error: {error}\n{USAGE}");
            process::exit(1);
        }
    };

    match command {
        Command::Version => println!("arp-whisper {}", env!("CARGO_PKG_VERSION")),
        Command::ValidateConfig(config_path) => {
            if let Err(error) = load_and_validate(&config_path) {
                eprintln!("Configuration validation failed: {error}");
                process::exit(1);
            }
            println!("Configuration is valid: {config_path}");
        }
        Command::Run(config_path) => run(&config_path),
    }
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Err("a configuration path is required".to_owned()),
        [flag] if flag == "--version" => Ok(Command::Version),
        [flag] if flag == "--validate-config" => {
            Err("--validate-config requires a configuration path".to_owned())
        }
        [flag] if flag.starts_with('-') => Err(format!("unknown option: {flag}")),
        [config_path] => Ok(Command::Run(config_path.clone())),
        [flag, config_path] if flag == "--validate-config" => {
            Ok(Command::ValidateConfig(config_path.clone()))
        }
        _ => Err("unexpected arguments".to_owned()),
    }
}

fn run(config_path: &str) {
    let conf = match load_and_validate(config_path) {
        Ok(conf) => conf,
        Err(error) => {
            eprintln!("Loading the configuration file failed: {error}");
            process::exit(1);
        }
    };

    let interface_name = match interface_name(&conf) {
        Ok(interface_name) => interface_name,
        Err(error) => {
            eprintln!("Configuration validation failed: {error}");
            process::exit(1);
        }
    };

    match logging_level(&conf) {
        "warn" => init_logger(LevelFilter::Warn),
        "debug" => init_logger(LevelFilter::Debug),
        "off" => init_logger(LevelFilter::Off),
        // "info" or option not specified
        _ => init_logger(LevelFilter::Info),
    }
    log::info!("Using configuration: {}", config_path);
    log::info!("Hearing to ARP requests using: {}", interface_name);

    let hosts = match load_hosts(&conf) {
        Ok(hosts) => hosts,
        Err(error) => {
            eprintln!("Error loading hosts from configuration: {}", error);
            process::exit(1);
        }
    };
    log::debug!("Hosts defined in configuration: {}", hosts.len());

    if let Err(error) = respond_arp_queries(interface_name, &hosts) {
        eprintln!("ARP responder failed: {}", error);
        process::exit(1);
    }
}

fn load_hosts(conf: &Ini) -> Result<Vec<Host>, NetworkError> {
    let hosts_section = conf
        .section(Some("Hosts"))
        .ok_or(NetworkError::MissingHostsSection)?;

    let mut hosts = Vec::with_capacity(hosts_section.len());
    for (ip_address, mac_address) in hosts_section {
        hosts.push(parse_host(ip_address, mac_address)?);
    }

    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(contents: &str) -> Ini {
        Ini::load_from_str(contents).expect("config should parse")
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parse_command_should_accept_version_flag() {
        assert_eq!(parse_command(&args(&["--version"])), Ok(Command::Version));
    }

    #[test]
    fn parse_command_should_accept_config_validation_flag() {
        assert_eq!(
            parse_command(&args(&["--validate-config", "config.ini"])),
            Ok(Command::ValidateConfig("config.ini".to_owned()))
        );
    }

    #[test]
    fn load_hosts_should_parse_hosts_from_config() {
        let conf = config(
            r#"
[Hosts]
192.168.1.2 = aa:bb:cc:dd:ee:ff
192.168.1.3 = 00:11:22:33:44:55
"#,
        );

        let hosts = load_hosts(&conf).expect("hosts should load");

        assert_eq!(hosts.len(), 2);
    }

    #[test]
    fn load_hosts_should_fail_when_hosts_section_is_missing() {
        let conf = config(
            r#"
[Network]
interface = eth0
"#,
        );

        let error = load_hosts(&conf).expect_err("hosts section should be required");

        assert_eq!(error, NetworkError::MissingHostsSection);
    }
}
