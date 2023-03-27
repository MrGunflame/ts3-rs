use ts3::event::{ClientEnterView, EventHandler};
use ts3::request::{ServerNotifyRegister, TextMessageTarget};
use ts3::{async_trait, Client};

const USERNAME: &str = "serveradmin";
const PASSWORD: &str = "password";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("127.0.0.1:10011").await?;

    client.set_event_handler(Handler);

    client.login(USERNAME, PASSWORD).await?;
    client.use_sid(1).await?;

    client
        .servernotifyregister(ServerNotifyRegister::Server)
        .await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn cliententerview(&self, client: Client, event: ClientEnterView) {
        println!("User joined: {}", event.client_nickname);

        // Greet the joined client with a "Hello World!".
        if let Err(err) = client
            .sendtextmessage(TextMessageTarget::Client(event.clid), "Hello World!")
            .await
        {
            println!("Error greeting user: {}", err);
        }
    }
}
