use ini::Ini;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    MissingInterface,
    EmptyInterface,
    MissingHostsSection,
    InvalidLoggingLevel { value: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInterface => write!(f, "missing [Network].interface in configuration"),
            Self::EmptyInterface => write!(f, "[Network].interface must not be empty"),
            Self::MissingHostsSection => write!(f, "missing [Hosts] section in configuration"),
            Self::InvalidLoggingLevel { value } => write!(
                f,
                "invalid logging_level `{value}`; expected one of: info, warn, debug, off"
            ),
        }
    }
}

pub fn load_and_validate(config_path: &str) -> Result<Ini, String> {
    let config =
        Ini::load_from_file(config_path).map_err(|error| format_load_error(config_path, &error))?;
    validate(&config).map_err(|error| error.to_string())?;
    Ok(config)
}

fn format_load_error(config_path: &str, error: &ini::Error) -> String {
    match error {
        ini::Error::Io(error) => format!("could not read `{config_path}`: {error}"),
        ini::Error::Parse(error) => format_parse_error(config_path, error),
    }
}

fn format_parse_error(config_path: &str, error: &ini::ParseError) -> String {
    let detail = match error.msg.as_ref() {
        "missing key" => "expected a setting in the form `key = value`",
        detail => detail,
    };

    format!(
        "invalid configuration file `{config_path}` at line {}, column {}: {detail}",
        error.line, error.col
    )
}

pub fn validate(config: &Ini) -> Result<(), ConfigError> {
    interface_name(config)?;

    if config.section(Some("Hosts")).is_none() {
        return Err(ConfigError::MissingHostsSection);
    }

    if let Some(level) = config.general_section().get("logging_level") {
        match level {
            "info" | "warn" | "debug" | "off" => {}
            value => {
                return Err(ConfigError::InvalidLoggingLevel {
                    value: value.to_owned(),
                })
            }
        }
    }

    Ok(())
}

pub fn interface_name(config: &Ini) -> Result<&str, ConfigError> {
    let interface_name = config
        .get_from(Some("Network"), "interface")
        .ok_or(ConfigError::MissingInterface)?;

    if interface_name.trim().is_empty() {
        return Err(ConfigError::EmptyInterface);
    }

    Ok(interface_name)
}

pub fn logging_level(config: &Ini) -> &str {
    config
        .general_section()
        .get("logging_level")
        .unwrap_or("info")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(contents: &str) -> Ini {
        Ini::load_from_str(contents).expect("config should parse")
    }

    fn valid_config(logging_level: Option<&str>) -> Ini {
        let logging_level = logging_level
            .map(|level| format!("logging_level = {level}\n\n"))
            .unwrap_or_default();
        config(&format!(
            "{logging_level}[Network]\ninterface = eth0\n\n[Hosts]\n"
        ))
    }

    #[test]
    fn validate_should_allow_empty_hosts_section() {
        assert!(validate(&valid_config(None)).is_ok());
    }

    #[test]
    fn validate_should_accept_supported_logging_levels() {
        for level in ["info", "warn", "debug", "off"] {
            assert!(validate(&valid_config(Some(level))).is_ok());
        }
    }

    #[test]
    fn validate_should_fail_when_interface_is_missing() {
        assert_eq!(
            validate(&config("[Hosts]\n")),
            Err(ConfigError::MissingInterface)
        );
    }

    #[test]
    fn validate_should_fail_when_interface_is_empty() {
        assert_eq!(
            validate(&config("[Network]\ninterface =\n\n[Hosts]\n")),
            Err(ConfigError::EmptyInterface)
        );
    }

    #[test]
    fn validate_should_fail_when_hosts_section_is_missing() {
        assert_eq!(
            validate(&config("[Network]\ninterface = eth0\n")),
            Err(ConfigError::MissingHostsSection)
        );
    }

    #[test]
    fn validate_should_fail_when_logging_level_is_invalid() {
        assert_eq!(
            validate(&valid_config(Some("trace"))),
            Err(ConfigError::InvalidLoggingLevel {
                value: "trace".to_owned(),
            })
        );
    }

    #[test]
    fn format_parse_error_should_explain_the_configuration_error() {
        let error = ini::ParseError {
            line: 2,
            col: 3,
            msg: "missing key".into(),
        };

        assert_eq!(
            format_parse_error("config.ini", &error),
            "invalid configuration file `config.ini` at line 2, column 3: expected a setting in the form `key = value`"
        );
    }
}
