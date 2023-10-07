use crate::logic::SendItem;
use crate::slint_generatedAppWindow::{AppWindow, Logic};
use crate::util::translator::tr;
use crate::{chat, config, SendCB};
use anyhow::{anyhow, Result};
use futures::{future::Either, stream::StreamExt};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::OrTransport, upgrade},
    gossipsub, identity, mdns, noise, quic,
    swarm::NetworkBehaviour,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux, PeerId, Transport,
};
use log::{debug, warn};
use slint::{ComponentHandle, Weak};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tokio::{select, sync::mpsc, task, time::Duration};

#[derive(NetworkBehaviour)]
struct CBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub fn init(ui: Weak<AppWindow>, cb: SendCB) -> mpsc::UnboundedSender<String> {
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let tx2 = tx.clone();
    task::spawn(async move {
        if let Err(e) = start_gossipsub(tx2, rx, config::net(), ui, cb).await {
            warn!("{:?}", e);
        }
    });

    tx
}

async fn start_gossipsub(
    tx: mpsc::UnboundedSender<String>,
    mut rx: mpsc::UnboundedReceiver<String>,
    topic: String,
    ui: Weak<AppWindow>,
    cb: SendCB,
) -> Result<()> {
    let swarm_conf = config::swarm();
    let id_keys = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(id_keys.public());
    let lp_id = local_peer_id.to_string();

    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&id_keys).map_err(|e| {
            anyhow!(format!(
                "signing libp2p-noise static keypair. Error: {:?}",
                e
            ))
        })?)
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(swarm_conf.connect_timeout))
        .boxed();
    let quic_transport = quic::tokio::Transport::new(quic::Config::new(&id_keys));
    let transport = OrTransport::new(quic_transport, tcp_transport)
        .map(|either_output, _| match either_output {
            Either::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            Either::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
        })
        .boxed();

    // content-address messages. No two messages of the same content will be propagated.
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(swarm_conf.keepalive_interval))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|e| anyhow!(format!("Valid config. Erroe: {:?}", e)))?;

    let mut gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(id_keys),
        gossipsub_config,
    )
    .map_err(|e| anyhow!(format!("Incorrect configuration, Error: {:?}", e)))?;

    let topic = gossipsub::IdentTopic::new(topic);
    gossipsub.subscribe(&topic)?;

    let mut swarm = {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let behaviour = CBehaviour { gossipsub, mdns };
        SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build()
    };

    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        select! {
            Some(msg) = rx.recv() => {
                debug!("Send message: {}", msg);

                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(topic.clone(), msg.as_bytes()) {
                    let (ui, estr) = (ui.clone(), e.to_string());
                    let _ = slint::invoke_from_event_loop(move || {
                        let sitem = SendItem::from(msg.as_str());
                        if sitem.r#type != "ping" {
                            ui.unwrap().global::<Logic>()
                                .invoke_show_message(slint::format!("{}. {}: {:?}", tr("发送失败"), tr("原因"), estr), "warning".into());
                        }
                    });
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(CBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    let mut peer_id_cache = HashSet::new();
                    for (peer_id, _multiaddr) in list {
                        debug!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                        let (ui, tx) = (ui.clone(), tx.clone());
                        let peer_id = peer_id.to_string();
                        if peer_id_cache.contains(&peer_id) {
                            continue;
                        }
                        peer_id_cache.insert(peer_id.clone());

                        let _ = slint::invoke_from_event_loop(move || {
                            let ui = ui.unwrap();
                            chat::send_handshake_request(&ui, tx, peer_id);
                        });
                    }
                },
                SwarmEvent::Behaviour(CBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        debug!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(CBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) =>  {
                    let msg = String::from_utf8_lossy(&message.data).to_string();
                    debug!(
                            "Got message: '{}' with id: {id} from peer: {peer_id}",
                           msg
                        );

                    let (ui, tx) = (ui.clone(), tx.clone());
                    let local_peer_id = lp_id.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        cb(ui,tx, msg, local_peer_id);
                    });
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    debug!("Local node is listening on {address}");
                }
                _ => {}
            }
        }
    }
}
