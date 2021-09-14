# TS3

[![Crates.io](https://img.shields.io/crates/v/ts3)](https://crates.io/crates/ts3)
[![Docs.rs](https://docs.rs/ts3/badge.svg)](https://docs.rs/ts3)

A fully asynchronous library to interact with the TeamSpeak 3 Server query interface.
See the docs [here](https://docs.rs/ts3).

## Usage

Add `ts3` to your `Cargo.toml`:
```
ts3 = "0.3.1"
```

Basic example usage:
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

Documentation and more examples can be found on [docs.rs](https://docs.rs/ts3).

# License

Licensed under either 
- [MIT License](/MrGunflame/ts3-rs/blob/master/LICENSE-MIT)
or
- [Apache License, Version 2.0](/MrGunflame/ts3-rs/blob/master/LICENSE-APACHE)
at your option.
