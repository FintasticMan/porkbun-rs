pub mod record;

use std::{
    io,
    net::{AddrParseError, IpAddr},
    string::FromUtf8Error,
};

use addr::domain;
use serde::Deserialize;
use serde_json::json;
use url::Url;

use record::{Content, Record, Type};

#[derive(Debug)]
pub enum DomainError {
    Invalid(String),
    HasPrefix(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::Invalid(d) => write!(f, "Invalid domain: {d}"),
            DomainError::HasPrefix(d) => write!(f, "Domain has prefix: {d}"),
        }
    }
}

impl std::error::Error for DomainError {}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    FromUtf8(FromUtf8Error),
    Reqwest(reqwest::Error),
    Url(url::ParseError),
    Domain(DomainError),
    AddrParse(AddrParseError),
    Custom(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::FromUtf8(e) => write!(f, "{e}"),
            Error::Reqwest(e) => write!(f, "{e}"),
            Error::Url(e) => write!(f, "{e}"),
            Error::Domain(e) => write!(f, "{e}"),
            Error::AddrParse(e) => write!(f, "{e}"),
            Error::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::FromUtf8(e) => Some(e),
            Error::Reqwest(e) => Some(e),
            Error::Url(e) => Some(e),
            Error::Domain(e) => Some(e),
            Error::AddrParse(e) => Some(e),
            Error::Custom(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Error::FromUtf8(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Reqwest(value)
    }
}

impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        Error::Url(value)
    }
}

impl From<AddrParseError> for Error {
    fn from(value: AddrParseError) -> Self {
        Error::AddrParse(value)
    }
}

pub(crate) fn split_domain<'a>(
    name: &'a domain::Name,
) -> Result<(Option<&'a str>, &'a str), Error> {
    let root = name
        .root()
        .ok_or_else(|| Error::Domain(DomainError::Invalid(name.to_string())))?;
    let prefix = name.prefix();

    Ok((prefix, root))
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_endpoint")]
    pub endpoint: Url,
    pub apikey: String,
    pub secretapikey: String,
}

fn default_endpoint() -> Url {
    "https://api.porkbun.com/api/json/v3/"
        .parse()
        .expect("Unable to parse the default endpoint")
}

pub struct Client {
    config: Config,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn test_auth(&self) -> Result<IpAddr, Error> {
        let url = self.config.endpoint.join("ping")?;

        let payload = json!({
            "secretapikey": self.config.secretapikey.as_str(),
            "apikey": self.config.apikey.as_str(),
        });

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            your_ip: IpAddr,
        }

        Ok(resp.json::<Response>()?.your_ip)
    }

    pub fn create_dns(&self, domain: &domain::Name, content: &Content) -> Result<i64, Error> {
        let (prefix, root) = split_domain(domain)?;
        let url = self.config.endpoint.join("dns/create/")?.join(root)?;

        let mut payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
            "type": content.type_as_str(),
            "content": content.value_to_string(),
        });
        if let Some(prefix) = prefix {
            payload["name"] = serde_json::Value::from(prefix);
        }

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct Response {
            id: i64,
        }

        Ok(resp.json::<Response>()?.id)
    }

    pub fn edit_dns(&self, domain: &domain::Name, id: i64, content: &Content) -> Result<(), Error> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .config
            .endpoint
            .join("dns/edit/")?
            .join(&format!("{root}/"))?
            .join(&id.to_string())?;

        let mut payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
            "type": content.type_as_str(),
            "content": content.value_to_string(),
        });
        if let Some(prefix) = prefix {
            payload["name"] = serde_json::Value::from(prefix);
        }

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        Ok(())
    }

    pub fn edit_dns_by_name_type(
        &self,
        domain: &domain::Name,
        content: &Content,
    ) -> Result<(), Error> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .config
            .endpoint
            .join("dns/editByNameType/")?
            .join(&format!("{root}/"))?
            .join(&format!("{}/", content.type_as_str()))?
            .join(prefix.unwrap_or(""))?;

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
            "content": content.value_to_string(),
        });

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        Ok(())
    }

    pub fn delete_dns(&self, domain: &domain::Name, id: i64) -> Result<(), Error> {
        let (prefix, root) = split_domain(domain)?;
        if prefix.is_some() {
            return Err(Error::Domain(DomainError::HasPrefix(domain.to_string())));
        }

        let url = self
            .config
            .endpoint
            .join("dns/delete/")?
            .join(&format!("{root}/"))?
            .join(&id.to_string())?;

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
        });

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        Ok(())
    }

    pub fn delete_dns_by_name_type(
        &self,
        domain: &domain::Name,
        type_: &Type,
    ) -> Result<(), Error> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .config
            .endpoint
            .join("dns/deleteByNameType/")?
            .join(&format!("{root}/"))?
            .join(&format!("{}/", type_.as_str()))?
            .join(prefix.unwrap_or(""))?;

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
        });

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        Ok(())
    }

    pub fn retrieve_dns(
        &self,
        domain: &domain::Name,
        id: Option<i64>,
    ) -> Result<Vec<Record>, Error> {
        let (prefix, root) = split_domain(domain)?;
        if prefix.is_some() {
            return Err(Error::Domain(DomainError::HasPrefix(domain.to_string())));
        }

        let url = self
            .config
            .endpoint
            .join("dns/retrieve/")?
            .join(&format!("{root}/"))?
            .join(&id.map_or_else(|| "".to_string(), |id| id.to_string()))?;

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
        });

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct Response {
            records: Vec<Record>,
        }

        let resp = resp.json::<Response>()?;

        Ok(resp.records)
    }

    pub fn retrieve_dns_by_name_type(
        &self,
        domain: &domain::Name,
        type_: &Type,
    ) -> Result<Vec<Record>, Error> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .config
            .endpoint
            .join("dns/retrieveByNameType/")?
            .join(&format!("{root}/"))?
            .join(&format!("{}/", type_.as_str()))?
            .join(prefix.unwrap_or(""))?;

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
        });

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct Response {
            records: Vec<Record>,
        }

        let resp = resp.json::<Response>()?;

        Ok(resp.records)
    }
}
