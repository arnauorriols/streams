const fetch = require("node-fetch");

global.fetch = fetch;
global.Headers = fetch.Headers;
global.Request = fetch.Request;
global.Response = fetch.Response;


let { Subscriber, SendOptions, Address } = require("./streams-vendor");

module.exports = function (RED) {
	function SubscriberNode(config) {
		Promise.resolve(config).then(
			async (config) => {
				RED.nodes.createNode(this, config);
				const node = this;
				const nodeUrl = config.nodeUrl || "https://chrysalis-nodes.iota.org/";
				const options = new SendOptions(nodeUrl, true);
				const subscriber = new Subscriber(config.seed, options);
				let announcementLink = Address.parse(config.announcementLink);
				this.status({ fill: "yellow", shape: "ring", text: "receiving announcement..." });
				await subscriber.clone().receive_announcement(announcementLink.copy());
				let lastMsgLink = announcementLink;
				this.status({ fill: "yellow", shape: "ring", text: "fetching messages..." });
				while (true) {
					var msg = await subscriber.clone().fetchNextMsg();
					if (msg) {
						node.send([msg, null]);
						lastMsgLink = msg.link;
					} else {
						break;
					}
				}
				this.status({ fill: "green", shape: "dot", text: "ready" });
				node.send([{}, null]);  // notify polling mechanism that the fetching is done
				node.on('input', async (msg) => {
					node.status({ fill: "yellow", shape: "ring", text: "fetching messages..." });
					try {
						while (true) {
							var streamsMsg = await subscriber.clone().fetchNextMsg();
							if (streamsMsg) {
								node.send([streamsMsg, null]);
								lastMsgLink = streamsMsg.link;
							} else {
								break;
							}
						}
						if (msg.payload) {
							node.status({ fill: "yellow", shape: "ring", text: "sending packet..." });
							let res = await subscriber.clone().send_signed_packet(lastMsgLink.copy(), to_bytes("public payload"), to_bytes(JSON.stringify(msg.payload)));
							lastMsgLink = res.link;
							msg.link = res.link;
							node.send([null, msg]);
						}
						this.status({ fill: "green", shape: "dot", text: "ready" });
					} catch (error) {
						node.status({ fill: "red", shape: "dot", text: "Error: " + error });
					}
					node.send([{}, null]);  // notify polling mechanism that the fetching is done
				});
			}
		);
	}
	RED.nodes.registerType("subscriber", SubscriberNode);
}

function to_bytes(str) {
	var bytes = [];
	for (var i = 0; i < str.length; ++i) {
		bytes.push(str.charCodeAt(i));
	}
	return bytes;
}
