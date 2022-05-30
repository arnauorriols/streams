// Rust
use std::{env, sync::Arc};

// 3rd-party
use anyhow::Result;
use rand::Rng;
use tokio::sync::Mutex;

// IOTA

// Streams
use streams::{
    transport::{bucket, tangle, Transport},
    TransportMessage,
};

mod scenarios;

trait GenericTransport:
    for<'a> Transport<'a, Msg = TransportMessage, SendResponse = TransportMessage> + Clone + Send
{
}

impl<T> GenericTransport for T where
    T: for<'a> Transport<'a, Msg = TransportMessage, SendResponse = TransportMessage> + Clone + Send
{
}

async fn run_did_scenario(transport: Arc<Mutex<tangle::Client>>) -> Result<()> {
    println!("## Running DID Test ##\n");
    let result = scenarios::did::example(transport).await;
    match &result {
        Err(err) => eprintln!("Error in DID test: {:?}", err),
        Ok(_) => println!("\n## DID test completed successfully!! ##\n"),
    }
    result
}

async fn run_basic_scenario<T: GenericTransport>(transport: T, seed: &str) -> Result<()> {
    println!("## Running single branch test with seed: {} ##\n", seed);
    let result = scenarios::basic::example(transport, seed).await;
    match &result {
        Err(err) => eprintln!("Error in Single Branch test: {:?}", err),
        Ok(_) => println!("\n## Single Branch Test completed successfully!! ##\n"),
    };
    result
}

async fn main_pure() -> Result<()> {
    println!("\n");
    println!("###########################################");
    println!("Running pure tests without accessing Tangle");
    println!("###########################################");
    println!("\n");

    // BucketTransport is an in-memory storage that needs to be shared between all the users,
    // hence the Arc<Mutex<BucketTransport>>
    let transport = Arc::new(Mutex::new(bucket::Client::new()));

    run_basic_scenario(transport.clone(), "PURESEEDA").await?;
    println!("################################################");
    println!("Done running pure tests without accessing Tangle");
    println!("################################################");
    Ok(())
}

async fn main_client() -> Result<()> {
    // Parse env vars with a fallback
    let node_url = env::var("URL").unwrap_or_else(|_| "https://chrysalis-nodes.iota.org".to_string());

    println!("\n");
    println!("########################################{}", "#".repeat(node_url.len()));
    println!("Running tests accessing Tangle via node {}", &node_url);
    println!("########################################{}", "#".repeat(node_url.len()));
    println!("\n");

    let transport =
        Arc::new(Mutex::new(tangle::Client::for_node(&node_url).await.unwrap_or_else(
            |e| panic!("error connecting Tangle client to '{}': {}", node_url, e),
        )));

    run_basic_scenario(transport.clone(), &new_seed()).await?;
    run_did_scenario(transport).await?;
    println!(
        "#############################################{}",
        "#".repeat(node_url.len())
    );
    println!("Done running tests accessing Tangle via node {}", &node_url);
    println!(
        "#############################################{}",
        "#".repeat(node_url.len())
    );
    Ok(())
}

fn new_seed() -> String {
    let alph9 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
    (0..10)
        .map(|_| alph9.chars().nth(rand::thread_rng().gen_range(0..27)).unwrap())
        .collect::<String>()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load or .env file, log message if we failed
    if dotenv::dotenv().is_err() {
        println!(".env file not found; copy and rename example.env to \".env\"");
    };

    match env::var("TRANSPORT").ok().as_deref() {
        Some("tangle") => main_client().await,
        Some("bucket") | None => main_pure().await,
        Some(other) => panic!("Unexpected TRANSPORT '{}'", other),
    }
}
