/* tslint:disable */
/* eslint-disable */
/**
*/
export function set_panic_hook(): void;
/**
*/
export enum ChannelType {
  SingleBranch,
  MultiBranch,
  SingleDepth,
}
/**
*/
export enum LedgerInclusionState {
  Conflicting,
  Included,
  NoTransaction,
}
/**
* Tangle representation of a Message Link.
*
* An `Address` is comprised of 2 distinct parts: the channel identifier
* ({@link ChannelAddress}) and the message identifier
* ({@link MsgId}). The channel identifier is unique per channel and is common in the
* `Address` of all messages published in it. The message identifier is
* produced pseudo-randomly out of the the message's sequence number, the
* previous message identifier, and other internal properties.
*/
export class Address {
  free(): void;
/**
* @param {ChannelAddress} channel_address
* @param {MsgId} msgid
*/
  constructor(channel_address: ChannelAddress, msgid: MsgId);
/**
* Generate the hash used to index the {@link Message} published in this address.
*
* Currently this hash is computed with {@link https://en.wikipedia.org/wiki/BLAKE_(hash_function)#BLAKE2|Blake2b256}.
* The returned Uint8Array contains the binary digest of the hash. To obtain the hexadecimal representation of the
* hash, use the convenience method {@link Address#toMsgIndexHex}.
* @returns {Uint8Array}
*/
  toMsgIndex(): Uint8Array;
/**
* Generate the hash used to index the {@link Message} published in this address.
*
* Currently this hash is computed with {@link https://en.wikipedia.org/wiki/BLAKE_(hash_function)#BLAKE2|Blake2b256}.
* The returned String contains the hexadecimal digest of the hash. To obtain the binary digest of the hash,
* use the method {@link Address#toMsgIndex}.
* @returns {string}
*/
  toMsgIndexHex(): string;
/**
* Render the `Address` as a colon-separated String of the hex-encoded {@link Address#channelAddress} and
* {@link Address#msgId} (`<channelAddressHex>:<msgIdHex>`) suitable for exchanging the `Address` between
* participants. To convert the String back to an `Address`, use {@link Address.parse}.
*
* @see Address.parse
* @returns {string}
*/
  toString(): string;
/**
* Decode an `Address` out of a String. The String must follow the format used by {@link Address#toString}
*
* @throws Throws an error if String does not follow the format `<channelAddressHex>:<msgIdHex>`
*
* @see Address#toString
* @see ChannelAddress#hex
* @see MsgId#hex
* @param {string} string
* @returns {Address}
*/
  static parse(string: string): Address;
/**
* @returns {Address}
*/
  copy(): Address;
/**
* @returns {ChannelAddress}
*/
  readonly channelAddress: ChannelAddress;
/**
* @returns {MsgId}
*/
  readonly msgId: MsgId;
}
/**
*/
export class Author {
  free(): void;
/**
* @param {string} seed
* @param {SendOptions} options
* @param {number} implementation
*/
  constructor(seed: string, options: SendOptions, implementation: number);
/**
* @param {Client} client
* @param {string} seed
* @param {number} implementation
* @returns {Author}
*/
  static from_client(client: Client, seed: string, implementation: number): Author;
/**
* @param {Client} client
* @param {Uint8Array} bytes
* @param {string} password
* @returns {Author}
*/
  static import(client: Client, bytes: Uint8Array, password: string): Author;
/**
* @param {string} password
* @returns {Uint8Array}
*/
  export(password: string): Uint8Array;
/**
* @returns {Author}
*/
  clone(): Author;
/**
* @returns {string}
*/
  channel_address(): string;
/**
* @returns {boolean}
*/
  is_multi_branching(): boolean;
/**
* @returns {Client}
*/
  get_client(): Client;
/**
* @param {string} psk_seed_str
* @returns {string}
*/
  store_psk(psk_seed_str: string): string;
/**
* @returns {string}
*/
  get_public_key(): string;
/**
* @returns {Promise<UserResponse>}
*/
  send_announce(): Promise<UserResponse>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  send_keyload_for_everyone(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @param {PskIds} psk_ids
* @param {PublicKeys} sig_pks
* @returns {Promise<UserResponse>}
*/
  send_keyload(link: Address, psk_ids: PskIds, sig_pks: PublicKeys): Promise<UserResponse>;
/**
* @param {Address} link
* @param {Uint8Array} public_payload
* @param {Uint8Array} masked_payload
* @returns {Promise<UserResponse>}
*/
  send_tagged_packet(link: Address, public_payload: Uint8Array, masked_payload: Uint8Array): Promise<UserResponse>;
/**
* @param {Address} link
* @param {Uint8Array} public_payload
* @param {Uint8Array} masked_payload
* @returns {Promise<UserResponse>}
*/
  send_signed_packet(link: Address, public_payload: Uint8Array, masked_payload: Uint8Array): Promise<UserResponse>;
/**
* @param {Address} link_to
* @returns {Promise<void>}
*/
  receive_subscribe(link_to: Address): Promise<void>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  receive_tagged_packet(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  receive_signed_packet(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @returns {Promise<Address>}
*/
  receive_sequence(link: Address): Promise<Address>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  receive_msg(link: Address): Promise<UserResponse>;
/**
* @param {Address} anchor_link
* @param {number} msg_num
* @returns {Promise<UserResponse>}
*/
  receive_msg_by_sequence_number(anchor_link: Address, msg_num: number): Promise<UserResponse>;
/**
* Fetch all the pending messages that the user can read so as to bring the state of the user up to date
*
* This is the main method to bring the user to the latest state of the channel in order to be able to
* publish new messages to it. It makes sure that the messages are processed in topologically order
* (ie parent messages before child messages), ensuring a consistent state regardless of the order of publication.
*
* @returns {number} the amount of messages processed
* @throws Throws error if an error has happened during message retrieval.
* @see {@link Author#fetchNextMsg} for a method that retrieves the immediately next message that the user can read
* @see {@link Author#fetchNextMsgs} for a method that retrieves all pending messages and collects
*      them into an Array.
* @returns {Promise<number>}
*/
  syncState(): Promise<number>;
/**
* Fetch all the pending messages that the user can read and collect them into an Array
*
* This is the main method to traverse the a channel forward at once.  It
* makes sure that the messages in the Array are topologically ordered (ie
* parent messages before child messages), ensuring a consistent state regardless
* of the order of publication.
*
* @returns {UserResponse[]}
* @throws Throws error if an error has happened during message retrieval.
* @see {@link Author#fetchNextMsg} for a method that retrieves the immediately next message that the user can read
* @see {@link Author#syncState} for a method that traverses all pending messages to update the state
*      without accumulating them.
* @returns {Promise<Array<any>>}
*/
  fetchNextMsgs(): Promise<Array<any>>;
/**
* Fetch the immediately next message that the user can read
*
* This is the main method to traverse the a channel forward message by message, as it
* makes sure that no message is returned unless its parent message in the branches tree has already
* been returned, ensuring a consistent state regardless of the order of publication.
*
* Keep in mind that internally this method might have to fetch multiple messages until the correct
* message to be returned is found.
*
* @throws Throws error if an error has happened during message retrieval.
* @see {@link Author#fetchNextMsgs} for a method that retrieves all pending messages and collects
*      them into an Array.
* @see {@link Author#syncState} for a method that traverses all pending messages to update the state
*      without accumulating them.
* @returns {Promise<UserResponse | undefined>}
*/
  fetchNextMsg(): Promise<UserResponse | undefined>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  fetch_prev_msg(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @param {number} num_msgs
* @returns {Promise<Array<any>>}
*/
  fetch_prev_msgs(link: Address, num_msgs: number): Promise<Array<any>>;
/**
* Generate the next batch of message {@link Address} to poll
*
* Given the set of users registered as participants of the channel and their current registered
* sequencing position, this method generates a set of new {@link Address} to poll for new messages
* (one for each user, represented by its identifier). However, beware that it is not recommended to
* use this method as a means to implement message traversal, as there's no guarantee that the addresses
* returned are the immediately next addresses to be processed. use {@link Author#fetchNextMsg} instead.
*
* Keep in mind that in multi-branch channels, the link returned corresponds to the next sequence message.
*
* @see Author#fetchNextMsg
* @see Author#fetchNextMsgs
* @returns {NextMsgAddress[]}
* @returns {Array<any>}
*/
  genNextMsgAddresses(): Array<any>;
/**
* @returns {Array<any>}
*/
  fetch_state(): Array<any>;
}
/**
* Channel application instance identifier (40 Byte)
*/
export class ChannelAddress {
  free(): void;
/**
* Render the `ChannelAddress` as a 40 Byte {@link https://developer.mozilla.org/es/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array|Uint8Array}
*
* @see ChannelAddress#hex
* @returns {Uint8Array}
*/
  bytes(): Uint8Array;
/**
* Render the `ChannelAddress` as a 40 Byte (80 char) hexadecimal String
*
* @see ChannelAddress#bytes
* @returns {string}
*/
  hex(): string;
/**
* Render the `ChannelAddress` as an exchangeable String. Currently
* outputs the same as {@link ChannelAddress#hex}.
*
* @see ChannelAddress#hex
* @see ChannelAddress.parse
* @returns {string}
*/
  toString(): string;
/**
* Decode a `ChannelAddress` out of a String. The string must be a 80 char long hexadecimal string.
*
* @see ChannelAddress#toString
* @throws Throws error if string does not follow the expected format
* @param {string} string
* @returns {ChannelAddress}
*/
  static parse(string: string): ChannelAddress;
/**
* @returns {ChannelAddress}
*/
  copy(): ChannelAddress;
}
/**
*/
export class Client {
  free(): void;
/**
* @param {string} node
* @param {SendOptions} options
*/
  constructor(node: string, options: SendOptions);
/**
* @param {Address} link
* @returns {Promise<any>}
*/
  get_link_details(link: Address): Promise<any>;
}
/**
*/
export class Cursor {
  free(): void;
}
/**
*/
export class Details {
  free(): void;
/**
* @returns {MessageMetadata}
*/
  get_metadata(): MessageMetadata;
/**
* @returns {MilestoneResponse | undefined}
*/
  get_milestone(): MilestoneResponse | undefined;
}
/**
*/
export class Message {
  free(): void;
/**
* @returns {Message}
*/
  static default(): Message;
/**
* @param {string | undefined} identifier
* @param {Uint8Array} public_payload
* @param {Uint8Array} masked_payload
* @returns {Message}
*/
  static new(identifier: string | undefined, public_payload: Uint8Array, masked_payload: Uint8Array): Message;
/**
* @returns {string}
*/
  get_identifier(): string;
/**
* @returns {Array<any>}
*/
  get_public_payload(): Array<any>;
/**
* @returns {Array<any>}
*/
  get_masked_payload(): Array<any>;
}
/**
*/
export class MessageMetadata {
  free(): void;
/**
*/
  conflict_reason?: number;
/**
* @returns {Array<any>}
*/
  readonly get_parent_message_ids: Array<any>;
/**
*/
  is_solid: boolean;
/**
*/
  ledger_inclusion_state?: number;
/**
* @returns {string}
*/
  readonly message_id: string;
/**
*/
  milestone_index?: number;
/**
*/
  referenced_by_milestone_index?: number;
/**
*/
  should_promote?: boolean;
/**
*/
  should_reattach?: boolean;
}
/**
*/
export class MilestoneResponse {
  free(): void;
/**
* Milestone index.
*/
  index: number;
/**
* @returns {string}
*/
  readonly message_id: string;
/**
* Milestone timestamp.
*/
  timestamp: BigInt;
}
/**
* Message identifier (12 Byte). Unique within a Channel.
*/
export class MsgId {
  free(): void;
/**
* Render the `MsgId` as a 12 Byte {@link https://developer.mozilla.org/es/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array|Uint8Array}
*
* @see MsgId#hex
* @returns {Uint8Array}
*/
  bytes(): Uint8Array;
/**
* Render the `MsgId` as a 12 Byte (24 char) hexadecimal String
*
* @see MsgId#bytes
* @returns {string}
*/
  hex(): string;
/**
* Render the `MsgId` as an exchangeable String. Currently
* outputs the same as {@link MsgId#hex}.
*
* @see MsgId#hex
* @see MsgId.parse
* @returns {string}
*/
  toString(): string;
/**
* Decode a `MsgId` out of a String. The string must be a 24 char long hexadecimal string.
*
* @see Msgid#toString
* @throws Throws error if string does not follow the expected format
* @param {string} string
* @returns {MsgId}
*/
  static parse(string: string): MsgId;
/**
* @returns {MsgId}
*/
  copy(): MsgId;
}
/**
*/
export class NextMsgAddress {
  free(): void;
/**
* @param {string} identifier
* @param {Address} address
* @returns {NextMsgAddress}
*/
  static new(identifier: string, address: Address): NextMsgAddress;
/**
*/
  address: Address;
/**
*/
  identifier: string;
}
/**
*/
export class PskIds {
  free(): void;
/**
* @returns {PskIds}
*/
  static new(): PskIds;
/**
* @param {string} id
*/
  add(id: string): void;
/**
* @returns {Array<any>}
*/
  get_ids(): Array<any>;
}
/**
*/
export class PublicKeys {
  free(): void;
/**
* @returns {PublicKeys}
*/
  static new(): PublicKeys;
/**
* @param {string} id
*/
  add(id: string): void;
/**
* @returns {Array<any>}
*/
  get_pks(): Array<any>;
}
/**
*/
export class SendOptions {
  free(): void;
/**
* @param {string} url
* @param {boolean} local_pow
*/
  constructor(url: string, local_pow: boolean);
/**
* @returns {SendOptions}
*/
  clone(): SendOptions;
/**
*/
  local_pow: boolean;
/**
* @returns {string}
*/
  url: string;
}
/**
*/
export class Subscriber {
  free(): void;
/**
* @param {string} seed
* @param {SendOptions} options
*/
  constructor(seed: string, options: SendOptions);
/**
* @param {Client} client
* @param {string} seed
* @returns {Subscriber}
*/
  static from_client(client: Client, seed: string): Subscriber;
/**
* @param {Client} client
* @param {Uint8Array} bytes
* @param {string} password
* @returns {Subscriber}
*/
  static import(client: Client, bytes: Uint8Array, password: string): Subscriber;
/**
* @returns {Subscriber}
*/
  clone(): Subscriber;
/**
* @returns {string}
*/
  channel_address(): string;
/**
* @returns {Client}
*/
  get_client(): Client;
/**
* @returns {boolean}
*/
  is_multi_branching(): boolean;
/**
* @param {string} psk_seed_str
* @returns {string}
*/
  store_psk(psk_seed_str: string): string;
/**
* @returns {string}
*/
  get_public_key(): string;
/**
* @returns {string}
*/
  author_public_key(): string;
/**
* @returns {boolean}
*/
  is_registered(): boolean;
/**
*/
  unregister(): void;
/**
* @param {string} password
* @returns {Uint8Array}
*/
  export(password: string): Uint8Array;
/**
* @param {Address} link
* @returns {Promise<void>}
*/
  receive_announcement(link: Address): Promise<void>;
/**
* @param {Address} link
* @returns {Promise<boolean>}
*/
  receive_keyload(link: Address): Promise<boolean>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  receive_tagged_packet(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  receive_signed_packet(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @returns {Promise<Address>}
*/
  receive_sequence(link: Address): Promise<Address>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  receive_msg(link: Address): Promise<UserResponse>;
/**
* @param {Address} anchor_link
* @param {number} msg_num
* @returns {Promise<UserResponse>}
*/
  receive_msg_by_sequence_number(anchor_link: Address, msg_num: number): Promise<UserResponse>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  send_subscribe(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @param {Uint8Array} public_payload
* @param {Uint8Array} masked_payload
* @returns {Promise<UserResponse>}
*/
  send_tagged_packet(link: Address, public_payload: Uint8Array, masked_payload: Uint8Array): Promise<UserResponse>;
/**
* @param {Address} link
* @param {Uint8Array} public_payload
* @param {Uint8Array} masked_payload
* @returns {Promise<UserResponse>}
*/
  send_signed_packet(link: Address, public_payload: Uint8Array, masked_payload: Uint8Array): Promise<UserResponse>;
/**
* Fetch all the pending messages that the user can read so as to bring the state of the user up to date
*
* This is the main method to bring the user to the latest state of the channel in order to be able to
* publish new messages to it. It makes sure that the messages are processed in topologically order
* (ie parent messages before child messages), ensuring a consistent state regardless of the order of publication.
*
* @returns {number} the amount of messages processed
* @throws Throws error if an error has happened during message retrieval.
* @see {@link Author#fetchNextMsg} for a method that retrieves the immediately next message that the user can read
* @see {@link Author#fetchNextMsgs} for a method that retrieves all pending messages and collects
*      them into an Array.
* @returns {Promise<number>}
*/
  syncState(): Promise<number>;
/**
* Fetch all the pending messages that the user can read and collect them into an Array
*
* This is the main method to traverse the a channel forward at once.  It
* makes sure that the messages in the Array are topologically ordered (ie
* parent messages before child messages), ensuring a consistent state regardless
* of the order of publication.
*
* @returns {UserResponse[]}
* @throws Throws error if an error has happened during message retrieval.
* @see {@link Author#fetchNextMsg} for a method that retrieves the immediately next message that the user can read
* @see {@link Author#syncState} for a method that traverses all pending messages to update the state
*      without accumulating them.
* @returns {Promise<Array<any>>}
*/
  fetchNextMsgs(): Promise<Array<any>>;
/**
* Fetch the immediately next message that the user can read
*
* This is the main method to traverse the a channel forward message by message, as it
* makes sure that no message is returned unless its parent message in the branches tree has already
* been returned, ensuring a consistent state regardless of the order of publication.
*
* Keep in mind that internally this method might have to fetch multiple messages until the correct
* message to be returned is found.
*
* @throws Throws error if an error has happened during message retrieval.
* @see {@link Author#fetchNextMsgs} for a method that retrieves all pending messages and collects
*      them into an Array.
* @see {@link Author#syncState} for a method that traverses all pending messages to update the state
*      without accumulating them.
* @returns {Promise<UserResponse | undefined>}
*/
  fetchNextMsg(): Promise<UserResponse | undefined>;
/**
* @param {Address} link
* @returns {Promise<UserResponse>}
*/
  fetch_prev_msg(link: Address): Promise<UserResponse>;
/**
* @param {Address} link
* @param {number} num_msgs
* @returns {Promise<Array<any>>}
*/
  fetch_prev_msgs(link: Address, num_msgs: number): Promise<Array<any>>;
/**
* Generate the next batch of message {@link Address} to poll
*
* Given the set of users registered as participants of the channel and their current registered
* sequencing position, this method generates a set of new {@link Address} to poll for new messages
* (one for each user, represented by its identifier). However, beware that it is not recommended to
* use this method as a means to implement message traversal, as there's no guarantee that the addresses
* returned are the immediately next addresses to be processed. use {@link Subscriber#fetchNextMsg} instead.
*
* Keep in mind that in multi-branch channels, the link returned corresponds to the next sequence message.
*
* @see Subscriber#fetchNextMsg
* @see Subscriber#fetchNextMsgs
* @returns {NextMsgAddress[]}
* @returns {Array<any>}
*/
  genNextMsgAddresses(): Array<any>;
/**
* @returns {Array<any>}
*/
  fetch_state(): Array<any>;
/**
*/
  reset_state(): void;
}
/**
*/
export class UserResponse {
  free(): void;
/**
* @param {Address} link
* @param {Address | undefined} seq_link
* @param {Message | undefined} message
* @returns {UserResponse}
*/
  static new(link: Address, seq_link?: Address, message?: Message): UserResponse;
/**
* @param {string} link
* @param {string | undefined} seq_link
* @param {Message | undefined} message
* @returns {UserResponse}
*/
  static fromStrings(link: string, seq_link?: string, message?: Message): UserResponse;
/**
* @returns {UserResponse}
*/
  copy(): UserResponse;
/**
* @returns {Address}
*/
  readonly link: Address;
/**
* @returns {Message | undefined}
*/
  readonly message: Message | undefined;
/**
* @returns {Address | undefined}
*/
  readonly seqLink: Address | undefined;
}
/**
*/
export class UserState {
  free(): void;
/**
* @param {string} identifier
* @param {Cursor} cursor
* @returns {UserState}
*/
  static new(identifier: string, cursor: Cursor): UserState;
/**
* @returns {number}
*/
  readonly branchNo: number;
/**
* @returns {string}
*/
  readonly identifier: string;
/**
* @returns {Address}
*/
  readonly link: Address;
/**
* @returns {number}
*/
  readonly seqNo: number;
}
