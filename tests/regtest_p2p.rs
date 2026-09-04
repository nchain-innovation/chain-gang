//! P2P handshake against a live regtest node.
//!
//! These are `#[ignore]`d because they need a node listening on 18444, so CI
//! does not run them. They exist because a wrong `Network::magic` value cannot
//! be caught any other way: the magic bytes are not part of the genesis hash,
//! so every offline test passes while no node will talk to us. `BSV_Regtest`
//! shipped with Bitcoin Core's `0xfabfb5da` until this test was written, and it
//! is Bitcoin Cash's `0xdab5bffa` that a bitcoin-sv node actually speaks.
//!
//! To run them, start a node:
//!
//! ```text
//! docker run -d --name cg-regtest -p 18443:18443 -p 18444:18444 \
//!   bitcoinsv/bitcoin-sv:latest \
//!   bitcoind -regtest -server -listen=1 -bind=0.0.0.0:18444 \
//!     -rpcbind=0.0.0.0 -rpcallowip=0.0.0.0/0 -rpcport=18443 \
//!     -rpcuser=cguser -rpcpassword=cgpass -minminingtxfee=0.000005 -printtoconsole
//! ```
//!
//! then `cargo test --test regtest_p2p -- --ignored --test-threads=1`.
//!
//! Note the node bans a peer that sends a bad message start, so a failing run
//! leaves the address banned. Clear it with `clearbanned` over RPC, or restart
//! the container, before rerunning.

use chain_gang::messages::{Message, Ping, Version, NODE_BITCOIN_CASH, PROTOCOL_VERSION};
use chain_gang::network::Network;
use chain_gang::peer::{Peer, SVPeerFilter};
use chain_gang::util::rx::Observable;
use chain_gang::util::secs_since;
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, UNIX_EPOCH};

const NODE: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const P2P_PORT: u16 = 18444;
const AGENT: &str = "chain-gang-handshake-probe";

fn version() -> Version {
    Version {
        version: PROTOCOL_VERSION,
        services: NODE_BITCOIN_CASH,
        timestamp: secs_since(UNIX_EPOCH) as i64,
        user_agent: AGENT.to_string(),
        ..Default::default()
    }
}

#[test]
#[ignore = "needs a regtest node on 18444; see the module docs"]
fn handshake_completes_on_regtest() {
    let peer = Peer::connect(
        NODE,
        P2P_PORT,
        Network::BSV_Regtest,
        version(),
        SVPeerFilter::new(0),
    );

    peer.connected_event()
        .poll_timeout(Duration::from_secs(15))
        .expect("handshake should complete against a regtest node");

    assert!(peer.connected(), "peer should report itself connected");

    let remote = peer.version().expect("the node's version message");
    assert!(
        remote.user_agent.contains("Bitcoin SV"),
        "unexpected user agent: {}",
        remote.user_agent
    );

    // A round trip proves the framing works past the handshake, not just up to it
    peer.send(&Message::Ping(Ping { nonce: 0x1234_5678 }))
        .expect("send ping");
    peer.messages()
        .poll_timeout(Duration::from_secs(15))
        .expect("a message back after the ping");

    peer.disconnect();
}

#[test]
#[ignore = "needs a regtest node on 18444; see the module docs"]
fn handshake_fails_with_testnet_magic() {
    // The same node and the same code, differing only in the magic bytes. Without
    // this, the test above would pass just as happily against a wrong constant
    // that some other network happened to accept.
    let peer = Peer::connect(
        NODE,
        P2P_PORT,
        Network::BSV_Testnet,
        version(),
        SVPeerFilter::new(0),
    );

    assert!(
        peer.connected_event()
            .poll_timeout(Duration::from_secs(8))
            .is_err(),
        "a testnet-magic handshake must not succeed against a regtest node"
    );
    assert!(!peer.connected());
    peer.disconnect();
}
