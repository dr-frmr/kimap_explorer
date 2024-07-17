use alloy_sol_types::SolEvent;
use kinode_process_lib::{await_message, call_init, eth, http, kimap, println, Address, Message};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

wit_bindgen::generate!({
    path: "target/wit",
    world: "kimap-explorer-mothu-et-doria-dot-os-v0",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize],
});

#[derive(Deserialize, Serialize)]
enum ExplorerRequest {
    Tree,
    SubTree(String),
}

#[derive(Deserialize, Serialize)]
struct NamespaceEntry {
    pub parent_path: String,
    pub name: String,
    pub child_hashes: BTreeSet<String>,
    pub data_keys: BTreeMap<String, eth::Bytes>,
}

struct State {
    pub kimap: kimap::Kimap,
    // map from an entry hash to its information
    pub index: BTreeMap<String, NamespaceEntry>,
}

impl State {
    pub fn new() -> Self {
        Self {
            kimap: kimap::Kimap::default(60),
            index: BTreeMap::from([(
                kimap::KIMAP_ROOT_HASH.to_string(),
                NamespaceEntry {
                    parent_path: "".to_string(),
                    name: "".to_string(),
                    child_hashes: BTreeSet::new(),
                    data_keys: BTreeMap::new(),
                },
            )]),
        }
    }

    pub fn add_mint(
        &mut self,
        parent_hash: &str,
        child_hash: String,
        name: String,
    ) -> anyhow::Result<()> {
        let parent = self
            .index
            .get_mut(parent_hash)
            .ok_or(anyhow::anyhow!("parent not found"))?;

        parent.child_hashes.insert(child_hash.clone());

        let parent_path = parent.name.clone() + &parent.parent_path;

        self.index.insert(
            child_hash,
            NamespaceEntry {
                parent_path,
                name,
                child_hashes: BTreeSet::new(),
                data_keys: BTreeMap::new(),
            },
        );

        Ok(())
    }

    pub fn add_note(
        &mut self,
        parent_hash: &str,
        note: String,
        data: eth::Bytes,
    ) -> anyhow::Result<()> {
        let parent = self
            .index
            .get_mut(parent_hash)
            .ok_or(anyhow::anyhow!("parent not found"))?;

        parent.data_keys.insert(note, data);

        Ok(())
    }

    pub fn tree(&self, root_hash: &str, nest_level: usize) -> anyhow::Result<String> {
        let root = self
            .index
            .get(root_hash)
            .ok_or(anyhow::anyhow!("root not found"))?;

        Ok(format!(
            "{}{}{}{}",
            if root.name.is_empty() {
                ".".to_string()
            } else {
                format!("└─ {}", root.name)
            },
            if root.parent_path.is_empty() {
                "".to_string()
            } else {
                format!(".{}", root.parent_path)
            },
            if root.data_keys.is_empty() {
                "".to_string()
            } else {
                format!(
                    "\r\n{}",
                    root.data_keys
                        .iter()
                        .map(|(key, bytes)| format!(
                            "{}└─ {}: {} bytes",
                            " ".repeat(nest_level * 4),
                            key,
                            bytes.len()
                        ))
                        .collect::<Vec<String>>()
                        .join("\r\n")
                )
            },
            if root.child_hashes.is_empty() {
                "".to_string()
            } else {
                format!(
                    "\r\n{}",
                    root.child_hashes
                        .iter()
                        .map(|hash| format!(
                            "{}{}",
                            " ".repeat(nest_level * 4),
                            self.tree(hash, nest_level + 1).unwrap()
                        ))
                        .collect::<Vec<String>>()
                        .join("\r\n")
                )
            }
        ))
    }

    // for frontend, todo improve
    pub fn get_node_json(&self, hash: &str) -> anyhow::Result<serde_json::Value> {
        let node = self
            .index
            .get(hash)
            .ok_or(anyhow::anyhow!("Node not found"))?;
        Ok(serde_json::to_value(node)?)
    }
}

