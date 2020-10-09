#![allow(dead_code)]

use arrayvec::ArrayString;
use async_trait::async_trait;
use devp2p::*;
use ethereum_types::*;
use hex_literal::hex;
use k256::ecdsa::SigningKey;
use rand::rngs::OsRng;
use rlp_derive::{RlpDecodable, RlpEncodable};
use std::{
    convert::identity,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use task_group::TaskGroup;
use tokio::sync::Mutex as AsyncMutex;
use tracing::*;
use tracing_subscriber::EnvFilter;
use trust_dns_resolver::{config::*, TokioAsyncResolver};
use uuid::Uuid;

const DNS_BOOTNODE: &str = "all.mainnet.ethdisco.net";

#[derive(Debug, Default)]
struct TaskMetrics {
    count: AtomicUsize,
}

impl task_group::Metrics for TaskMetrics {
    fn task_started(&self, id: Uuid, name: String) {
        let c = self.count.fetch_add(1, Ordering::Relaxed);
        trace!("Current tasks: {}. Started task {}/{}", c + 1, name, id)
    }

    fn task_stopped(&self, id: Uuid, name: String) {
        let c = self.count.fetch_sub(1, Ordering::Relaxed);
        trace!("Current tasks: {}. Stopped task {}/{}", c - 1, name, id)
    }
}

#[derive(Debug, RlpEncodable, RlpDecodable)]
struct StatusMessage {
    protocol_version: usize,
    network_id: usize,
    total_difficulty: U256,
    best_hash: H256,
    genesis_hash: H256,
}

#[derive(Debug)]
struct CapabilityServerImpl;

#[async_trait]
impl CapabilityServer for CapabilityServerImpl {
    async fn on_peer_connect(&self, _: PeerId) -> PeerConnectOutcome {
        let status_message = StatusMessage {
            protocol_version: 63,
            network_id: 1,
            total_difficulty: 17608636743620256866935_u128.into(),
            best_hash: H256::from(hex!(
                "28042e7e4d35a3482bf5f0d862501868b04c1734f483ceae3bf1393561951829"
            )),
            genesis_hash: H256::from(hex!(
                "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3"
            )),
        };

        PeerConnectOutcome::Retain {
            hello: Some(Message {
                id: 0,
                data: rlp::encode(&status_message).into(),
            }),
        }
    }
    #[instrument(skip(peer, message), level = "debug", name = "ingress", fields(peer=&*peer.id.to_string(), id=message.id))]
    async fn on_ingress_message(
        &self,
        peer: IngressPeer,
        message: Message,
    ) -> Result<(Option<Message>, Option<ReputationReport>), HandleError> {
        let Message { id, data } = message;

        info!(
            "Received message with id {}, data {}",
            id,
            hex::encode(&data)
        );

        if id == 0 {
            match rlp::decode::<StatusMessage>(&data) {
                Ok(v) => {
                    info!("Decoded status message: {:?}", v);
                }
                Err(e) => {
                    info!("Failed to decode status message: {}! Kicking peer.", e);
                    return Ok((
                        None,
                        Some(ReputationReport::Kick {
                            reason: Some(DisconnectReason::ProtocolBreach),
                            ban: false,
                        }),
                    ));
                }
            }
        }

        let out_id = match id {
            3 => Some(4),
            5 => Some(6),
            _ => None,
        };

        Ok((
            out_id.map(|id| Message {
                id,
                data: rlp::encode_list::<String, String>(&[]).into(),
            }),
            None,
        ))
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let secret_key = SigningKey::random(&mut OsRng);

    let task_metrics = Arc::new(TaskMetrics::default());
    let task_group = Arc::new(TaskGroup::new_with_metrics(task_metrics.clone()));

    let mut dns_resolver = dnsdisc::Resolver::new(Arc::new(
        TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
            .await
            .unwrap(),
    ));
    dns_resolver.with_task_group(task_group.clone());

    let discovery = DnsDiscovery::new(Arc::new(dns_resolver), DNS_BOOTNODE.to_string(), None);

    let discovery: Arc<AsyncMutex<dyn Discovery>> = Arc::new(AsyncMutex::new(discovery));

    let client = RLPxNodeBuilder::new()
        .with_task_group(task_group.clone())
        .with_listen_options(ListenOptions {
            discovery_tasks: std::iter::repeat(discovery).take(1).collect(),
            max_peers: 50,
            addr: "0.0.0.0:30303".parse().unwrap(),
        })
        .build(secret_key)
        .await
        .unwrap();

    let _handle = client.register(
        CapabilityInfo {
            name: CapabilityName(ArrayString::from("eth").unwrap()),
            version: 63,
            length: 17,
        },
        Arc::new(CapabilityServerImpl),
    );

    loop {
        tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
        info!(
            "Peers: {}.",
            client.connected_peers(identity, None, None).len()
        );
    }
}
