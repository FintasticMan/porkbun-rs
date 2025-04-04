pub mod record;

use std::net::IpAddr;

use addr::domain;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error as ThisError;
use url::Url;

use record::{Content, Record, Type};

#[derive(ThisError, Debug)]
pub enum DomainError {
    #[error("domain {0:?} has a prefix")]
    HasPrefix(String),
    #[error("domain {0:?} doesn't have a root")]
    MissingRoot(String),
}

#[derive(ThisError, Debug)]
pub enum ApiError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
}

#[derive(ThisError, Debug)]
pub enum ClientBuilderError {
    #[error("missing field: {0}")]
    MissingField(String),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
}

pub struct ClientBuilder {
    endpoint: Option<Url>,
    apikey: Option<String>,
    secretapikey: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            apikey: None,
            secretapikey: None,
        }
    }

    pub fn endpoint(mut self, endpoint: &Url) -> Self {
        self.endpoint = Some(endpoint.clone());
        self
    }

    pub fn endpoint_if_some(mut self, endpoint: Option<&Url>) -> Self {
        if let Some(endpoint) = endpoint {
            self.endpoint = Some(endpoint.clone());
        }
        self
    }
    pub fn apikey(mut self, apikey: &str) -> Self {
        self.apikey = Some(apikey.to_string());
        self
    }

    pub fn secretapikey(mut self, secretapikey: &str) -> Self {
        self.secretapikey = Some(secretapikey.to_string());
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        let endpoint = match self.endpoint {
            Some(endpoint) => endpoint,
            None => "https://api.porkbun.com/api/json/v3/".parse()?,
        };
        let apikey = self
            .apikey
            .ok_or_else(|| ClientBuilderError::MissingField("apikey".to_string()))?;
        let secretapikey = self
            .secretapikey
            .ok_or_else(|| ClientBuilderError::MissingField("secretapikey".to_string()))?;

        Ok(Client {
            endpoint,
            apikey,
            secretapikey,
            client: reqwest::blocking::Client::new(),
        })
    }
}

pub struct Client {
    endpoint: Url,
    apikey: String,
    secretapikey: String,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn test_auth(&self) -> Result<IpAddr, ApiError> {
        let url = self.endpoint.join("ping")?;

        let payload = json!({
            "secretapikey": self.secretapikey.as_str(),
            "apikey": self.apikey.as_str(),
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

    pub fn create_dns(
        &self,
        domain: &domain::Name,
        content: &Content,
        ttl: Option<i64>,
        prio: Option<i64>,
    ) -> Result<i64, ApiError> {
        let (prefix, root) = split_domain(domain)?;
        let url = self.endpoint.join("dns/create/")?.join(root)?;

        let mut payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
            "type": content.type_as_str(),
            "content": content.value_to_string(),
        });
        if let Some(prefix) = prefix {
            payload["name"] = serde_json::Value::from(prefix);
        }
        if let Some(ttl) = ttl {
            payload["ttl"] = serde_json::Value::from(ttl);
        }
        if let Some(prio) = prio {
            payload["prio"] = serde_json::Value::from(prio);
        }

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct Response {
            #[serde(deserialize_with = "record::deserialize_to_i64")]
            id: i64,
        }

        Ok(resp.json::<Response>()?.id)
    }

    pub fn edit_dns(
        &self,
        domain: &domain::Name,
        id: i64,
        content: &Content,
        ttl: Option<i64>,
        prio: Option<i64>,
    ) -> Result<(), ApiError> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .endpoint
            .join("dns/edit/")?
            .join(&format!("{root}/"))?
            .join(&id.to_string())?;

        let mut payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
            "type": content.type_as_str(),
            "content": content.value_to_string(),
        });
        if let Some(prefix) = prefix {
            payload["name"] = serde_json::Value::from(prefix);
        }
        if let Some(ttl) = ttl {
            payload["ttl"] = serde_json::Value::from(ttl);
        }
        if let Some(prio) = prio {
            payload["prio"] = serde_json::Value::from(prio);
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
        ttl: Option<i64>,
        prio: Option<i64>,
    ) -> Result<(), ApiError> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .endpoint
            .join("dns/editByNameType/")?
            .join(&format!("{root}/"))?
            .join(&format!("{}/", content.type_as_str()))?
            .join(prefix.unwrap_or(""))?;

        let mut payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
            "content": content.value_to_string(),
        });
        if let Some(ttl) = ttl {
            payload["ttl"] = serde_json::Value::from(ttl);
        }
        if let Some(prio) = prio {
            payload["prio"] = serde_json::Value::from(prio);
        }

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        Ok(())
    }

    pub fn delete_dns(&self, domain: &domain::Name, id: i64) -> Result<(), ApiError> {
        let (prefix, root) = split_domain(domain)?;
        if prefix.is_some() {
            return Err(ApiError::Domain(DomainError::HasPrefix(domain.to_string())));
        }

        let url = self
            .endpoint
            .join("dns/delete/")?
            .join(&format!("{root}/"))?
            .join(&id.to_string())?;

        let payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
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
    ) -> Result<(), ApiError> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .endpoint
            .join("dns/deleteByNameType/")?
            .join(&format!("{root}/"))?
            .join(&format!("{}/", type_.as_str()))?
            .join(prefix.unwrap_or(""))?;

        let payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
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
    ) -> Result<Vec<Record>, ApiError> {
        let (prefix, root) = split_domain(domain)?;
        if prefix.is_some() {
            return Err(ApiError::Domain(DomainError::HasPrefix(domain.to_string())));
        }

        let url = self
            .endpoint
            .join("dns/retrieve/")?
            .join(&format!("{root}/"))?
            .join(&id.map_or_else(|| "".to_string(), |id| id.to_string()))?;

        let payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
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
    ) -> Result<Vec<Record>, ApiError> {
        let (prefix, root) = split_domain(domain)?;
        let url = self
            .endpoint
            .join("dns/retrieveByNameType/")?
            .join(&format!("{root}/"))?
            .join(&format!("{}/", type_.as_str()))?
            .join(prefix.unwrap_or(""))?;

        let payload = json!({
            "secretapikey": self.secretapikey,
            "apikey": self.apikey,
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

fn split_domain<'a>(name: &'a domain::Name) -> Result<(Option<&'a str>, &'a str), DomainError> {
    let root = name
        .root()
        .ok_or_else(|| DomainError::MissingRoot(name.to_string()))?;
    let prefix = name.prefix();

    Ok((prefix, root))
}
