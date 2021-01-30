# TS3

[![Crates.io](https://img.shields.io/crates/v/ts3)](https://crates.io/crates/ts3)
[![Docs.rs](https://docs.rs/ts3/badge.svg)](https://docs.rs/ts3)

An async library to connect to the ts3 serverquey interface.

## Examples

```
use ts3::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a new client
    let client = Client::new("127.0.0.1:10011").await?;

    // Connect to virtualserver 1
    client.use_sid(1).await?;

    // Use whoami to fetch info about the query client
    let data = client.whoami().await?;

    println!("{}", data);
}
```
