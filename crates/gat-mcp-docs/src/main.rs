use anyhow::{Context, Result};
use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    serve, Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
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
    version_roots: HashMap<String, PathBuf>,
    default_version: String,
    resource_texts: Vec<String>,
    search_index: HashMap<String, Vec<usize>>,
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

#[derive(Deserialize)]
struct DocRequestParams {
    version: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opt::parse();
    let canonical_root = fs::canonicalize(&opts.docs)?;
    let (resources, resource_texts) = collect_resources(&opts.docs)?;
    let index = load_doc_index(&opts.docs)?;
    let mut version_roots = HashMap::new();
    for version in &index.versions {
        let base = base_path_for_uri(&version.uri);
        version_roots.insert(version.name.clone(), base);
    }
    if !version_roots.contains_key(&index.default) {
        version_roots.insert(index.default.clone(), PathBuf::new());
    }
    let search_index = build_search_index(&resource_texts);
    let state = Arc::new(AppState {
        docs_root: opts.docs,
        canonical_root,
        resources,
        resource_texts,
        search_index,
        version_roots,
        default_version: index.default.clone(),
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

async fn get_doc(
    AxumPath(path): AxumPath<String>,
    Query(params): Query<DocRequestParams>,
    Extension(state): Extension<Arc<AppState>>,
) -> Response {
    let rel = StdPath::new(&path);
    let version = params.version.as_deref();
    let base = resolve_version_root(&state, version);
    let target = state.docs_root.join(base).join(rel);
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
    let query = params.q.split_whitespace().collect::<Vec<_>>().join(" ");
    let query_lower = query.to_lowercase();
    let tokens = tokenize(&query_lower);
    let mut scores: HashMap<usize, usize> = HashMap::new();
    for token in &tokens {
        if let Some(entries) = state.search_index.get(token) {
            for idx in entries {
                *scores.entry(*idx).or_insert(0) += 1;
            }
        }
    }
    let mut hits: Vec<(usize, usize)> = scores.into_iter().collect();
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.truncate(10);
    let results = hits
        .into_iter()
        .map(|(idx, _)| {
            let path = state.resources[idx].path.clone();
            let snippet = build_snippet(&state.resource_texts[idx], &query_lower);
            SearchHit { path, snippet }
        })
        .collect();
    Json(results)
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

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn build_search_index(texts: &[String]) -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, text) in texts.iter().enumerate() {
        for token in tokenize(&text.to_lowercase()) {
            index.entry(token).or_default().push(idx);
        }
    }
    index
}

fn build_snippet(text: &str, query: &str) -> String {
    let lower = text.to_lowercase();
    if let Some(idx) = lower.find(query) {
        return highlight_snippet(text, idx, query.len());
    }
    for token in tokenize(query) {
        if let Some(idx) = lower.find(&token) {
            return highlight_snippet(text, idx, token.len());
        }
    }
    text.lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(120)
        .collect()
}

fn highlight_snippet(text: &str, idx: usize, len: usize) -> String {
    let context = 60;
    let start = clamp_backward(text, idx.saturating_sub(context));
    let end = clamp_forward(text, (idx + len + context).min(text.len()));
    let snippet = &text[start..end];
    let rel_start = idx - start;
    let rel_end = rel_start + len;
    let mut highlighted = String::new();
    if start > 0 {
        highlighted.push('…');
    }
    highlighted.push_str(&snippet[..rel_start]);
    highlighted.push_str("**");
    highlighted.push_str(&snippet[rel_start..rel_end]);
    highlighted.push_str("**");
    highlighted.push_str(&snippet[rel_end..]);
    if end < text.len() {
        highlighted.push('…');
    }
    highlighted
}

fn clamp_backward(text: &str, mut pos: usize) -> usize {
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

fn clamp_forward(text: &str, mut pos: usize) -> usize {
    while pos < text.len() && !text.is_char_boundary(pos) {
        pos += 1;
    }
    pos.min(text.len())
}

fn load_doc_index(root: &StdPath) -> Result<DocIndex> {
    let path = root.join("index.json");
    let file = File::open(&path).context("opening docs index")?;
    let index: DocIndex = serde_json::from_reader(file).context("parsing docs/index.json")?;
    Ok(index)
}

fn base_path_for_uri(uri: &str) -> PathBuf {
    let trimmed = uri
        .strip_prefix("/doc/")
        .unwrap_or(uri.trim_start_matches('/'));
    let parent = StdPath::new(trimmed)
        .parent()
        .unwrap_or_else(|| StdPath::new(""));
    parent.to_path_buf()
}

fn resolve_version_root(state: &AppState, version: Option<&str>) -> PathBuf {
    let name = version.unwrap_or(&state.default_version);
    state
        .version_roots
        .get(name)
        .unwrap_or_else(|| state.version_roots.get(&state.default_version).unwrap())
        .clone()
}

fn collect_resources(root: &StdPath) -> anyhow::Result<(Vec<Resource>, Vec<String>)> {
    let mut entries = Vec::new();
    let mut texts = Vec::new();
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
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            entries.push(Resource {
                path: path.clone(),
                uri: format!("/doc/{path}"),
                kind: kind.to_string(),
            });
            texts.push(text);
        }
    }
    Ok((entries, texts))
}

#[derive(Deserialize)]
struct DocIndex {
    default: String,
    versions: Vec<DocVersion>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DocVersion {
    name: String,
    uri: String,
    generated_at: String,
}
