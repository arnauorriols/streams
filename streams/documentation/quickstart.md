# Preliminaries

A IOTA Streams user needs a form of identification. There are 3 different kinds of identification available: pre-shared keys, ed25519 keypairs, and DIDs. In this tutorial we are going to use the three of them. Pre-shared keys and ed25519 keypairs don't need any preparation. However, to use a DID you need to perform a couple of setup steps:

```rust
let stronghold_path: PathBuf = "./example-strong.hodl".into();
let password: String = "my-password".into();
let mut did_account: Account = Account::builder()
	.storage(AccountStorage::Stronghold(
		stronghold_path,
		Some(password),
		None,
	))
	.create_identity(IdentitySetup::default())
	.await?;

did_account
	.update_identity()
	.create_method()
	.fragment("<verification method fragment>")
	.apply()
	.await?;
```

Nothing out of the ordinary; first we create a new DID account, configuring it to secure the keys with Stronghold. Then, we create a new verification method that should ideally be used *exclusively* in this stream. Note the name that you give to the method (the *fragment*); we will have to tell that to Streams when building the `User` instance. If you want more details on what are DIDs and how to use them, head out to the [IOTA Identity wiki](https://wiki.iota.org/identity.rs/introduction)

# Create Stream Channel

The first step when using Streams is, well, to create a stream. The first, most essential piece of a stream is its *channel*. The *channel* of a stream is where everything happens: it's where messages are published and users subscribe to. The user that creates a channel is called the *author* of the channel. Other users that participate in the stream *subscribe* to this channel, and are called *subscribers*.

```rust
let mut author = User::builder()
	.with_identity(DID::with_account(
		did_account,
		"<verification method fragment>",
	))
	.build();
let channel_address = author.create_channel(1, "<channel root topic>").await?;
println!("{}", channel_address);
```

In these few lines, we have instantiated a new Streams user, usign the DID account and verification method created earlier, and then we have created a new channel. Notice how the channel creation receives 2 parameters. The first is the numerical *id* that you chose to give to this channel; you have to make sure each channel a user creates has a different number, as there cannot be 2 channels from the same user with the same number. The second parameter is the root *topic* of the channel. We'll cover topics in a bit, so for now you just consider the channel topic as the *title* of the channel.

Notice also how `create_channel()` returns the `channel_address`. This address points to the *channel announcement*. Anybody that wants to subscribe to this channel will have to get this address from the author and *connect* to it.

# Publish public messages

Streams messages can be public or private. Public messages can be read by anybody that knows the address of the channel. Private messages can be read only by those that have subscribed to the channel and have been granted permission to read by the author of the channel. Right after creating a channel, only public messages can be sent:

```rust
author
	.message()
	.with_payload(b"Welcome to my sensor readings channel")
	.public()
	.send()
	.await?;
author
	.message()
	.public()
	.signed()
	.with_payload(
		b"
subscription prices:
- temperature: 1 MIOTA
- humidity: 1.2 MIOTA
- pressure: 1.4 MIOTA
",
	)
	.send()
	.await?;
```

Besides the binary payload of the message, notice 2 optional modifiers of the message. The first is `public()`, which changes the default behaviour that is to send private messages. The second is `signed()`, which authenticates the message by signing it with the identity of the publisher, instead of a MAC.

# Read public messages

As mentioned, public messages can be read by anybody that knows the channel address, without the need to subscribe to it:

```rust
let mut anon = User::new();
anon.connect("<channel_address>").await?;
let messages = anon.messages();
while let Some(message) = messages.try_next().await? {
	println!("{}", message);
}
```

notice how an anonymous user can use `User::new()` without any parameter at all; all the options are designed with sane defaults so that the out-of-the-box experience is as smooth as possible (by default, a user is anonymous, and uses the IOTA mainnet as transport of the messages). Notice also how `User::connect(&str)` takes a `&str` for parameter; this string is the channel address shared by the author of the channel.

# Subscribe to a channel

This is where things start to get interesting. Publishing public messages is good, but where Streams really shines is in its capacity to secure and control access to the messages. The first step for sending private messages is to have identified *participants* of the channel. All *participants* of a channel (other than the author) are called *subscribers*, as they *subscribe* to the channel.

```rust
let mut subscriber1 = User::builder()
	.with_identity(DID::with_private_key(
		private_key,
		"verification method fragment",
	))
	.with_alias(Keypair::random())
	.build();
subscriber1.connect("<channel_address>").await?;
let subscription_address = subscriber1.subscribe().await?;

let mut subscriber2 = User::builder()
	.with_identity(Keypair::from_seed("my secret seed"))
	.with_alias(Keypair::from_seed("my secret alias seed"))
	.build();
subscriber2.connect("<channel_address>").await?;

let mut generic_reader = User::builder()
	.with_identity(Psk::from_seed(
		"the secret seed shared among generic readers",
	))
	.build();
generic_reader
	.connect("<channel_address>")
	.await?;
```

In this example, we are creating 3 instances of subscribers, showcasing the different kinds of identification that users can use within Streams. The first uses a DID just like the author above, but instead of using the account, passes the verification method's private key to Streams. This is arguably less secure than passing the account, but it's the only option in those environments where the account is not compatible. The second kind of identification is an ed25519 Keypair. A keypair can be generated from a seed (so that it can be reproduced) or randomly. If generated randomly, keep in mind that once inside Streams, the private key cannot be recovered! The third kind of identification is a pre-shared key, also generated usually from a seed.

Notice also how subscriber1 and subscriber2 configure an alias besides their identity, and how subscriber1 calls `User::subscribe()`, while the others don't. We'll see why in the following step.

# Accept subscribers into the channel

For a subscriber to be fully subscribed to a channel, the author of the channel must *accept* the subscriber. There are 2 ways to accept a subscriber: if the subscriber has issued a *subscription request* with `User::subscribe()`, she has to share the *address* of the request with the author, who then can decide whether to *accept* (or not) the subscriber. Alternatively, the subscriber can share its `Identifier` (the public part of its `Identity`) and the author can add it manually to the channel.

```rust
let subscriber1_identifier = author
	.accept_subscription("<subscription_address>")
	.await?;
let subscriber2_identifier = Identifier::public_key("<subscriber2 alias public key>");
let generic_readers_identifier =
	Identifier::psk_id(Psk::from_seed("the secret seed shared among generic readers").id());
let is_new = author.add_subscriber(subscriber2_identifier.clone());
let is_new = author.add_subscriber(generic_readers_identifier.clone());
```

Notice how the identifier of the subscriber2 is its alias public-key, instead of its true identity public-key! The identifier of the subscribers is what is used to control the permissions and tracking of messages. For this reason, all subscribers must know the identifier of all the other subscribers. While the identifier contains public information of the user, the participation of the subscriber on that particular channel (or any of its internal branches, as we'll learn below) might be sensible information that should remain confidential. For this reason, users might prefer to be identified with an alias for all the administrative purposes, and if they are publishers of messages, opt to reveal their true identity by signing the messages.

# Publish private messages

When added to a channel, subscribers by default have only read permission. The author of the channel must explicitly grant write permission to those subscribers that should be allowed to publish messages. In this step, we create a new subscriber, the *device*. The author accepts its subscription request, and then sets its permission to *write*. The device then publishes readings from its sensors every hour on different *topics*.

```rust
let mut device = user::builder()
	.with_identity(DID::with_account(device_did_account, "streams"))
	.build();
device.connect("<channel_address>").await?;
let dev_subscription_address = device.subscribe().await?;

// The author, somewhere else...
let device_identifier = author
	.accept_subscription("<dev_subscription_address>")
	.await?;
author
	.permissions("<channel root topic>")
	.change(Permission::write(device_identifier))
	.apply()
	.await?;

loop {
	device
		.message()
		.with_topic("building1/room5/temperature")
		.with_payload(temp_sensor.read().to_bytes())
		.signed()
		.send()
		.await?;

	device
		.message()
		.with_topic("building1/room5/humidity")
		.with_payload(humidity_sensor.read().to_bytes())
		.signed()
		.send()
		.await?;

	sleep(Duration::from_secs(1 * 60 * 60));
}
```

Notice how the author changes the permissions of the *channel root topic*. A *topic* is the name given by the users to a *branch* of messages. Initially, an Streams channel has a single branch, called the *root branch*. When creating the channel, the author gives a *topic* to that root branch. All subscribers are added by default to this branch with readonly permission. Messages published without an explicit topic are published to the root branch. In this example, on the other hand, the device publishes its messages with another topic! When a user first publishes to an unexisting branch, the branch is *branched from* the root branch, inheriting its access list (the list of subscribers and their permissions). As the device has write access over the root branch, it is allowed to create new branches from it, and publish messages to them. For now, the rest of subscribers have readonly access to these new branches, but in the following step we'll learn how we can further restrict this.

# Control Access to branches

There are various benefits to organizing the messages into branches. One of them is the ability of the author of the channel to control the access of subscribers to these groups of messages:

```rust
author
	.permissions("building1/room5/temperature")
	.remove(&[subscriber2_identifier])
	.apply()
	.await?;
author
	.permissions("building1/room5/humidity")
	.set(&[
		Permission::write(device_identifier),
		Permission::read(subscriber2_identifier),
		Permission::read(generic_readers_identifier),
	])
	.apply()
	.await?;
author
	.permissions("agora")
	.set(
		user.permissions("<channel root topic>")
			.map(Permission::write),
	)
	.apply()
	.await?;
```

In this step, we have changed the permissions of the branches so that:

- `building1/room5/temperature` is readable by subscriber1 and generic readers, and writable by the device
- `building1/room5/humidity` is readable by subscriber2 and generic readers, and writable by the device
- a new branch `agora` is writable by all subscribers

Now is a good time to do a mention regarding pre-shared keys. Pre-shared keys are an special kind of identity; they are thought off as a mechanism to grant *readonly* access to many subscribers using a single key. The important idea to remember is that pre-shared keys do not represent an individual, but rather a group of individuals that share the same key.

# Read messages from a selection of topics and publishers

Reading private messages is done just like reading public messages shown at the beginning of this tutorial. However, branches also open the possibility to consume only parts of the messages published at the channel, by selecting from which branches we want to fetch messages. We can select one branch, multiple branches, the identifier of a particular publisher, or any combination of the above:

```rust
let messages = subscriber1.messages().from(&[
	Selector::topic("building1/room5/temperature"),
	Selector::topic("agora"),
]);
while let Some(message) = messages.try_next().await? {
	// Do your thing
}

// generic_reader, somewhere else...
let device_identifier =
	Identifier::did("did:iota:8dQAzVbbf6FLW9ckwyCBnKmcMGcUV9LYJoXtgQkHcNQy#streams");
let messages = generic_reader
	.messages()
	.from(&[Selector::identifier(device_identifier)]);
while let Some(message) = messages.try_next().await? {
	// Do your thing
}
```

Notice how in this example subscriber1 iterates over the messages of the branch `building1/room5/temperature` and `agora`, while the generic reader fetches any message published by the device, ignoring any message that might be published by the other subscribers in the `agora`.

# Stateless recovery

At this point we need to stop for a moment and explain a bit of the internal mechanics of Streams. Streams is a *stateless* protocol. What this means (at least for Streams) is that there isn't any centralized server or broker that processes, manages and stores administrative data like branches and permissions. In Streams, all this data is processed by the user's instances (in the end of the day, there's nothing else than user instances in Streams). The upside of this approach is that all the data is available in the messages themselves (properly secured) and any user (even the author) can consistently recreate its state by rereading all the messages of the channel (an operation called *synchronizing with the channel*):

```rust
let mut author = User::builder()
	.with_identity(DID::with_account(
		did_account,
		"<verification method fragment>",
	))
	.build();
author.connect("<channel_address>").await?;
author.sync().await?;
```

Technically speaking, it is not necessary to call `author.sync()`, as the user instance automatically synchronizes with the channel on any operation. However, as stateless recovery can take quite a lot of time its advisable to manually force a synchronization at a controlled time.

# Backup & restore

The downside of being an stateless protocol is that the users do all the work. Particularly the *stateless* recovery can take quite a lot of time once the stream has accumulated enough messages. For this reason, it is advisable to take a mixed approach by backing up the state of the user instance from time to time and use these backups as snapshots from where the user instance can perform the rest of the synchronization.

Streams provides an straighforward backup and restore mechanism. The backup is secured with a password and generates a binary blob that can be stored at your preferred storage solution:

```rust
let encrypted_backup = device.backup("my secret backup password").await;
// This is pseudocode, use your preferred storage solution!
backup_storage.write("device backup", encrypted_backup);
let encrypted_backup = backup_storage.read("device backup");
let device = User::restore("my secret backup password", encrypted_backup).await?;
device.selective_sync(&[
	Selector::topic("building1/room5/temperature"),
	Selector::topic("building1/room5/humidity"),
])
.await?;
```

Notice how in this example we call `device.selective_sync(&[Selector])` instead of `device.sync()`. While users must process all messages, they actually only need to process the minimal set of messages they need to be able to progress in the branches they are currently interested in. In other words, branches act as a logical separation of *chains* of messages, and can be handled independently. In this particular example, the device only needs to synchronize the branches it publishes to, as it does not care about the rest of the branches of the channel, even though it is technically included in them. As the device is the only publisher in these branches, it will only need to sync any message published by itself after the last backup, and any new message that the author might have published to change the access list of these branches.

# Grant admin permissions to subscribers and create subbranches

Permissions over the branches can normally be controlled only by the author alone. However, the author can also designate subscribers as *administrators* of a branch, granting them permission to control the permissions of other subscribers within the branch or any sub-branch (ie. any new branch *branched from* the branch that they administer). This can be very useful when setting up a multi-tenant solution; the author creates a branch for each tenant and designates one or more subscribers as administrators of each tenant branch (in our example, `subscriber-1 workspace` is the tenant branch, and subscriber-1 the administrator of the branch). Within that branch, the administrator can create as many sub-branches as needed, and decide freely which subscribers can read and write message to them (from all the subscribers that have been accepted to the channel by the author).

```rust
author
	.permissions("subscriber-1 workspace")
	.set(&[Permission::admin(subscriber1_identifier)])
	.apply()
	.await?;

// The subscriber-1, somewhere else...
let subscriber2_identifier = Identifier::public_key("<subscriber2 public key>");
subscriber1
	.branch_from(
		"subscriber-1 workspace",
		"subscriber-1 workspace/collaborators/subscriber-2",
	)
	.await?;
subscriber1
	.permissions("subscriber-1 workspace/collaborators/subscriber-2")
	.add(&[Permission::write(subscriber2_identifier)])
	.apply()
	.await?;

// The subscriber-2, somewhere else...
subscriber2
	.branch_from(
		"subscriber-1 workspace/collaborators/subscriber-2",
		"subscriber-1 workspace/collaborators/subscriber-2/predictions",
	)
	.await?;
subscriber2
	.message()
	.topic("subscriber-1 workspace/collaborators/subscriber-2/predictions")
	.payload(prediction_engine.output().to_bytes())
	.send()
	.await?;
```

Notice how the subscriber1 needs to manually create the sub-branches. Normally, branches are created automatically whenever a user publishes with a new topic. However, to be able to do that, users must have write permissions over the channel's *root branch*, because branches are by default *branched from* the root branch. In our previous examples, the device is granted write permission over the root branch, but not subscriber-1. Subscriber-1 has only read permission over the root branch, and the administration permission that the author extends to her in this snippet is only valid within the realm of the branch `subscriber-1 workspace`.

In this example, susbcriber-1 creates a sub-branch of `susbcriber-1 workspace` (with `User::branch_from()`) for an external collaborator, subscriber-2, and gives write permission over this sub-branch to her. Notice how this gives subscriber-2 the ability to create yet another sub-branch *branched from* the branch she has write permission over, `subscriber-1 workspace/collaborators/subscriber-2`. To create branches, subscribers need *write* permission; to control access to them, they need *admin* permission.

# Peek messages without replacing state

The following couple of topics require first another technical clarification. In order to perform the cryptographic control of the permissions over the messages, these need to be read in order, because one message cannot be decrypted without the proper *cryptographic state* resulting from the decryption of the previous message, which in turn cannot be decrypted without the proper *cryptographic state* of the previous message, and so on right until the initial message of the channel, the *channel announcement* message.

For this reason, the user instance keeps a cache of the *cryptographic state* of certain messages in memory: the state of the administrative messages, and the state of the last message of each branch, what is called the *tip* of the branch. This is usually not a problem, but it implies that once a message is fetched, the previous one can no longer be fetched (unless the whole branch is reread), and this can be too rigid for certain use cases.

For example, what happens if there are messages that by themselves cannot be fully processed, and need to wait for future messages before they can be understood? A common case of this is when sensors have a complicated encoding, that varies according to an operation mode that is reported in another message, or when sensors emit error checks right after the reading. An approach could be to keep the messages in memory until the rest of the messages a received, but this can because a messy implementation and can end up with consistency issues if not implemented right. Luckily, Streams provides a straighforward API to *peek* into the next messages without losing the state of the current tip:

```rust
let mut messages = subscriber1
	.messages()
	.from(&[Selector::topic("building1/room5/temperature")]);
loop {
	if let &[reading, sensor_mode, error_check] = messages.peek(3).await? {
		if error_check.payload() == b"\0" {
			let value = match sensor_mode.payload() {
				b"\x01" => mode_1.decode(reading.payload()),
				b"\x02" => mode_2.decode(reading.payload()),
				b"\x03" => mode_3.decode(reading.payload()),
			};
		}
		messages = messages.skip(3);
	}
    sleep(Duration::from_secs(30));
}
```

Notice how `message.peek(3)` is called every 30 seconds. `message.peek(3)` returns at most 3 next messages (or less if there aren't enough). In this example, we keep peeking the next 3 messages until there are indeed 3 messages available, at which point we can proceed decoding the sensor reading and advance the *cursor* (in this case with `messages.skip(3)`). Even if `peek()` is called multiple times, messages are only fetched once and then kept in a separate cache until they are "officially" consumed by advancing the *cursor*.

# Direct message access and manual cache flush

WIP

# Use a private Tangle

So far all the examples assume the user uses the IOTA mainnet network, which is the default if no other is configured. Streams can also be used with IOTA testnet, or any private Tangle:

```rust
let mut author = User::builder()
	.with_node_url("https://my-tangle.io")
	.with_identity(DID::account(did_account, "streams"))
	.build();
```

# Use a custom transport client

Streams by default uses an internal iota.rs client to perform the necessary requests to store the messages in the Tangle. However, this client (known as *the transport client*) can be instantiated outside of Streams with custom options and passed to the user builder. Actually, while the Tangle adds unique properties to a stream, it's not the only transport over which one can create a stream! The *transport layer* is abstracted in a very simple interface that can be implemented for any solution that provides essentially a key/value storage interface, including adhoc smart-contracts, or even traditional databases.

```rust
let mut author = User::builder()
	.with_transport(MyTransportClient::new("https://my-fancy-sc.io"))
	.with_identity(Keypair::from_seed("my secret seed"))
	.build();
```