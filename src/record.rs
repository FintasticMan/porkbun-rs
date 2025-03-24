use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use serde::Deserialize;
use strum_macros::{EnumString, IntoStaticStr};

#[derive(Debug, EnumString, IntoStaticStr)]
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
}

#[derive(Debug, IntoStaticStr)]
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
        struct ContentDeserializable {
            #[serde(rename = "type")]
            type_: String,
            content: String,
        }

        ContentDeserializable::deserialize(deserializer).and_then(|c| {
            Ok(
                match Type::from_str(c.type_.as_str()).map_err(Error::custom)? {
                    Type::A => Content::A(c.content.parse().map_err(Error::custom)?),
                    Type::Mx => Content::Mx(c.content),
                    Type::Cname => Content::Cname(c.content),
                    Type::Alias => Content::Alias(c.content),
                    Type::Txt => Content::Txt(c.content),
                    Type::Ns => Content::Ns(c.content),
                    Type::Aaaa => Content::Aaaa(c.content.parse().map_err(Error::custom)?),
                    Type::Srv => Content::Srv(c.content),
                    Type::Tlsa => Content::Tlsa(c.content),
                    Type::Caa => Content::Caa(c.content),
                    Type::Https => Content::Https(c.content),
                    Type::Svcb => Content::Svcb(c.content),
                },
            )
        })
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
    s.parse::<i64>().map_err(serde::de::Error::custom)
}
