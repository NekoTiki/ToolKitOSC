//! Signing in to the share server. The JWT for each server lives in the system password store (see
//! `secrets`) and never reaches the webview: Rust signs the `/host` link in with it, adds it to the
//! AI requests, and hands the webview only the user it names. `auth.json` lists which servers have
//! one, since the store can't be listed.
//!
//! Signing in runs here too: the server's `/auth/desktop/ws` socket sends a browser link
//! (`auth-uri`), then the token once the user has signed in there.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
use tokio::time::{interval_at, sleep, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;

use crate::secrets;
use crate::state::AppState;

const FILE: &str = "auth.json";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const PING_INTERVAL: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct Auth {
    inner: Mutex<Inner>
}

#[derive(Default)]
struct Inner {
    loaded: bool,
    /// Server URL -> JWT.
    tokens: BTreeMap<String, String>,
    /// The sign-in running now: its server, its stop switch and the browser link it got.
    sign_in: Option<SignIn>
}

struct SignIn {
    url: String,
    stop: Option<oneshot::Sender<()>>,
    uri: Option<String>
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AuthFile {
    #[serde(default)]
    servers: BTreeSet<String>
}

/// What the webview gets: for each server with a token, the user it names (`null` if it can't be
/// read), plus where tokens are kept.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// False until the tokens have been read at startup.
    loaded: bool,
    users: BTreeMap<String, Value>,
    secret_store: secrets::Backend
}

fn auth(app: &AppHandle) -> &Auth {
    &app.state::<AppState>().inner().server.auth
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

fn secret_key(url: &str) -> String {
    format!("server-token:{url}")
}

fn save_index(app: &AppHandle, servers: BTreeSet<String>) {
    let result = (|| -> Result<(), String> {
        let path = path(app).ok_or("no app data folder")?;
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&AuthFile { servers }).map_err(|e| e.to_string())?;
        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save {FILE}: {err}");
    }
}

/// At startup, off the main thread since the password store can be slow: reads every token, then
/// signs the server link in if it connected before they were ready.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        load(&app);
        emit(&app);
        super::refresh(&app);
    });
}

/// Blocking.
fn load(app: &AppHandle) {
    if auth(app).inner.lock().unwrap().loaded {
        return;
    }
    let servers = path(app)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|data| serde_json::from_str::<AuthFile>(&data).ok())
        .unwrap_or_default()
        .servers;
    let tokens: BTreeMap<String, String> =
        servers.into_iter().filter_map(|url| secrets::get(app, &secret_key(&url)).map(|token| (url, token))).collect();

    let mut inner = auth(app).inner.lock().unwrap();
    if !inner.loaded {
        inner.loaded = true;
        inner.tokens = tokens;
    }
}

pub fn token(app: &AppHandle, url: &str) -> Option<String> {
    auth(app).inner.lock().unwrap().tokens.get(url).cloned()
}

pub fn view(app: &AppHandle) -> View {
    let inner = auth(app).inner.lock().unwrap();
    View {
        loaded: inner.loaded,
        users: inner.tokens.iter().map(|(url, token)| (url.clone(), decode_user(token).unwrap_or(Value::Null))).collect(),
        secret_store: secrets::backend()
    }
}

fn emit(app: &AppHandle) {
    let _ = app.emit("auth-changed", view(app));
}

/// Blocking: writes the password store. `None` signs out of that server.
pub fn set_token(app: &AppHandle, url: &str, token: Option<String>) -> Result<(), String> {
    load(app);
    secrets::set(app, &secret_key(url), token.as_deref())?;

    let servers = {
        let mut inner = auth(app).inner.lock().unwrap();
        match token {
            Some(token) => inner.tokens.insert(url.to_owned(), token),
            None => inner.tokens.remove(url)
        };
        inner.tokens.keys().cloned().collect()
    };
    save_index(app, servers);
    emit(app);
    super::refresh(app);
    Ok(())
}

/// The first launch after this moved out of the webview: takes over the tokens it kept in
/// localStorage, for the servers that don't have one here yet. Blocking.
pub fn migrate(app: &AppHandle, tokens: BTreeMap<String, String>) -> Result<(), String> {
    load(app);
    for (url, token) in tokens {
        if !token.is_empty() && self::token(app, &url).is_none() {
            set_token(app, &url, Some(token))?;
        }
    }
    Ok(())
}

/// The JWT's `user` claim. Not verified: the server does that, this only shows who it is.
fn decode_user(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let mut claims: Value = serde_json::from_slice(&bytes).ok()?;
    claims.get_mut("user").map(Value::take).filter(|user| !user.is_null())
}

/// Starts signing in to `url` unless that's already running, and re-sends the browser link it has.
pub fn sign_in(app: &AppHandle, url: String) {
    let uri = {
        let mut inner = auth(app).inner.lock().unwrap();
        match &inner.sign_in {
            Some(running) if running.url == url => Some(running.uri.clone()),
            _ => {
                if let Some(stop) = inner.sign_in.take().and_then(|old| old.stop) {
                    let _ = stop.send(());
                }
                let (tx, rx) = oneshot::channel();
                inner.sign_in = Some(SignIn { url: url.clone(), stop: Some(tx), uri: None });
                tauri::async_runtime::spawn(run_sign_in(app.clone(), url, rx));
                None
            }
        }
    };

    let _ = app.emit("auth-uri", uri.flatten());
}

