mod network;
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
    println!("Using configuration: {}", config_path);

    // Parse the configuration
    let conf = Ini::load_from_file(config_path).unwrap();
    let interface_name = conf.get_from(Some("Network"), "interface").unwrap();
    println!("Interface name: {}", interface_name);

    // Let's get all the Hosts
    let mut hosts = Vec::new();
    for (k, v) in conf.section(Some("Hosts")).unwrap().iter() {
        hosts.push(new_host(k, v));
    }
    println!("Hosts defined: {}", hosts.len());

    respond_arp_queries(interface_name, hosts);
}
