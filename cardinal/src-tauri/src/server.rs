use crate::{
    commands::{NodeInfoMetadata, NodeInfoRequest, SearchJob, SearchOptionsPayload},
    sort::SortEntry,
};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use rouille::Response;
use search_cache::SearchOutcome;
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::ToSocketAddrs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

static SEARCH_VERSION: AtomicU64 = AtomicU64::new(1000000);
static CACHE_TIMEOUT: u64 = 20;

struct CacheEntry {
    results: Vec<search_cache::SlabIndex>,
    highlights: Vec<String>,
    timestamp: Instant,
}

#[derive(Clone)]
pub struct ServerState {
    pub search_tx: Sender<SearchJob>,
    pub result_rx: Receiver<Result<SearchOutcome>>,
    pub node_info_tx: Sender<NodeInfoRequest>,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    search_lock: Arc<Mutex<()>>,
}

impl ServerState {
    pub fn new(
        search_tx: Sender<SearchJob>,
        result_rx: Receiver<Result<SearchOutcome>>,
        node_info_tx: Sender<NodeInfoRequest>,
    ) -> Self {
        Self {
            search_tx,
            result_rx,
            node_info_tx,
            cache: Arc::new(Mutex::new(HashMap::new())),
            search_lock: Arc::new(Mutex::new(())),
        }
    }
}

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub options: SearchOptionsPayload,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub sort: Option<crate::sort::SortStatePayload>,
}

#[derive(Clone, Serialize)]
pub struct ServerNodeInfo {
    pub path: String,
    pub metadata: Option<NodeInfoMetadata>,
    pub icon: Option<String>,
}

#[derive(Serialize)]
pub struct ServerSearchResponse {
    pub results: Vec<ServerNodeInfo>,
    pub highlights: Vec<String>,
}

pub fn start_server(state: ServerState, addr: impl ToSocketAddrs) {
    let state = Arc::new(state);
    rouille::start_server(addr, move |request| {
        rouille::log(request, std::io::stdout(), || {
            rouille::router!(request,
                (GET) (/search) => {
                    let query = request.get_param("query").unwrap_or_default();
                    let limit = request.get_param("limit").and_then(|v| v.parse().ok());
                    let offset = request.get_param("offset").and_then(|v| v.parse().ok());
                    let req = SearchRequest {
                        query,
                        options: SearchOptionsPayload::default(),
                        limit,
                        offset,
                        sort: None,
                    };
                    handle_search(state.clone(), req)
                },
                (POST) (/search) => {
                    let req: SearchRequest = match rouille::input::json_input(request) {
                        Ok(r) => r,
                        Err(_) => return Response::text("Invalid JSON").with_status_code(400),
                    };
                    handle_search(state.clone(), req)
                },
                _ => Response::empty_404()
            )
        })
    });
}

fn handle_search(state: Arc<ServerState>, req: SearchRequest) -> Response {
    let cache_key = format!(
        "{}_{}_{}",
        req.query,
        req.options.case_insensitive,
        serde_json::to_string(&req.sort).unwrap_or_default()
    );

    let mut cached_results = None;
    let mut cached_highlights = Vec::new();

    {
        let mut cache = state.cache.lock().unwrap();
        cache.retain(|_, entry| entry.timestamp.elapsed() < Duration::from_secs(CACHE_TIMEOUT));

        if let Some(entry) = cache.get(&cache_key) {
            cached_results = Some(entry.results.clone());
            cached_highlights = entry.highlights.clone();
        }
    }

    let (slab_indices, highlights) = if let Some(results) = cached_results {
        (results, cached_highlights)
    } else {
        // Clear any stale results
        // TODO: better logic?
        let _lock = state.search_lock.lock().unwrap();
        while let Ok(_) = state.result_rx.try_recv() {}

        let version = SEARCH_VERSION.fetch_add(1, Ordering::Relaxed);
        let cancellation_token = CancellationToken::new(version);

        if let Err(e) = state.search_tx.send(SearchJob {
            query: req.query,
            options: req.options,
            cancellation_token,
        }) {
            tracing::error!("Failed to send search job: {:?}", e);
            return Response::text("Internal Server Error").with_status_code(500);
        }

        let outcome = match state.result_rx.recv() {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(e)) => {
                tracing::error!("Search error: {:?}", e);
                return Response::text("Internal Server Error").with_status_code(500);
            }
            Err(e) => {
                tracing::error!("Reply channel closed: {:?}", e);
                return Response::text("Internal Server Error").with_status_code(500);
            }
        };

        let mut fetched_indices = outcome.nodes.unwrap_or_default();

        if let Some(sort) = &req.sort {
            let (node_info_reply_tx, node_info_reply_rx) = bounded(1);
            if let Err(e) = state.node_info_tx.send(NodeInfoRequest {
                slab_indices: fetched_indices.clone(),
                response_tx: node_info_reply_tx,
            }) {
                tracing::error!("Failed to send node info request for sorting: {:?}", e);
            } else if let Ok(all_nodes) = node_info_reply_rx.recv() {
                let mut entries: Vec<_> = fetched_indices
                    .into_iter()
                    .zip(all_nodes)
                    .map(|(slab_index, node)| SortEntry::new(slab_index, node))
                    .collect();
                crate::sort::sort_entries(&mut entries, sort);
                fetched_indices = entries.into_iter().map(|e| e.slab_index).collect();
            }
        }

        {
            let mut cache = state.cache.lock().unwrap();
            cache.insert(
                cache_key,
                CacheEntry {
                    results: fetched_indices.clone(),
                    highlights: outcome.highlights.clone(),
                    timestamp: Instant::now(),
                },
            );
        }

        (fetched_indices, outcome.highlights)
    };

    let mut results: Vec<search_cache::SlabIndex> = slab_indices;

    if let Some(offset) = req.offset {
        if offset < results.len() {
            results = results[offset..].to_vec();
        } else {
            results.clear();
        }
    }
    if let Some(limit) = req.limit {
        results.truncate(limit);
    }

    let (node_info_reply_tx, node_info_reply_rx) = bounded(1);
    if let Err(e) = state.node_info_tx.send(NodeInfoRequest {
        slab_indices: results,
        response_tx: node_info_reply_tx,
    }) {
        tracing::error!("Failed to send node info request: {:?}", e);
        return Response::text("Internal Server Error").with_status_code(500);
    }

    let final_nodes = match node_info_reply_rx.recv() {
        Ok(nodes) => nodes,
        Err(e) => {
            tracing::error!("Node info reply channel closed: {:?}", e);
            return Response::text("Internal Server Error").with_status_code(500);
        }
    };

    let final_results: Vec<ServerNodeInfo> = final_nodes
        .into_iter()
        .map(|node| {
            let path = node.path.to_string_lossy().into_owned();
            ServerNodeInfo {
                path,
                metadata: node.metadata.as_ref().map(NodeInfoMetadata::from_metadata),
                icon: None,
            }
        })
        .collect();

    Response::json(&ServerSearchResponse {
        results: final_results,
        highlights,
    })
}
