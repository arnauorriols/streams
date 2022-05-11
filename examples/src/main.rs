#![allow(non_snake_case)]

use core::cell::RefCell;

use std::env;

use rand::Rng;

use iota_streams::{
    app::transport::tangle::client::Client,
    app_channels::api::tangle::{
        ChannelType,
        Transport,
    },
    core::prelude::{
        Rc,
        String,
    },
};

mod branching;

async fn run_recovery_single_branch_test<T: Transport>(transport: T, seed: &str) {
    println!("\tRunning Recovery Test (single-branch), seed: {}", seed);
    match branching::recovery::example(transport, ChannelType::SingleBranch, seed).await {
        Err(err) => println!("Error in recovery (single-branch) test: {:?}", err),
        Ok(_) => println!("\tRecovery test (single-branch) completed!!"),
    }
    println!("#######################################");
}

async fn run_recovery_multi_branch_test<T: Transport>(transport: T, seed: &str) {
    println!("\tRunning Recovery Test (multi-branch), seed: {}", seed);
    match branching::recovery::example(transport, ChannelType::MultiBranch, seed).await {
        Err(err) => println!("Error in recovery (multi-branch) test: {:?}", err),
        Ok(_) => println!("\tRecovery test (multi-branch) completed!!"),
    }
    println!("#######################################");
}

async fn run_single_branch_test<T: Transport>(transport: T, seed: &str) {
    println!("\tRunning Single Branch Test, seed: {}", seed);
    match branching::single_branch::example(transport, ChannelType::SingleBranch, seed).await {
        Err(err) => println!("Error in Single Branch test: {:?}", err),
        Ok(_) => println!("\tSingle Branch Test completed!!"),
    }
    println!("#######################################");
}

async fn run_single_depth_test<T: Transport>(transport: T, seed: &str) {
    println!("\tRunning Single Branch Test, seed: {}", seed);
    match branching::single_depth::example(transport, ChannelType::SingleDepth, seed).await {
        Err(err) => println!("Error in Single Depth test: {:?}", err),
        Ok(_) => println!("\tSingle Depth Test completed!!"),
    }
    println!("#######################################");
}

async fn run_multi_branch_test<T: Transport>(transport: T, seed: &str) {
    println!("\tRunning Multi Branch Test, seed: {}", seed);
    match branching::multi_branch::example(transport, ChannelType::MultiBranch, seed).await {
        Err(err) => println!("Error in Multi Branch test: {:?}", err),
        Ok(_) => println!("\tMulti Branch Test completed!!"),
    }
    println!("#######################################");
}

async fn main_pure() {
    let transport = iota_streams::app_channels::api::tangle::BucketTransport::new();

    println!("#######################################");
    println!("Running pure tests without accessing Tangle");
    println!("#######################################");
    println!("\n");

    // BucketTransport is an in-memory storage that needs to be shared between all the users,
    // hence the Rc<RefCell<BucketTransport>>
    let transport = Rc::new(RefCell::new(transport));

    run_single_branch_test(transport.clone(), "PURESEEDA").await;
    run_single_depth_test(transport.clone(), "PURESEEDB").await;
    run_multi_branch_test(transport.clone(), "PURESEEDC").await;
    run_recovery_single_branch_test(transport.clone(), "PURESEEDD").await;
    run_recovery_multi_branch_test(transport.clone(), "PURESEEDF").await;
    println!("Done running pure tests without accessing Tangle");
    println!("#######################################");
}

async fn main_client() {
    // Parse env vars with a fallback
    let node_url = env::var("URL").unwrap_or_else(|_| "https://chrysalis-nodes.iota.org".to_string());

    let transport = Client::new_from_url(&node_url);

    println!("#######################################");
    println!("Running tests accessing Tangle via node {}", &node_url);
    println!("#######################################");
    println!("\n");

    run_single_branch_test(transport.clone(), &new_seed()).await;
    run_single_depth_test(transport.clone(), &new_seed()).await;
    run_multi_branch_test(transport.clone(), &new_seed()).await;
    run_recovery_single_branch_test(transport.clone(), &new_seed()).await;
    run_recovery_multi_branch_test(transport.clone(), &new_seed()).await;
    println!("Done running tests accessing Tangle via node {}", &node_url);
    println!("#######################################");
}

fn new_seed() -> String {
    let alph9 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
    (0..10)
        .map(|_| alph9.chars().nth(rand::thread_rng().gen_range(0, 27)).unwrap())
        .collect::<String>()
}

#[tokio::main]
async fn main() {
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
