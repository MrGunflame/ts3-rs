use ts3::{Client, Decode};
use async_trait::async_trait;

#[tokio::main]
async fn main() {
    let password = "bN5cSqv+";


    let client = Client::new("localhost:10011").await.unwrap();

    client.set_event_handler(Handler);

    client.login("serveradmin", password).await.unwrap();
    client.use_sid(1).await.unwrap();

    client.servernotifyregister(ts3::client::ServerNotifyRegister::TextPrivate).await.unwrap();

    loop {}
}

struct Handler;

#[async_trait]
impl ts3::EventHandler for Handler {
    async fn textmessage(&self, client: Client, event: ts3::event::TextMessage) {
        println!("{:?}", event);
        println!("{} said: {}", event.invokername, event.msg);
    }
}