pub fn sign_in_cancel(app: &AppHandle) {
    let old = auth(app).inner.lock().unwrap().sign_in.take();
    if let Some(stop) = old.and_then(|old| old.stop) {
        let _ = stop.send(());
    }
    let _ = app.emit("auth-uri", None::<String>);
}

async fn run_sign_in(app: AppHandle, url: String, mut stop: oneshot::Receiver<()>) {
    let socket_url = format!("{}/auth/desktop/ws", url.trim_end_matches('/'));

    loop {
        tokio::select! {
            done = sign_in_session(&app, &url, &socket_url) => if done {
                return;
            },
            _ = &mut stop => return
        }
        tokio::select! {
            _ = sleep(RETRY_DELAY) => {}
            _ = &mut stop => return
        }
    }
}

/// `true` once signed in.
async fn sign_in_session(app: &AppHandle, url: &str, socket_url: &str) -> bool {
    let mut socket = match crate::net::connect(socket_url, CONNECT_TIMEOUT).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::debug!("Sign-in socket {socket_url} unreachable: {err}");
            return false;
        }
    };

    let mut ticker = interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut waiting = false;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if waiting || socket.send(Message::text("ping")).await.is_err() {
                    return false;
                }
                waiting = true;
            }
            message = socket.next() => {
                waiting = false;
                let text = match message {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return false,
                    Some(Ok(_)) => continue
                };
                let Ok(data) = serde_json::from_str::<Value>(text.as_str()) else {
                    continue;
                };
                match data.get("type").and_then(Value::as_str) {
                    Some("auth-uri") => {
                        let uri = data.pointer("/message/authUrl").and_then(Value::as_str).map(str::to_owned);
                        if let Some(running) = auth(app).inner.lock().unwrap().sign_in.as_mut().filter(|s| s.url == url) {
                            running.uri = uri.clone();
                        }
                        let _ = app.emit("auth-uri", uri);
                    }
                    Some("auth-token") => {
                        let Some(token) = data.pointer("/message/token").and_then(Value::as_str).map(str::to_owned) else {
                            continue;
                        };
                        let _ = socket.close(None).await;
                        tracing::info!("Signed in to {url}");
                        {
                            let mut inner = auth(app).inner.lock().unwrap();
                            if inner.sign_in.as_ref().is_some_and(|s| s.url == url) {
                                inner.sign_in = None;
                            }
                        }
                        let _ = app.emit("auth-uri", None::<String>);
                        let (app, url) = (app.clone(), url.to_owned());
                        let saved = tauri::async_runtime::spawn_blocking(move || set_token(&app, &url, Some(token))).await;
                        if let Ok(Err(err)) | Err(err) = saved.map_err(|e| e.to_string()) {
                            tracing::error!("Couldn't save the sign-in token: {err}");
                        }
                        return true;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Response {
    status: u16,
    body: Value
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::builder().timeout(Duration::from_secs(60)).build().unwrap_or_default())
}

/// `ws://host` -> `http://host`, `wss://host` -> `https://host`.
fn http_url(url: &str) -> String {
    match url.strip_prefix("ws") {
        Some(rest) => format!("http{rest}"),
        None => url.to_owned()
    }
}

/// A request to the server's HTTP API at `path`, signed with the token for `url`.
pub async fn request(app: &AppHandle, url: &str, method: &str, path: &str, body: Option<Value>) -> Result<Response, String> {
    let token = token(app, url).ok_or("Not logged in - log in from Settings first")?;
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut request = client().request(method, format!("{}{path}", http_url(url).trim_end_matches('/'))).bearer_auth(token);
    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let text = response.text().await.map_err(|e| e.to_string())?;
    let body = serde_json::from_str(&text).unwrap_or(Value::Null);
    Ok(Response { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt(claims: &Value) -> String {
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!("{}.{}.sig", engine.encode(b"{\"alg\":\"HS256\"}"), engine.encode(claims.to_string()))
    }

    #[test]
    fn decodes_the_user_claim() {
        let token = jwt(&serde_json::json!({ "user": { "discord": { "id": "42", "name": "Sam" } }, "iat": 1 }));
        assert_eq!(decode_user(&token).unwrap()["discord"]["name"], "Sam");
    }

    #[test]
    fn no_user_claim_is_none() {
        assert!(decode_user(&jwt(&serde_json::json!({ "iat": 1 }))).is_none());
        assert!(decode_user("not-a-jwt").is_none());
    }

    #[test]
    fn http_url_follows_the_ws_scheme() {
        assert_eq!(http_url("ws://localhost:3000"), "http://localhost:3000");
        assert_eq!(http_url("wss://vot.example"), "https://vot.example");
    }
}
