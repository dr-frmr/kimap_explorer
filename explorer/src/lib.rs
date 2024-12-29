use alloy_sol_types::SolEvent;
use kinode::process::kimap_explorer::{Name, Namehash, Request as ExplorerRequest};
use kinode_app_framework::{app, eth, http, kimap, println, Address, Message};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

wit_bindgen::generate!({
    path: "target/wit",
    world: "kimap-explorer-doria-dot-kino-v1",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize, kinode_app_framework::SerdeJsonInto],
});

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum Req {
    ExplorerRequest(ExplorerRequest),
    Eth(eth::EthSubResult),
}

#[derive(Debug, Deserialize, Serialize)]
enum HttpApi {
    /// fetch node info indexed within this app
    GetNode(Name),
    /// fetch onchain tba and owner for a node
    GetTba(Name),
}

#[derive(Debug, Deserialize, Serialize)]
enum DataKey {
    /// facts are immutable
    Fact(eth::Bytes),
    /// notes are mutable: we store all versions of the note, most recent last
    /// if indexing full history, this will be the note's full history --
    /// it is also possible to receive a snapshot and not have updates from before that.
    Note(Vec<eth::Bytes>),
}

#[derive(Debug, Deserialize, Serialize)]
struct Node {
    /// everything that comes before a name, from root, with dots separating and a leading dot
    pub parent_path: String,
    /// the name of the node -- a string.
    pub name: Name,
    /// the children of the node
    pub child_names: BTreeSet<Name>,
    /// the node's data keys
    pub data_keys: BTreeMap<String, DataKey>,
}

#[derive(Debug, Deserialize, Serialize)]
struct State {
    pub kimap: kimap::Kimap,
    /// lookup table from name to namehash
    pub names: HashMap<Name, Namehash>,
    /// map from a namehash to its information
    pub index: BTreeMap<String, Node>,
}

