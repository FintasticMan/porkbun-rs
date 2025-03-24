use std::collections::HashMap;
use std::net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr};

use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct Config {
    pub endpoint: String,
    pub apikey: String,
    pub secretapikey: String,
}

#[derive(Deserialize)]
struct ContentDeserializable {
    #[serde(rename = "type")]
    type_: String,
    content: String,
}

#[derive(Debug)]
pub enum Content {
    A(Ipv4Addr),
    Aaaa(Ipv6Addr),
}

impl Content {
    pub fn type_as_str(&self) -> &'static str {
        match self {
            Content::A(_) => "A",
            Content::Aaaa(_) => "AAAA",
        }
    }

    pub fn addr_to_string(&self) -> String {
        match self {
            Content::A(addr) => addr.to_string(),
            Content::Aaaa(addr) => addr.to_string(),
        }
    }
}

impl From<IpAddr> for Content {
    fn from(value: IpAddr) -> Self {
        match value {
            IpAddr::V4(addr) => Content::A(addr),
            IpAddr::V6(addr) => Content::Aaaa(addr),
        }
    }
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        ContentDeserializable::deserialize(deserializer).and_then(|c| match c.type_.as_str() {
            "A" => {
                Ok(Content::A(c.content.parse().map_err(
                    |e: AddrParseError| D::Error::custom(e.to_string()),
                )?))
            }
            "AAAA" => {
                Ok(Content::Aaaa(c.content.parse().map_err(
                    |e: AddrParseError| D::Error::custom(e.to_string()),
                )?))
            }
            _ => Err(D::Error::custom(format!(
                "Invalid content type: {}",
                c.type_
            ))),
        })
    }
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    ParseInt(std::num::ParseIntError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Error::Reqwest(e) => e.to_string(),
            Error::ParseInt(e) => e.to_string(),
        };
        write!(f, "{}", string)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Reqwest(e) => Some(e),
            Error::ParseInt(e) => Some(e),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Reqwest(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Error::ParseInt(value)
    }
}

#[derive(Deserialize)]
pub struct Record {
    pub id: i64,
    pub name: String,
    #[serde(flatten)]
    pub content: Content,
    pub ttl: i64,
    pub prio: i64,
    pub notes: String,
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

    pub fn test_auth(&self) -> Result<String, Error> {
        let url = format!("{}/ping", self.config.endpoint);

        let payload = HashMap::from([
            ("secretapikey", self.config.secretapikey.as_str()),
            ("apikey", self.config.apikey.as_str()),
        ]);

        let resp = self.client.post(url).json(&payload).send()?;

        resp.error_for_status_ref()?;

        Ok(resp.text()?)
    }

    pub fn create_dns(
        &self,
        domain: &str,
        name: Option<&str>,
        content: Content,
    ) -> Result<i64, Error> {
        let url = format!("{}/dns/create/{}", self.config.endpoint, domain);

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
            "name": name.unwrap_or(""),
            "type": content.type_as_str(),
            "content": content.addr_to_string(),
        });

        let resp = self.client.post(url).json(&payload).send()?;

        resp.error_for_status_ref()?;

        #[derive(Deserialize)]
        struct Response {
            id: i64,
        }

        Ok(resp.json::<Response>()?.id)
    }

    pub fn edit_dns(
        &self,
        domain: &str,
        id: i64,
        name: Option<&str>,
        content: Content,
    ) -> Result<(), Error> {
        let url = format!("{}/dns/edit/{}/{}", self.config.endpoint, domain, id);

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
            "name": name.unwrap_or(""),
            "type": content.type_as_str(),
            "content": content.addr_to_string(),
        });

        let resp = self.client.post(url).json(&payload).send()?;

        resp.error_for_status()?;

        Ok(())
    }

    pub fn edit_dns_by_name_type(
        &self,
        domain: &str,
        name: Option<&str>,
        content: Content,
    ) -> Result<(), Error> {
        let url = format!(
            "{}/dns/editByNameType/{}/{}/{}",
            self.config.endpoint,
            domain,
            content.type_as_str(),
            name.unwrap_or("")
        );

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
            "content": content.addr_to_string(),
        });

        let resp = self.client.post(url).json(&payload).send()?;

        resp.error_for_status()?;

        Ok(())
    }

    pub fn delete_dns(&self, domain: &str, id: i64) -> Result<(), Error> {
        let url = format!("{}/dns/delete/{}/{}", self.config.endpoint, domain, id);

        let payload = json!({
            "secretapikey": self.config.secretapikey,
            "apikey": self.config.apikey,
        });

        let resp = self.client.post(url).json(&payload).send()?;

        resp.error_for_status()?;

        Ok(())
    }
}
