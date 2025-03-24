pub mod record;

use serde::Deserialize;
use serde_json::json;

use record::{Content, Record, Type};

#[derive(Deserialize)]
pub struct Config {
    pub endpoint: String,
    pub apikey: String,
    pub secretapikey: String,
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    ParseInt(std::num::ParseIntError),
    Custom(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Error::Reqwest(e) => e.to_string(),
            Error::ParseInt(e) => e.to_string(),
            Error::Custom(s) => s.clone(),
        };
        write!(f, "{}", string)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Reqwest(e) => Some(e),
            Error::ParseInt(e) => Some(e),
            Error::Custom(_) => None,
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

        let payload = json!({
            "secretapikey": self.config.secretapikey.as_str(),
            "apikey": self.config.apikey.as_str(),
        });

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
            "content": content.value_to_string(),
        });

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
            "content": content.value_to_string(),
        });

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

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
            "content": content.value_to_string(),
        });

        self.client
            .post(url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        Ok(())
    }

    pub fn delete_dns(&self, domain: &str, id: i64) -> Result<(), Error> {
        let url = format!("{}/dns/delete/{}/{}", self.config.endpoint, domain, id);

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
        domain: &str,
        name: Option<&str>,
        type_: Type,
    ) -> Result<(), Error> {
        let url = format!(
            "{}/dns/deleteByNameType/{}/{}/{}",
            self.config.endpoint,
            domain,
            type_.as_str(),
            name.unwrap_or(""),
        );

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

    pub fn retrieve_dns(&self, domain: &str, id: Option<i64>) -> Result<Vec<Record>, Error> {
        let url = format!(
            "{}/dns/retrieve/{}/{}",
            self.config.endpoint,
            domain,
            id.map(|id| id.to_string()).unwrap_or("".to_string())
        );

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

        if id.is_some() && resp.records.len() != 1 {
            return Err(Error::Custom("Multiple records found".to_string()));
        }

        Ok(resp.records)
    }

    pub fn retrieve_dns_by_name_type(
        &self,
        domain: &str,
        name: Option<&str>,
        type_: Type,
    ) -> Result<Vec<Record>, Error> {
        let url = format!(
            "{}/dns/retrieveByNameType/{}/{}/{}",
            self.config.endpoint,
            domain,
            type_.as_str(),
            name.unwrap_or(""),
        );

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

        if resp.records.len() != 1 {
            return Err(Error::Custom("Multiple records found".to_string()));
        }

        Ok(resp.records)
    }
}
