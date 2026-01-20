//! CLI argument parsing for RLM server.

use clap::{Arg, Command};
use std::path::PathBuf;

/// Command line interface for RLM server.
#[derive(Debug, Clone)]
pub struct Cli {
    /// Configuration file path.
    pub config: Option<PathBuf>,
    /// Server host override.
    pub host: Option<String>,
    /// Server port override.
    pub port: Option<u16>,
    /// Log level override.
    pub log_level: Option<String>,
    /// Log format override.
    pub log_format: Option<String>,
    /// Show version information.
    pub version: bool,
    /// Generate example configuration file.
    pub generate_config: Option<PathBuf>,
    /// Validate configuration and exit.
    pub validate: bool,
}

impl Cli {
    /// Parse command line arguments.
    pub fn parse() -> Self {
        let matches = Command::new("rlm-server")
            .about("RLM (Recursive Language Model) OpenAI-compatible server")
            .version(env!("CARGO_PKG_VERSION"))
            .author("RLM Project")
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .value_name("FILE")
                    .help("Configuration file path")
                    .value_parser(clap::value_parser!(PathBuf)),
            )
            .arg(
                Arg::new("host")
                    .short('h')
                    .long("host")
                    .value_name("HOST")
                    .help("Server host address")
                    .env("RLM_SERVER_HOST"),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .value_name("PORT")
                    .help("Server port number")
                    .value_parser(clap::value_parser!(u16))
                    .env("RLM_SERVER_PORT"),
            )
            .arg(
                Arg::new("log-level")
                    .short('l')
                    .long("log-level")
                    .value_name("LEVEL")
                    .help("Log level (trace, debug, info, warn, error)")
                    .value_parser(["trace", "debug", "info", "warn", "error"])
                    .env("RLM_LOG_LEVEL"),
            )
            .arg(
                Arg::new("log-format")
                    .long("log-format")
                    .value_name("FORMAT")
                    .help("Log format (json, pretty)")
                    .value_parser(["json", "pretty"])
                    .env("RLM_LOG_FORMAT"),
            )
            .arg(
                Arg::new("version")
                    .short('V')
                    .long("version")
                    .help("Show version information")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("generate-config")
                    .long("generate-config")
                    .value_name("FILE")
                    .help("Generate example configuration file and exit")
                    .value_parser(clap::value_parser!(PathBuf)),
            )
            .arg(
                Arg::new("validate")
                    .long("validate")
                    .help("Validate configuration and exit")
                    .action(clap::ArgAction::SetTrue),
            )
            .get_matches();

        Self {
            config: matches.get_one::<PathBuf>("config").cloned(),
            host: matches.get_one::<String>("host").cloned(),
            port: matches.get_one::<u16>("port").copied(),
            log_level: matches.get_one::<String>("log-level").cloned(),
            log_format: matches.get_one::<String>("log-format").cloned(),
            version: matches.get_flag("version"),
            generate_config: matches.get_one::<PathBuf>("generate-config").cloned(),
            validate: matches.get_flag("validate"),
        }
    }

    /// Check if any action flags are set (non-server operations).
    pub fn has_action(&self) -> bool {
        self.version || self.generate_config.is_some() || self.validate
    }
}

/// Print version information.
pub fn print_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!();
    println!("RLM (Recursive Language Model) OpenAI-compatible server");
    println!("Enables processing of arbitrarily long contexts through recursive decomposition");
    println!();
    println!("Paper: \"RLM: A Recursive Language Model for Long Contexts\" (arXiv:2512.24601)");
    println!("Repository: https://github.com/example/rlm");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI can be constructed (basic smoke test)
        // In real tests, you'd use clap's testing utilities
        let cli = Cli {
            config: None,
            host: Some("127.0.0.1".to_string()),
            port: Some(3000),
            log_level: Some("debug".to_string()),
            log_format: Some("json".to_string()),
            version: false,
            generate_config: None,
            validate: false,
        };

        assert_eq!(cli.host.as_deref(), Some("127.0.0.1"));
        assert_eq!(cli.port, Some(3000));
        assert!(!cli.has_action());
    }

    #[test]
    fn test_action_detection() {
        let cli_with_version = Cli {
            config: None,
            host: None,
            port: None,
            log_level: None,
            log_format: None,
            version: true,
            generate_config: None,
            validate: false,
        };

        assert!(cli_with_version.has_action());

        let cli_with_generate = Cli {
            config: None,
            host: None,
            port: None,
            log_level: None,
            log_format: None,
            version: false,
            generate_config: Some(PathBuf::from("config.yaml")),
            validate: false,
        };

        assert!(cli_with_generate.has_action());
    }
}