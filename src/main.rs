mod network;
use log::{LevelFilter, Log, Metadata, Record};
use std::env;
use std::io::{self, Write};
use std::process;
use std::sync::OnceLock;

use ini::Ini;

use crate::network::{parse_host, respond_arp_queries, Host, NetworkError};

struct StderrLogger;

static LOGGER: OnceLock<StderrLogger> = OnceLock::new();

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
    let mut args = env::args().skip(1);

    // Get config file path from the arguments
    let config_path = match args.next() {
        Some(n) => n,
        None => {
            eprintln!("Usage: arp-whisper <CONFIG_PATH>");
            process::exit(1);
        }
    };

    let conf = match Ini::load_from_file(config_path.clone()) {
        Ok(conf) => conf,
        Err(error) => {
            eprintln!("Loading the configuration file failed with: {}", error);
            process::exit(1);
        }
    };

    let interface_name = match conf.get_from(Some("Network"), "interface") {
        Some(interface_name) => interface_name,
        None => {
            eprintln!("Error: No interface defined.");
            process::exit(1);
        }
    };

    let log_level = conf
        .general_section()
        .get("logging_level")
        .unwrap_or("info");
    match log_level {
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
