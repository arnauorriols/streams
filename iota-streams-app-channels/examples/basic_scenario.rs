use iota_streams_app::transport::tangle::client::Client;
use iota_streams_app_channels::api::tangle::test::example;

fn main() {
    let tsp = Client::new_from_url("https://nodes.devnet.iota.org:443");
    assert!(dbg!(smol::block_on(example(tsp))).is_ok());
}
