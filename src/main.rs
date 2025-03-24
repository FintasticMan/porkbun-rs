use porkbun::{record::Content, Client};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::builder()
        .set_default("endpoint", "https://api.porkbun.com/api/json/v3")?
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("PORKBUN"))
        .build()?;

    let config: porkbun::Config = config.try_deserialize()?;

    let client = Client::new(config);

    dbg!(client.retrieve_dns("riverhill.xyz", None)?);

    Ok(())
}
