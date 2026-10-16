//! OpenShock's REST API: the shocker list (which also checks the token), and sending controls when
//! the live hub isn't connected.
use reqwest::Client;
use serde::Deserialize;

use super::{Control, Hub, API};

/// GET /1/shockers/own wraps its list in `{ data, message }`.
#[derive(Deserialize)]
struct Envelope<T> {
    data: T
}

pub async fn shockers(client: &Client, token: &str) -> Result<Vec<Hub>, String> {
    let response = client
        .get(format!("{API}/1/shockers/own"))
        .header("OpenShockToken", token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("OpenShock answered {}", response.status()));
    }

    response.json::<Envelope<Vec<Hub>>>().await.map(|envelope| envelope.data).map_err(|e| e.to_string())
}

pub async fn control(client: &Client, token: &str, controls: &[Control]) -> Result<(), String> {
    let body: Vec<_> = controls.iter().map(Control::rest_json).collect();
    let response = client
        .post(format!("{API}/1/shockers/control"))
        .header("OpenShockToken", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("OpenShock answered {}", response.status()))
    }
}