call_init!(init);
fn init(our: Address) {
    println!("online");

    let mut state = State::new();

    let filter = eth::Filter::new()
        .address(*state.kimap.address())
        .from_block(kimap::KIMAP_FIRST_BLOCK)
        .to_block(eth::BlockNumberOrTag::Latest)
        .events(vec![
            "Mint(bytes32,bytes32,bytes,bytes)",
            "Note(bytes32,bytes32,bytes,bytes,bytes)",
        ]);

    state.kimap.provider.subscribe_loop(1, filter.clone());

    loop {
        match state.kimap.provider.get_logs(&filter) {
            Ok(logs) => {
                for log in logs {
                    if let Err(e) = handle_log(&our, &mut state, &log) {
                        // print errors at verbosity=1
                        println!("log-handling error! {e:?}");
                    }
                }
                break;
            }
            Err(e) => {
                println!("got eth error while fetching logs: {e:?}, trying again in 5s...");
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        }
    }

    http::serve_ui(&our, "ui", false, false, vec!["/"]).unwrap();

    http::bind_http_path("/api/tree", false, false).unwrap();
    http::bind_http_path("/api/node/:hash", false, false).unwrap();
    http::bind_http_path("/api/info/:hash", false, false).unwrap();

    loop {
        match await_message() {
            Err(e) => {
                println!("error: {e}");
            }
            Ok(message) => {
                if let Err(e) = handle_message(&our, &mut state, message, &filter) {
                    println!("error: {e}");
                }
            }
        }
    }
}

fn handle_message(
    our: &Address,
    state: &mut State,
    message: Message,
    filter: &eth::Filter,
) -> anyhow::Result<()> {
    let Message::Request { source, body, .. } = message else {
        return Err(anyhow::anyhow!("unhandled message: {message:?}"));
    };

    if source.process == "eth:distro:sys" {
        match handle_eth_message(&our, state, &body) {
            Ok(_) => Ok(()),
            Err(e) => {
                state.kimap.provider.subscribe_loop(1, filter.clone());
                Err(e)
            }
        }
    } else if source.process == "http_server:distro:sys" {
        if let Ok(http::HttpServerRequest::Http(req)) = serde_json::from_slice(&body) {
            if req.path()? == "/api/tree" {
                let tree = state.get_node_json(&kimap::KIMAP_ROOT_HASH)?;
                let headers = HashMap::from([("Content-Type".into(), "application/json".into())]);
                http::send_response(
                    http::StatusCode::OK,
                    Some(headers),
                    tree.to_string().as_bytes().to_vec(),
                );
            } else if req.path()?.starts_with("/api/node/") {
                let hash = req.url_params().get("hash").unwrap();
                let node = state.get_node_json(&hash)?;
                let headers = HashMap::from([("Content-Type".into(), "application/json".into())]);
                http::send_response(
                    http::StatusCode::OK,
                    Some(headers),
                    node.to_string().as_bytes().to_vec(),
                );
            } else if req.path()?.starts_with("/api/info/") {
                let hash = req.url_params().get("hash").unwrap();
                let ret = state.kimap.get_hash(&hash);

                if let Ok((tba, owner, data)) = ret {
                    let info = serde_json::json!({
                        "tba": tba,
                        "owner": owner,
                        "data": data,
                    });

                    let headers =
                        HashMap::from([("Content-Type".into(), "application/json".into())]);

                    http::send_response(
                        http::StatusCode::OK,
                        Some(headers),
                        info.to_string().as_bytes().to_vec(),
                    );
                } else {
                    http::send_response(http::StatusCode::NOT_FOUND, None, vec![]);
                }
            } else {
                http::send_response(http::StatusCode::NOT_FOUND, None, vec![]);
            }
        }
        Ok(())
    } else {
        match serde_json::from_slice(&body) {
            Ok(ExplorerRequest::Tree) => {
                println!("tree:\r\n{}", state.tree(&kimap::KIMAP_ROOT_HASH, 0)?);
                Ok(())
            }
            Ok(ExplorerRequest::SubTree(root_hash)) => {
                println!("tree:\r\n{}", state.tree(&root_hash, 0)?);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!(
                "got invalid message from {}: {:?}, {e:?}",
                source,
                std::str::from_utf8(&body)
            )),
        }
    }
}

fn handle_eth_message(our: &Address, state: &mut State, body: &[u8]) -> anyhow::Result<()> {
    let Ok(eth_result) = serde_json::from_slice::<eth::EthSubResult>(body) else {
        return Err(anyhow::anyhow!("got invalid message"));
    };

    let eth::EthSub { result, .. } =
        eth_result.map_err(|e| anyhow::anyhow!("got eth subscription error: {e:?}"))?;

    if let eth::SubscriptionResult::Log(log) = result {
        handle_log(our, state, &log)
    } else {
        return Err(anyhow::anyhow!(
            "got unhandled eth subscription result: {result:?}"
        ));
    }
}

fn handle_log(_our: &Address, state: &mut State, log: &eth::Log) -> anyhow::Result<()> {
    match log.topics()[0] {
        kimap::contract::Mint::SIGNATURE_HASH => {
            let decoded = kimap::contract::Mint::decode_log_data(log.data(), true).unwrap();

            let parent_hash = decoded.parenthash.to_string();
            let child_hash = decoded.childhash.to_string();
            let label = String::from_utf8(decoded.name.to_vec())?;

            println!("got mint: {label}, parent_hash: {parent_hash}, child_hash: {child_hash}");
            match state.add_mint(&parent_hash, child_hash, label) {
                Ok(()) => println!("added entry to index"),
                Err(e) => println!("ERROR: {e}"),
            }
        }
        kimap::contract::Note::SIGNATURE_HASH => {
            let decoded = kimap::contract::Note::decode_log_data(log.data(), true).unwrap();

            let parent_hash = decoded.nodehash.to_string();
            let note = String::from_utf8(decoded.note.to_vec())?;

            println!("got note: {note}, node_hash: {parent_hash}",);
            match state.add_note(&parent_hash, note, decoded.data) {
                Ok(()) => println!("added note to index"),
                Err(e) => println!("ERROR: {e}"),
            }
        }
        _ => {}
    }

    Ok(())
}
