use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::Deserialize;
use strum_macros::IntoStaticStr;

#[derive(Debug, Deserialize, PartialEq, Eq, IntoStaticStr)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum Type {
    A,
    Mx,
    Cname,
    Alias,
    Txt,
    Ns,
    Aaaa,
    Srv,
    Tlsa,
    Caa,
    Https,
    Svcb,
}

impl Type {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub fn from(value: &Content) -> Self {
        match value {
            Content::A(_) => Type::A,
            Content::Mx(_) => Type::Mx,
            Content::Cname(_) => Type::Cname,
            Content::Alias(_) => Type::Alias,
            Content::Txt(_) => Type::Txt,
            Content::Ns(_) => Type::Ns,
            Content::Aaaa(_) => Type::Aaaa,
            Content::Srv(_) => Type::Srv,
            Content::Tlsa(_) => Type::Tlsa,
            Content::Caa(_) => Type::Caa,
            Content::Https(_) => Type::Https,
            Content::Svcb(_) => Type::Svcb,
        }
    }
}

#[derive(Debug, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Content {
    A(Ipv4Addr),
    Mx(String),
    Cname(String),
    Alias(String),
    Txt(String),
    Ns(String),
    Aaaa(Ipv6Addr),
    Srv(String),
    Tlsa(String),
    Caa(String),
    Https(String),
    Svcb(String),
}

impl Content {
    pub fn type_as_str(&self) -> &'static str {
        self.into()
    }

    pub fn value_to_string(&self) -> String {
        match self {
            Content::A(addr) => addr.to_string(),
            Content::Mx(value) => value.clone(),
            Content::Cname(value) => value.clone(),
            Content::Alias(value) => value.clone(),
            Content::Txt(value) => value.clone(),
            Content::Ns(value) => value.clone(),
            Content::Aaaa(addr) => addr.to_string(),
            Content::Srv(value) => value.clone(),
            Content::Tlsa(value) => value.clone(),
            Content::Caa(value) => value.clone(),
            Content::Https(value) => value.clone(),
            Content::Svcb(value) => value.clone(),
        }
    }

    pub fn from(type_: &Type, content: &str) -> Result<Content, std::net::AddrParseError> {
        Ok(match type_ {
            Type::A => Content::A(content.parse()?),
            Type::Mx => Content::Mx(content.to_string()),
            Type::Cname => Content::Cname(content.to_string()),
            Type::Alias => Content::Alias(content.to_string()),
            Type::Txt => Content::Txt(content.to_string()),
            Type::Ns => Content::Ns(content.to_string()),
            Type::Aaaa => Content::Aaaa(content.parse()?),
            Type::Srv => Content::Srv(content.to_string()),
            Type::Tlsa => Content::Tlsa(content.to_string()),
            Type::Caa => Content::Caa(content.to_string()),
            Type::Https => Content::Https(content.to_string()),
            Type::Svcb => Content::Svcb(content.to_string()),
        })
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

        #[derive(Deserialize)]
        struct ContentDeserializable<'a> {
            #[serde(rename = "type")]
            type_: Type,
            content: &'a str,
        }

        ContentDeserializable::deserialize(deserializer)
            .and_then(|c| Content::from(&c.type_, c.content).map_err(Error::custom))
    }
}

#[derive(Debug, Deserialize)]
pub struct Record {
    #[serde(deserialize_with = "deserialize_string_to_i64")]
    pub id: i64,
    pub name: String,
    #[serde(flatten)]
    pub content: Content,
}

fn deserialize_string_to_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
