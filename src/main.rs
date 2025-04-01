use std::{
    cmp::Ordering,
    net::{Ipv4Addr, Ipv6Addr},
    process::Command,
};

use addr::{domain, parse_domain_name};
use config::FileFormat;
use directories::ProjectDirs;
use itertools::Itertools;
use log::{debug, info, warn, LevelFilter};
use serde::Deserialize;

use hamsando::{
    record::{Content, Type},
    Client, Error,
};
use url::Url;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Ipv4Scope {
    Private,
    Public,
}

#[derive(Deserialize)]
struct DomainConfig {
    name: String,
    ipv4: Option<Ipv4Scope>,
    #[serde(default)]
    ipv6: bool,
}

#[derive(Deserialize)]
struct IpConfig {
    device: String,
    #[serde(default = "default_ip_oracle")]
    ip_oracle: Url,
}

fn default_ip_oracle() -> Url {
    "https://api.ipify.org/"
        .parse()
        .expect("Unable to parse the default IP oracle")
}

#[derive(Deserialize)]
struct Config {
    api: hamsando::Config,
    ip: IpConfig,
    domains: Vec<DomainConfig>,
}

enum IpVersion {
    Ipv4,
    Ipv6,
}

fn run_ip_command(device: &str, ip_version: &IpVersion) -> Result<String, Error> {
    let ip_version_arg = match ip_version {
        IpVersion::Ipv4 => "-4",
        IpVersion::Ipv6 => "-6",
    };
    let ip_output = Command::new("ip")
        .arg(ip_version_arg)
        .arg("-o")
        .arg("address")
        .arg("show")
        .arg("scope")
        .arg("global")
        .arg("dev")
        .arg(device)
        .output()?;

    let ip_output = String::from_utf8(ip_output.stdout)?;

    let first_line = ip_output
        .lines()
        .next()
        .ok_or_else(|| Error::Custom("Empty output from ip command".to_string()))?;

    let ip_with_subnet = first_line
        .split_whitespace()
        .nth(3)
        .ok_or_else(|| Error::Custom("Nothing found at expected position".to_string()))?;

    let ip = ip_with_subnet
        .split('/')
        .next()
        .ok_or_else(|| Error::Custom("Malformed IP with subnet".to_string()))?;
    Ok(ip.to_string())
}

fn get_ipv4_private(device: &str) -> Result<Ipv4Addr, Error> {
    let ip = run_ip_command(device, &IpVersion::Ipv4)?;
    Ok(ip.parse()?)
}

fn get_ipv4_public(ip_oracle: Url) -> Result<Ipv4Addr, Error> {
    Ok(reqwest::blocking::get(ip_oracle)?
        .error_for_status()?
        .text()?
        .trim()
        .parse()?)
}

fn get_ipv6(device: &str) -> Result<Ipv6Addr, Error> {
    let ip = run_ip_command(device, &IpVersion::Ipv6)?;
    Ok(ip.parse()?)
}

fn update_dns(client: &Client, domain: &domain::Name, content: &Content) -> Result<(), Error> {
    let dns = client.retrieve_dns_by_name_type(domain, &Type::from(content))?;
    match dns.len().cmp(&1) {
        Ordering::Less => client.create_dns(domain, content).map(|_| ()),
        Ordering::Equal => {
            if dns[0].content == *content {
                return Ok(());
            }
            client.edit_dns(domain, dns[0].id, content)
        }
        Ordering::Greater => Err(Error::Custom(format!(
            "Multiple DNS records for domain {domain}"
        ))),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(LevelFilter::max())
        .parse_default_env()
        .init();

    let project_dirs = ProjectDirs::from("", "", "hamsando")
        .ok_or_else(|| Error::Custom("Unable to find config directory".to_string()))?;
    let config_file = project_dirs.config_dir().join("config.toml");

    debug!(
        "Loading configuration from {} and environment",
        config_file.display()
    );

    let config = config::Config::builder()
        .add_source(config::File::new(
            config_file
                .to_str()
                .ok_or_else(|| Error::Custom("Config file path is not valid UTF-8".to_string()))?,
            FileFormat::Toml,
        ))
        .add_source(config::Environment::with_prefix("HAMSANDO"))
        .build()?;

    let config: Config = config.try_deserialize()?;

    let client = Client::new(config.api);
    client.test_auth()?;

    info!("Successfully authenticated");

    let ipv4_private = get_ipv4_private(&config.ip.device);
    let ipv4_public = get_ipv4_public(config.ip.ip_oracle);
    let ipv6 = get_ipv6(&config.ip.device);

    for domain in config.domains.iter().unique_by(|d| &d.name) {
        let name = match parse_domain_name(&domain.name) {
            Ok(name) => name,
            Err(e) => {
                warn!("Parsing domain name failed: {e}");
                continue;
            }
        };

        if let Some(scope) = &domain.ipv4 {
            let ipv4 = match scope {
                Ipv4Scope::Private => {
                    info!("Updating IPv4 to private IP for domain {name}");
                    &ipv4_private
                }
                Ipv4Scope::Public => {
                    info!("Updating IPv4 to public IP for domain {name}");
                    &ipv4_public
                }
            };
            match ipv4 {
                Ok(ipv4) => {
                    if let Err(e) = update_dns(&client, &name, &Content::A(*ipv4)) {
                        warn!("Updating A record for {name} failed: {e}");
                    };
                }
                Err(e) => {
                    warn!("Unable to get IPv4: {e}");
                }
            };
        }

        if domain.ipv6 {
            info!("Updating IPv6 for domain {name}");
            match &ipv6 {
                Ok(ipv6) => {
                    if let Err(e) = update_dns(&client, &name, &Content::Aaaa(*ipv6)) {
                        warn!("Updating AAAA record for {name} failed: {e}");
                    }
                }
                Err(e) => {
                    warn!("Unable to get IPv6: {e}");
                }
            };
        }
    }

    Ok(())
}
