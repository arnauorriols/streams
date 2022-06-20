use anyhow::Result;

use streams::User;
use test_fixtures::private_key_did;

#[tokio::test]
async fn can_create_stream_with_topic() -> Result<()> {
    let mut author = User::builder()
        .with_identity(private_key_did())
        .build();
    let channel_address = author.create_stream("<channel root topic>").await?;
    println!("{}", channel_address);
    Ok(())
}
