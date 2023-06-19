mod network;
use simple_logger::{self, SimpleLogger};
use std::env;

use std::process;

use ini::Ini;

use crate::network::{new_host, respond_arp_queries};

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
        "warn" => SimpleLogger::new()
            .with_level(log::LevelFilter::Warn)
            .init()
            .unwrap(),
        "debug" => SimpleLogger::new()
            .with_level(log::LevelFilter::Debug)
            .init()
            .unwrap(),
        "off" => SimpleLogger::new()
            .with_level(log::LevelFilter::Off)
            .init()
            .unwrap(),
        // "info" or option not specified
        _ => SimpleLogger::new()
            .with_level(log::LevelFilter::Info)
            .init()
            .unwrap(),
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
