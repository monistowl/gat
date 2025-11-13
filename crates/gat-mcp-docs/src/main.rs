use anyhow::Result;
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    serve, Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    path::{Path as StdPath, PathBuf},
    sync::Arc,
};
use tokio::net::TcpListener;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about = "Serve GAT docs over HTTP/MCP", long_about = None)]
struct Opt {
    /// Root docs directory
    #[arg(long, default_value = "docs")]
    docs: PathBuf,
    /// Address to bind the MCP server
    #[arg(long, default_value = "127.0.0.1:4321")]
    addr: SocketAddr,
}

#[derive(Clone)]
struct AppState {
    docs_root: PathBuf,
    canonical_root: PathBuf,
    resources: Vec<Resource>,
}

#[derive(Serialize, Clone)]
struct Resource {
    path: String,
    uri: String,
    kind: String,
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
}

#[derive(Serialize)]
struct SearchHit {
    path: String,
    snippet: String,
}

#[derive(Deserialize)]
struct ExplainParams {
    command: String,
}

#[derive(Serialize)]
struct ExplainResult {
    path: String,
    excerpt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opt::parse();
    let canonical_root = fs::canonicalize(&opts.docs)?;
    let resources = collect_resources(&opts.docs)?;
    let state = Arc::new(AppState {
        docs_root: opts.docs,
        canonical_root,
        resources,
    });

    let app = Router::new()
        .route("/resources", get(list_resources))
        .route("/doc/*path", get(get_doc))
        .route("/search", get(search_docs))
        .route("/explain", get(explain_command))
        .layer(Extension(state));

    println!("Serving docs at {}", opts.addr);
    let listener = TcpListener::bind(opts.addr).await?;
    serve(listener, app).await?;

    Ok(())
}

async fn list_resources(Extension(state): Extension<Arc<AppState>>) -> Json<Vec<Resource>> {
    Json(state.resources.clone())
}

async fn get_doc(Path(path): Path<String>, Extension(state): Extension<Arc<AppState>>) -> Response {
    let rel = StdPath::new(&path);
    let target = state.docs_root.join(rel);
    match fs::canonicalize(&target) {
        Ok(canonical) => {
            if !canonical.starts_with(&state.canonical_root) {
                return (StatusCode::FORBIDDEN, "invalid path").into_response();
            }
            match fs::read(&canonical) {
                Ok(bytes) => {
                    let mime = mime_guess::from_path(&canonical).first_or_octet_stream();
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", mime.as_ref())
                        .body(bytes.into())
                        .unwrap()
                }
                Err(_) => (StatusCode::NOT_FOUND, "file not readable").into_response(),
            }
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn search_docs(
    Query(params): Query<SearchParams>,
    Extension(state): Extension<Arc<AppState>>,
) -> Json<Vec<SearchHit>> {
    let query = params.q.to_lowercase();
    let mut hits = Vec::new();
    for resource in state.resources.iter() {
        if hits.len() >= 10 {
            break;
        }
        if resource.path.to_lowercase().contains(&query) {
            hits.push(SearchHit {
                path: resource.path.clone(),
                snippet: "matched path".into(),
            });
            continue;
        }
        let file_path = state.docs_root.join(&resource.path);
        if let Ok(text) = fs::read_to_string(&file_path) {
            if let Some(idx) = text.to_lowercase().find(&query) {
                let snippet = text[idx..]
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(120)
                    .collect();
                hits.push(SearchHit {
                    path: resource.path.clone(),
                    snippet,
                });
            }
        }
    }
    Json(hits)
}

async fn explain_command(
    Query(params): Query<ExplainParams>,
    Extension(state): Extension<Arc<AppState>>,
) -> Json<Option<ExplainResult>> {
    let query = params.command.to_lowercase();
    for resource in state.resources.iter() {
        if resource.path.contains("cli") && resource.path.to_lowercase().contains(&query) {
            if let Ok(text) = fs::read_to_string(state.docs_root.join(&resource.path)) {
                let excerpt = text.lines().take(6).collect::<Vec<_>>().join("\n");
                return Json(Some(ExplainResult {
                    path: resource.path.clone(),
                    excerpt,
                }));
            }
        }
    }
    Json(None)
}

fn collect_resources(root: &StdPath) -> anyhow::Result<Vec<Resource>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(root).unwrap();
            let path = rel.to_string_lossy().replace('\\', "/");
            let kind = match entry.path().extension().and_then(|s| s.to_str()) {
                Some("md") => "markdown",
                Some("json") => "json",
                Some("1") => "manpage",
                _ => "file",
            };
            entries.push(Resource {
                path: path.clone(),
                uri: format!("/doc/{path}"),
                kind: kind.to_string(),
            });
        }
    }
    Ok(entries)
}
