// Rust
use std::sync::Arc;

// 3rd-arty
use anyhow::{anyhow, Result};
use textwrap::{fill, indent};
use tokio::sync::Mutex;

// IOTA
use identity::{
    core::Timestamp,
    did::MethodScope,
    iota::IotaVerificationMethod,
    prelude::{Client as DIDClient, IotaDocument, KeyPair as DIDKeyPair},
};

// Streams
use streams::{
    id::{DIDInfo, Ed25519, Permissioned, Psk, DID},
    transport::tangle,
    User,
};

use super::utils::{print_send_result, print_user};

const PUBLIC_PAYLOAD: &[u8] = b"PUBLICPAYLOAD";
const MASKED_PAYLOAD: &[u8] = b"MASKEDPAYLOAD";

pub async fn example(transport: Arc<Mutex<tangle::Client>>) -> Result<()> {
    let did_client = DIDClient::new().await?;
    println!("> Making DID with method for the Author");
    let author_did_info = make_did_info(&did_client, "auth_key").await?;
    println!("> Making another DID with method for a Subscriber");
    let subscriber_did_info = make_did_info(&did_client, "sub_key").await?;

    // Generate a simple PSK for storage by users
    let psk = Psk::from_seed("A pre shared key");

    let mut author = User::builder()
        .with_identity(DID::PrivateKey(author_did_info))
        .with_transport(transport.clone())
        .build()?;
    let mut subscriber_a = User::builder()
        .with_identity(DID::PrivateKey(subscriber_did_info))
        .with_transport(transport.clone())
        .build()?;
    let mut subscriber_b = User::builder()
        .with_identity(Ed25519::from_seed("SUBSCRIBERB9SEED"))
        .with_transport(transport.clone())
        .build()?;
    let mut subscriber_c = User::builder()
        .with_identity(psk)
        .with_transport(transport.clone())
        .build()?;

    println!("> Author creates stream and sends its announcement");
    // Start at index 1, because we can. Will error if its already in use
    let announcement = author.create_stream(1).await?;
    print_send_result(&announcement);
    print_user("Author", &author);

    println!("> Subscribers read the announcement to connect to the stream");
    subscriber_a.receive_message(announcement.address()).await?;
    print_user("Subscriber A", &subscriber_a);
    subscriber_b.receive_message(announcement.address()).await?;
    print_user("Subscriber B", &subscriber_b);
    subscriber_c.receive_message(announcement.address()).await?;
    print_user("Subscriber C", &subscriber_c);

    // Predefine Subscriber A
    println!("> Subscribers A and B sends subscription");
    let subscription_a_as_a = subscriber_a.subscribe(announcement.address().relative()).await?;
    print_send_result(&subscription_a_as_a);
    print_user("Subscriber A", &subscriber_a);

    let subscription_b_as_b = subscriber_b.subscribe(announcement.address().relative()).await?;
    print_send_result(&subscription_b_as_b);
    print_user("Subscriber A", &subscriber_b);

    println!("> Author stores the PSK used by Subscriber C");
    author.add_psk(psk);

    println!("> Author reads subscription of subscribers A and B");
    let _subscription_a_as_author = author.receive_message(subscription_a_as_a.address()).await?;
    let subscription_b_as_author = author.receive_message(subscription_b_as_b.address()).await?;
    print_user("Author", &author);

    println!("> Author issues keyload for everybody [Subscriber A, Subscriber B, PSK]");
    let first_keyload_as_author = author.send_keyload_for_all(announcement.address().relative()).await?;
    print_send_result(&first_keyload_as_author);
    print_user("Author", &author);

    println!("> Author sends 3 signed packets linked to the keyload");
    let mut last_msg = first_keyload_as_author.clone();
    for _ in 0..3 {
        last_msg = author
            .send_signed_packet(last_msg.address().relative(), PUBLIC_PAYLOAD, MASKED_PAYLOAD)
            .await?;
        print_send_result(&last_msg);
    }
    print_user("Author", &author);

    println!("> Author issues new keyload for only Subscriber B and PSK");
    let second_keyload_as_author = author
        .send_keyload(
            last_msg.address().relative(),
            [
                Permissioned::Read(subscription_b_as_author.header().publisher()),
                Permissioned::Read(psk.into()),
            ],
        )
        .await?;
    print_send_result(&second_keyload_as_author);
    print_user("Author", &author);

    println!("> Author sends 2 more signed packets linked to the latest keyload");
    let mut last_msg = second_keyload_as_author;
    for _ in 0..2 {
        last_msg = author
            .send_signed_packet(last_msg.address().relative(), PUBLIC_PAYLOAD, MASKED_PAYLOAD)
            .await?;
        print_send_result(&last_msg);
    }
    print_user("Author", &author);

    println!("> Author sends 1 more signed packet linked to the first keyload");
    let last_msg = author
        .send_signed_packet(first_keyload_as_author.address().relative(), PUBLIC_PAYLOAD, MASKED_PAYLOAD)
        .await?;
    print_send_result(&last_msg);
    print_user("Author", &author);

    println!("> Subscriber C receives 8 messages:");
    let messages_as_c = subscriber_c.fetch_next_messages().await?;
    print_user("Subscriber C", &subscriber_c);
    for message in &messages_as_c {
        println!("\t{}", message.address());
        println!("{}", indent(&fill(&format!("{:?}", message.content()), 140), "\t| "));
        println!("\t---");
    }
    assert_eq!(8, messages_as_c.len());

    println!("> Subscriber B receives 8 messages:");
    let messages_as_b = subscriber_b.fetch_next_messages().await?;
    print_user("Subscriber B", &subscriber_b);
    for message in &messages_as_c {
        println!("\t{}", message.address());
        println!("{}", indent(&fill(&format!("{:?}", message.content()), 140), "\t| "));
        println!("\t---");
    }
    assert_eq!(8, messages_as_b.len());

    println!("> Subscriber A receives 6 messages:");
    let messages_as_a = subscriber_a.fetch_next_messages().await?;
    print_user("Subscriber A", &subscriber_a);
    for message in &messages_as_c {
        println!("\t{}", message.address());
        println!("{}", indent(&fill(&format!("{:?}", message.content()), 140), "\t| "));
        println!("\t---");
    }
    assert_eq!(6, messages_as_a.len());

    Ok(())
}

async fn make_did_info(did_client: &DIDClient, fragment: &str) -> Result<DIDInfo> {
    // Create Keypair to act as base of identity
    let keypair = DIDKeyPair::new_ed25519()?;
    // Generate original DID document
    let mut document = IotaDocument::new(&keypair)?;
    // Sign document and publish to the tangle
    document.sign_self(keypair.private(), document.default_signing_method()?.id().clone())?;
    let receipt = did_client.publish_document(&document).await?;
    let did = document.id().clone();

    let streams_method_keys = DIDKeyPair::new_ed25519()?;
    let method = IotaVerificationMethod::new(
        did.clone(),
        streams_method_keys.type_(),
        streams_method_keys.public(),
        fragment,
    )?;
    if document.insert_method(method, MethodScope::VerificationMethod).is_ok() {
        document.metadata.previous_message_id = *receipt.message_id();
        document.metadata.updated = Timestamp::now_utc();
        document.sign_self(keypair.private(), document.default_signing_method()?.id().clone())?;

        let _update_receipt = did_client.publish_document(&document).await?;
    } else {
        return Err(anyhow!("Failed to update method"));
    }

    Ok(DIDInfo::new(did, fragment.to_string(), streams_method_keys))
}
