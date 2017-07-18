extern crate devp2p;
extern crate rand;
extern crate secp256k1;
extern crate etcommon_crypto;
extern crate etcommon_bigint as bigint;
extern crate etcommon_rlp as rlp;

#[macro_use]
extern crate log;
#[macro_use]
extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate env_logger;
extern crate url;

use etcommon_crypto::SECP256K1;
use tokio_core::reactor::Core;
use secp256k1::key::{PublicKey, SecretKey};
use rand::os::OsRng;
use futures::future;
use futures::{Stream, Sink, Future};
use std::str::FromStr;
use std::time::Duration;
use devp2p::ETHStream;
use devp2p::dpt::DPTNode;
use bigint::{H256, U256, H512};
use url::Url;

const GENESIS_HASH: &str = "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3";
const GENESIS_DIFFICULTY: usize = 17179869184;
const NETWORK_ID: usize = 1;

const BOOTSTRAP_NODES: [&str; 8] = [
    "enode://e809c4a2fec7daed400e5e28564e23693b23b2cc5a019b612505631bbe7b9ccf709c1796d2a3d29ef2b045f210caf51e3c4f5b6d3587d43ad5d6397526fa6179@174.112.32.157:30303",
	"enode://6e538e7c1280f0a31ff08b382db5302480f775480b8e68f8febca0ceff81e4b19153c6f8bf60313b93bef2cc34d34e1df41317de0ce613a201d1660a788a03e2@52.206.67.235:30303",
	"enode://5fbfb426fbb46f8b8c1bd3dd140f5b511da558cd37d60844b525909ab82e13a25ee722293c829e52cb65c2305b1637fa9a2ea4d6634a224d5f400bfe244ac0de@162.243.55.45:30303",
	"enode://42d8f29d1db5f4b2947cd5c3d76c6d0d3697e6b9b3430c3d41e46b4bb77655433aeedc25d4b4ea9d8214b6a43008ba67199374a9b53633301bca0cd20c6928ab@104.155.176.151:30303",
	"enode://814920f1ec9510aa9ea1c8f79d8b6e6a462045f09caa2ae4055b0f34f7416fca6facd3dd45f1cf1673c0209e0503f02776b8ff94020e98b6679a0dc561b4eba0@104.154.136.117:30303",
	"enode://72e445f4e89c0f476d404bc40478b0df83a5b500d2d2e850e08eb1af0cd464ab86db6160d0fde64bd77d5f0d33507ae19035671b3c74fec126d6e28787669740@104.198.71.200:30303",
	"enode://39abab9d2a41f53298c0c9dc6bbca57b0840c3ba9dccf42aa27316addc1b7e56ade32a0a9f7f52d6c5db4fe74d8824bcedfeaecf1a4e533cacb71cf8100a9442@144.76.238.49:30303",
    "enode://f50e675a34f471af2438b921914b5f06499c7438f3146f6b8936f1faeb50b8a91d0d0c24fb05a66f05865cd58c24da3e664d0def806172ddd0d4c5bdbf37747e@144.76.238.49:30306"
];

fn main() {
    env_logger::init();

    let addr = "0.0.0.0:30303".parse().unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut client = ETHStream::new(
        &addr, &handle,
        SecretKey::new(&SECP256K1, &mut OsRng::new().unwrap()),
        "etclient Rust/0.1.0".to_string(), 1,
        H256::from_str(GENESIS_HASH).unwrap(),
        H256::from_str(GENESIS_HASH).unwrap(),
        U256::from(GENESIS_DIFFICULTY),
        BOOTSTRAP_NODES.iter().map(|v| DPTNode::from_url(&Url::parse(v).unwrap()).unwrap()).collect(),
        Duration::new(600, 0),
        Duration::new(700, 0),
        25,
        Duration::new(5, 0)).unwrap();

    loop {
        let (val, new_client) = core.run(client.into_future().map_err(|(e, _)| e)).unwrap();
        client = new_client;
        println!("received {:?}, active {}", val, client.active_peers().len());
    }
}
