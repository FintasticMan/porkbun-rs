use porkbun::{Client, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::builder()
        .set_default("endpoint", "https://api.porkbun.com/api/json/v3")?
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("PORKBUN"))
        .build()?;

    let config: porkbun::Config = config.try_deserialize()?;

    let client = Client::new(config);

    println!(
        "{}",
        client.create_dns("riverhill.xyz", Some(""), Content::A("0.0.0.0".parse()?))?
    );

    Ok(())
}