impl kinode_app_framework::State for State {
    /// generate a new state and subscribe to the kimap and catch up on logs from genesis
    fn new() -> Self {
        let kimap = kimap::Kimap::default(60);

        kimap
            .provider
            .subscribe_loop(1, Self::make_filter(&kimap), 0, 0);

        let mut new_state = Self {
            kimap: kimap.clone(),
            names: HashMap::from([(String::new(), kimap::KIMAP_ROOT_HASH.to_string())]),
            index: BTreeMap::from([(
                kimap::KIMAP_ROOT_HASH.to_string(),
                Node {
                    parent_path: String::new(),
                    name: String::new(),
                    child_names: BTreeSet::new(),
                    data_keys: BTreeMap::new(),
                },
            )]),
        };

        loop {
            match kimap.provider.get_logs(&Self::make_filter(&kimap)) {
                Ok(logs) => {
                    for log in logs {
                        if let Err(e) = new_state.handle_log(&log) {
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

        new_state
    }
}

impl State {
    pub fn make_filter(kimap: &kimap::Kimap) -> eth::Filter {
        eth::Filter::new()
            .address(*kimap.address())
            .from_block(kimap::KIMAP_FIRST_BLOCK)
            .to_block(eth::BlockNumberOrTag::Latest)
            .events(vec![
                kimap::contract::Mint::SIGNATURE,
                kimap::contract::Note::SIGNATURE,
                kimap::contract::Fact::SIGNATURE,
            ])
    }

    pub fn handle_log(&mut self, log: &eth::Log) -> anyhow::Result<()> {
        match log.topics()[0] {
            kimap::contract::Mint::SIGNATURE_HASH => {
                let decoded = kimap::contract::Mint::decode_log_data(log.data(), true).unwrap();

                let parent_hash = decoded.parenthash.to_string();
                let child_hash = decoded.childhash.to_string();
                let label = String::from_utf8(decoded.label.to_vec())?;

                self.add_mint(&parent_hash, child_hash, label)?;
            }
            kimap::contract::Note::SIGNATURE_HASH => {
                let decoded = kimap::contract::Note::decode_log_data(log.data(), true).unwrap();

                let parent_hash = decoded.parenthash.to_string();
                let note_label = String::from_utf8(decoded.label.to_vec())?;

                self.add_note(&parent_hash, note_label, decoded.data)?;
            }
            kimap::contract::Fact::SIGNATURE_HASH => {
                let decoded = kimap::contract::Fact::decode_log_data(log.data(), true).unwrap();

                let parent_hash = decoded.parenthash.to_string();
                let fact_label = String::from_utf8(decoded.label.to_vec())?;

                self.add_fact(&parent_hash, fact_label, decoded.data)?;
            }
            _ => {}
        }

        Ok(())
    }

    pub fn add_mint(
        &mut self,
        parent_hash: &str,
        child_hash: Namehash,
        name: Name,
    ) -> anyhow::Result<()> {
        let parent_node: &mut Node = self
            .index
            .get_mut(parent_hash)
            .ok_or(anyhow::anyhow!("parent for child {child_hash} not found!"))?;

        let parent_path: String = if parent_hash == kimap::KIMAP_ROOT_HASH {
            String::new()
        } else if parent_node.parent_path.is_empty() {
            format!(".{}", parent_node.name)
        } else {
            format!(".{}{}", parent_node.name, parent_node.parent_path)
        };

        let full_name = format!("{}{}", name, parent_path);

        parent_node.child_names.insert(full_name.clone());
        self.names.insert(full_name, child_hash.clone());
        self.index.insert(
            child_hash,
            Node {
                parent_path,
                name,
                child_names: BTreeSet::new(),
                data_keys: BTreeMap::new(),
            },
        );

        Ok(())
    }

    pub fn add_note(
        &mut self,
        parent_hash: &str,
        note_label: String,
        data: eth::Bytes,
    ) -> anyhow::Result<()> {
        let parent: &mut Node = self.index.get_mut(parent_hash).ok_or(anyhow::anyhow!(
            "parent {parent_hash} not found for note {note_label}"
        ))?;

        match parent.data_keys.entry(note_label) {
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(DataKey::Note(vec![data]));
            }
            std::collections::btree_map::Entry::Occupied(mut e) => {
                if let DataKey::Note(ref mut notes) = e.get_mut() {
                    notes.push(data);
                }
            }
        }

        Ok(())
    }

    pub fn add_fact(
        &mut self,
        parent_hash: &str,
        fact_label: String,
        data: eth::Bytes,
    ) -> anyhow::Result<()> {
        let parent: &mut Node = self.index.get_mut(parent_hash).ok_or(anyhow::anyhow!(
            "parent {parent_hash} not found for fact {fact_label}"
        ))?;

        // this should never ever happen
        if parent.data_keys.contains_key(&fact_label) {
            return Err(anyhow::anyhow!(
                "fact {fact_label} already exists on parent {parent_hash}"
            ));
        }

        parent.data_keys.insert(fact_label, DataKey::Fact(data));

        Ok(())
    }

    pub fn tree(&self, root_hash: &str, nest_level: usize) -> String {
        let Some(root) = self.index.get(root_hash) else {
            return String::new();
        };

        format!(
            "{}{}{}{}",
            if root.name.is_empty() {
                ".".to_string()
            } else {
                format!("└─ {}", root.name)
            },
            if root.parent_path.is_empty() {
                String::new()
            } else {
                root.parent_path.to_string()
            },
            if root.data_keys.is_empty() {
                String::new()
            } else {
                format!(
                    "\r\n{}",
                    root.data_keys
                        .iter()
                        .map(|(label, data_key)| format!(
                            "{}└─ {}: {} bytes",
                            " ".repeat(nest_level * 4),
                            label,
                            match data_key {
                                // note will never have an empty vector
                                DataKey::Note(notes) => notes.last().unwrap().len(),
                                DataKey::Fact(bytes) => bytes.len(),
                            }
                        ))
                        .collect::<Vec<String>>()
                        .join("\r\n")
                )
            },
            if root.child_names.is_empty() {
                String::new()
            } else {
                format!(
                    "\r\n{}",
                    root.child_names
                        .iter()
                        .map(|name| if let Some(namehash) = self.names.get(name) {
                            format!(
                                "{}{}",
                                " ".repeat(nest_level * 4),
                                self.tree(namehash, nest_level + 1)
                            )
                        } else {
                            String::new()
                        })
                        .collect::<Vec<String>>()
                        .join("\r\n")
                )
            }
        )
    }
}

app!(
    "Kimap Explorer",
    None,
    None,
    http_handler,
    local_request_handler,
    remote_request_handler
);

fn http_handler(state: &mut State, call: HttpApi) -> (http::server::HttpResponse, Vec<u8>) {
    match call {
        HttpApi::GetNode(name) => {
            let Some(namehash) = state.names.get(&name) else {
                return (
                    http::server::HttpResponse::new(http::StatusCode::NOT_FOUND),
                    "name not found".as_bytes().to_vec(),
                );
            };
            let Some(node) = state.index.get(namehash) else {
                return (
                    http::server::HttpResponse::new(http::StatusCode::NOT_FOUND),
                    "namehash not found".as_bytes().to_vec(),
                );
            };
            (
                http::server::HttpResponse::new(http::StatusCode::OK),
                serde_json::to_vec(&node).expect("failed to serialize node"),
            )
        }
        HttpApi::GetTba(name) => {
            let Ok((tba, owner, data)) = state.kimap.get(&name) else {
                return (
                    http::server::HttpResponse::new(http::StatusCode::NOT_FOUND),
                    vec![],
                );
            };
            let info = serde_json::json!({
                "tba": tba,
                "owner": owner,
                "data": data,
            });
            (
                http::server::HttpResponse::new(http::StatusCode::OK),
                info.to_string().as_bytes().to_vec(),
            )
        }
    }
}

fn local_request_handler(
    _message: &Message,
    state: &mut State,
    _server: &mut http::server::HttpServer,
    request: Req,
) {
    match request {
        Req::ExplorerRequest(request) => match request {
            ExplorerRequest::Tree(name) => {
                let Some(namehash) = state.names.get(&name) else {
                    println!("name not found");
                    return;
                };
                println!("\r\n{}", state.tree(&namehash, 0));
            }
            ExplorerRequest::TreeFromNamehash(namehash) => {
                println!("\r\n{}", state.tree(&namehash, 0));
            }
        },
        Req::Eth(eth_result) => {
            let Ok(eth::EthSub { result, .. }) = eth_result else {
                return;
            };

            let Ok(sub_result) = serde_json::from_value::<eth::SubscriptionResult>(result) else {
                println!("got really weird eth result");
                return;
            };

            if let eth::SubscriptionResult::Log(log) = sub_result {
                match state.handle_log(&log) {
                    Ok(()) => return,
                    Err(e) => println!("log-handling error! {e:?}"),
                }
            } else {
                println!("got unhandled eth subscription result: {sub_result:?}");
            }
        }
    }
}

/// won't get these since manifest declares networking false.
fn remote_request_handler(
    _message: &Message,
    _state: &mut State,
    _server: &mut http::server::HttpServer,
    _request: Req,
) {
    return;
}
