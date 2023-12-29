# arp-whisper 📡

Welcome to the arp-whisper project! 🎉

## Description

arp-whisper is an open-source project written in Rust. It listens to ARP requests on a network interface and responds to them based on a list of IP-MAC address mappings defined in a configuration file. With arp-whisper, you can easily create an ARP responder to handle network queries and provide the appropriate MAC address for each IP address.

## How It Works

arp-whisper continuously monitors incoming ARP requests on the specified network interface. It checks if the request's target IP address matches any of the IP addresses in the configuration file. If a match is found, arp-whisper generates an ARP response with the corresponding MAC address and sends it back to the requester.

## How to Contribute

We welcome contributions from everyone! Whether you are a seasoned developer or new to open-source, there are various ways you can contribute to arp-whisper:

- 🐛 **Submit Bug Reports**: If you encounter any issues or bugs while using arp-whisper, please submit a detailed bug report on our [issue tracker](https://github.com/FacundoAcevedo/arp-whisper/issues). Include steps to reproduce the problem and any relevant information.
- 💡 **Suggest Enhancements**: Have an idea for a new feature or an improvement? Feel free to open an issue and share your suggestions. We appreciate your input!
- 💻 **Submit Pull Requests**: If you want to contribute code to arp-whisper, fork the repository, make your changes, and submit a pull request. We'll review your contribution and work together to merge it into the project.
- 📖 **Improve Documentation**: Documentation is crucial for any project. If you find areas where the documentation can be enhanced or if you'd like to add more examples or explanations, please don't hesitate to make a pull request with your updates.
- 👍 **Spread the Word**: If you find arp-whisper useful, please star the project on [GitHub](https://github.com/FacundoAcevedo/arp-whisper) and share it with others who might benefit from it.

## Installation

To install arp-whisper, ensure you have Rust and Cargo installed, then run the following command:

```shell
cargo install arp-whisper
```

### Systemd service

To install the service run:

```shell
sudo install -o root -g root -m 644 etc/arp-whisper.service /etc/systemd/system/
# Reload the daemon
sudo systemctl daemon-reload
# Start the service
sudo systemctl start arp-whisper
```

### AppArmor profile

`arp-whisper` has to be installed in `/usr/bin/arp-whisper`

```shell
sudo install -o root -g root -m 644 /path/to/your/usr.bin.arp-whisper /etc/apparmor.d/
# Load the profile
sudo apparmor_parser -r /etc/apparmor.d/usr.bin.arp-whisper
```

## Configuration Example

You can configure arp-whisper using a configuration file. Here's an example of the configuration file format:

```ini
; Optional field
; Default value: info
; Possible values: info, warn, debug, off
logging_level = "debug"
[Network]
interface = eth0

[Hosts]
; ip = mac_address
192.168.1.2 = aa:bb:cc:dd:ee:ff
192.168.1.3 = 00:11:22:33:44:55
```

In this example, the [Network] section specifies the network interface to listen on, and the [Hosts] section defines the IP-MAC address mappings.
How to Run

To run arp-whisper with your configuration file, use the following command:

```shell
sudo arp-whisper config.ini
```

Replace config.ini with the path to your actual configuration file.

## License

arp-whisper is licensed under the GNU General Public License v3.0 (GPLv3). See the LICENSE file for more details.
