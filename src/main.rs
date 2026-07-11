mod network;
use log::{LevelFilter, Log, Metadata, Record};
use std::env;
use std::io::{self, Write};
use std::process;
use std::sync::OnceLock;

use ini::Ini;

use crate::network::{new_host, respond_arp_queries};

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

    // Parse the configuration
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

    // Let's get all the Hosts
    let mut hosts = Vec::new();
    for (k, v) in conf.section(Some("Hosts")).unwrap().iter() {
        hosts.push(new_host(k, v));
    }
    log::debug!("Hosts defined in configuration: {}", hosts.len());

    respond_arp_queries(interface_name, hosts);
}
