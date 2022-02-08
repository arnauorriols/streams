use iota_streams_app::transport::tangle::client::Client;
use iota_streams_app_channels::api::tangle::test::example;

#[tokio::main]
async fn main() {
    let tsp = Client::new_from_url("https://nodes.devnet.iota.org:443");
    assert!(dbg!(example(tsp).await).is_ok());
}
