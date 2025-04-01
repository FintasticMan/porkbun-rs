use std::{
    cmp::Ordering,
    net::{Ipv4Addr, Ipv6Addr},
    process::{Command, Stdio},
};

use addr::{domain, parse_domain_name};
use config::FileFormat;
use directories::ProjectDirs;
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

fn run_ip_command(device: &str, ip_version: &str) -> Result<Vec<u8>, Error> {
    let ip_child = Command::new("ip")
        .args([
            ip_version, "-o", "address", "show", "scope", "global", "dev", device,
        ])
        .stdout(Stdio::piped())
        .spawn()?;
    let ip_out = ip_child
        .stdout
        .ok_or_else(|| Error::Custom("Failed to open ip stdout".to_string()))?;
    let head_child = Command::new("head")
        .args(["-n", "1"])
        .stdin(Stdio::from(ip_out))
        .stdout(Stdio::piped())
        .spawn()?;
    let head_out = head_child
        .stdout
        .ok_or_else(|| Error::Custom("Failed to open head stdout".to_string()))?;
    let awk_child = Command::new("awk")
        .arg("{printf \"%s\", $4}")
        .stdin(Stdio::from(head_out))
        .stdout(Stdio::piped())
        .spawn()?;
    let awk_out = awk_child
        .stdout
        .ok_or_else(|| Error::Custom("Failed to open awk stdout".to_string()))?;
    Ok(Command::new("sed")
        .args(["-E", "s/\\/.*?//"])
        .stdin(Stdio::from(awk_out))
        .output()?
        .stdout)
}

fn get_ipv4_private(device: &str) -> Result<Ipv4Addr, Error> {
    let ip = String::from_utf8(run_ip_command(device, "-4")?)?;
    Ok(ip.trim().parse()?)
}

fn get_ipv4_public(ip_oracle: Url) -> Result<Ipv4Addr, Error> {
    Ok(reqwest::blocking::get(ip_oracle)?
        .error_for_status()?
        .text()?
        .trim()
        .parse()?)
}

fn get_ipv6(device: &str) -> Result<Ipv6Addr, Error> {
    let ip = String::from_utf8(run_ip_command(device, "-6")?)?;
    Ok(ip.trim().parse()?)
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
    let project_dirs = ProjectDirs::from("", "", "hamsando")
        .ok_or_else(|| Error::Custom("Unable to find config directory".to_string()))?;
    let config_file = project_dirs.config_dir().join("config.toml");

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

    let ipv4_private = get_ipv4_private(&config.ip.device);
    let ipv4_public = get_ipv4_public(config.ip.ip_oracle);
    let ipv6 = get_ipv6(&config.ip.device);

    for domain in config.domains.iter() {
        let name = match parse_domain_name(&domain.name) {
            Ok(name) => name,
            Err(e) => {
                eprintln!("Parsing domain name failed: {e}");
                continue;
            }
        };

        if let Some(scope) = &domain.ipv4 {
            let ipv4 = match match scope {
                Ipv4Scope::Private => &ipv4_private,
                Ipv4Scope::Public => &ipv4_public,
            } {
                Ok(ipv4) => ipv4,
                Err(e) => {
                    eprintln!("Unable to get IPv4: {e}");
                    continue;
                }
            };
            if let Err(e) = update_dns(&client, &name, &Content::A(*ipv4)) {
                eprintln!("Updating DNS failed: {e}");
                continue;
            };
        }

        if domain.ipv6 {
            let ipv6 = match &ipv6 {
                Ok(ipv6) => ipv6,
                Err(e) => {
                    eprintln!("Unable to get IPv6 {e}");
                    continue;
                }
            };
            if let Err(e) = update_dns(&client, &name, &Content::Aaaa(*ipv6)) {
                eprintln!("Updating DNS failed: {e}");
                continue;
            };
        }
    }

    Ok(())
}
