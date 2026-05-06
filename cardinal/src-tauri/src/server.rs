use crate::commands::{NodeInfoMetadata, NodeInfoRequest, SearchJob, SearchOptionsPayload};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use crossbeam_channel::{Sender, bounded};
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct ServerState {
    pub search_tx: Sender<SearchJob>,
    pub node_info_tx: Sender<NodeInfoRequest>,
}

impl ServerState {
    pub fn new(search_tx: Sender<SearchJob>, node_info_tx: Sender<NodeInfoRequest>) -> Self {
        Self {
            search_tx,
            node_info_tx,
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

pub async fn start_server(state: ServerState, port: u16) {
    let app = Router::new()
        .route("/search", get(search_handler_get))
        .route("/search", post(search_handler))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    if let Ok(listener) = TcpListener::bind(addr).await {
        tracing::info!("Starting axum HTTP server on {}", addr);
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Server error: {}", e);
        }
    }
}

async fn search_handler_get(
    State(state): State<Arc<ServerState>>,
    Query(req): Query<SearchRequest>,
) -> impl IntoResponse {
    handle_search(state, req).await
}

async fn search_handler(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    handle_search(state, req).await
}

async fn handle_search(state: Arc<ServerState>, req: SearchRequest) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let (slab_indices, highlights) = {
            let cancellation_token = CancellationToken::new_search();

            let (result_tx, result_rx) = bounded(1);
            if let Err(e) = state.search_tx.send(SearchJob {
                query: req.query,
                options: req.options,
                cancellation_token,
                result_tx,
            }) {
                tracing::error!("Failed to send search job: {:?}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            let outcome = match result_rx.recv() {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(e)) => {
                    tracing::error!("Search error: {:?}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
                Err(e) => {
                    tracing::error!("Reply channel closed: {:?}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            };

            let mut fetched_indices = outcome.nodes.unwrap_or_default();

            if let Some(sort) = &req.sort {
                if fetched_indices.len() > 10000 {
                    fetched_indices.truncate(10000);
                }
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
                        .map(|(slab_index, node)| crate::sort::SortEntry::new(slab_index, node))
                        .collect();
                    crate::sort::sort_entries(&mut entries, sort);
                    fetched_indices = entries.into_iter().map(|e| e.slab_index).collect();
                }
            }

            (fetched_indices, outcome.highlights)
        };

        let mut results = slab_indices;

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
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        let final_nodes = match node_info_reply_rx.recv() {
            Ok(nodes) => nodes,
            Err(e) => {
                tracing::error!("Node info reply channel closed: {:?}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        let final_results: Vec<_> = final_nodes
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

        Ok(Json(ServerSearchResponse {
            results: final_results,
            highlights,
        }))
    })
    .await;

    match result {
        Ok(Ok(response)) => response.into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
