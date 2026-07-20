# arp-whisper

`arp-whisper` is a small Rust ARP responder.

It listens for ARP requests on one interface and replies with the MAC address configured for the target IP. This is useful when you need deterministic ARP replies for a set of hosts on a controlled network.

## What it does

- Reads an INI configuration file
- Watches one network interface for ARP requests
- Matches the request target IP against a configured host list
- Sends an ARP reply with the configured MAC address
- Logs to stderr with selectable log levels

## Requirements

- Linux or macOS with a working Rust toolchain
- Root privileges or `CAP_NET_RAW`
- A network interface that can see ARP traffic

## Configuration

The binary supports these commands:

```bash
arp-whisper <CONFIG_PATH>
arp-whisper --validate-config <CONFIG_PATH>
arp-whisper --version
```

`--validate-config` checks that the configuration file can be read, has a non-empty
`[Network].interface`, includes a `[Hosts]` section, and uses a supported
`logging_level` when one is configured. It does not access network interfaces.

Example configuration:

```ini
logging_level = "debug"

[Network]
interface = eth0

[Hosts]
; ip = mac_address
192.168.1.2 = aa:bb:cc:dd:ee:ff
192.168.1.3 = 00:11:22:33:44:55
192.168.100.33 = 00:11:22:33:44:55
```

### Configuration keys

- `logging_level`
  - Optional
  - Valid values: `info`, `warn`, `debug`, `off`
  - Default: `info`
- `[Network].interface`
  - Required
  - Name of the interface to listen on, for example `eth0` or `en0`
- `[Hosts]`
  - Required
  - Each entry maps an IPv4 address to a MAC address

## Build

From source:

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

## Run

Build a release binary if you do not already have one:

```bash
cargo build --release
```

Then run the binary with root privileges:

```bash
sudo ./target/release/arp-whisper example-config/config.ini
```

### Example run

With this config:

```ini
[Network]
interface = eth0

[Hosts]
192.168.1.2 = aa:bb:cc:dd:ee:ff
```

arp-whisper will reply to an ARP request for `192.168.1.2` with `aa:bb:cc:dd:ee:ff`.

## Installation

Install the binary from the repository:

```bash
cargo install --path .
```

If you want a system-wide binary for the service file, install it to the path used by your deployment. The bundled `arp-whisper.service` currently expects the binary at `/bin/arp-whisper`.

## Systemd

The repository includes a systemd unit in `etc/arp-whisper.service`.

Example deployment:

```bash
sudo install -o root -g root -m 644 etc/arp-whisper.service /etc/systemd/system/
sudo install -o root -g root -m 755 target/release/arp-whisper /bin/arp-whisper
sudo install -o root -g root -m 644 example-config/config.ini /etc/arp-whisper.ini
sudo systemctl daemon-reload
sudo systemctl enable --now arp-whisper
```

The service enables:

- `CAP_NET_RAW`
- `RestrictAddressFamilies=AF_UNIX AF_NETLINK AF_PACKET`
- `NoNewPrivileges=yes`
- `ProtectSystem=strict`

If you change the binary path, update `ExecStart` in the unit file accordingly.

## AppArmor

The repository also includes an AppArmor profile in `etc/usr.bin.arp-whisper`.

The profile assumes the binary is installed at `/usr/bin/arp-whisper`.

Example:

```bash
sudo install -o root -g root -m 644 etc/usr.bin.arp-whisper /etc/apparmor.d/
sudo apparmor_parser -r /etc/apparmor.d/usr.bin.arp-whisper
```

## Development

Useful `make` targets:

```bash
make test
make coverage
make validate
make fmt
make clippy
```

## Testing

Run the unit tests:

```bash
make test
```

Generate an HTML coverage report:

```bash
make coverage
```

The report is written to `target/coverage/tarpaulin-report.html`.

## Troubleshooting

### `Interface not found`

The interface name in `[Network].interface` does not exist on the machine.

Check the available interfaces and update the config file.

### `invalid MAC address` or `invalid IP address`

One of the entries in `[Hosts]` is malformed.

Use standard IPv4 notation and a colon-separated MAC address:

```ini
192.168.1.2 = aa:bb:cc:dd:ee:ff
```

### No replies are sent

Check the following:

- The process has root privileges or `CAP_NET_RAW`
- The interface sees the ARP requests
- The target IP exists in `[Hosts]`
- The config file path passed to the binary is correct

## License

`arp-whisper` is licensed under GPL-3.0-only. See `LICENCE` for details.
