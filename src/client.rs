//! Client construction for examples.

use crate::config::{
    ACCOUNT_ID_ENV, API_KEY_ID_ENV, API_PRIVATE_KEY_ENV, API_URL_ENV, SUB_ACCOUNT_ID_ENV, WS_URL_ENV,
    load_dotenv,
};
use polyester::{Client, Config};
use std::env;

/// Build a client from env. When `require_auth` is true, API key credentials are required.
pub fn client_from_env(require_auth: bool) -> anyhow::Result<Client> {
    load_dotenv(".env");
    let mut cfg = Config {
        hydrate_catalogs: true,
        ..Default::default()
    };
    if let Ok(url) = env::var(API_URL_ENV) {
        let url = url.trim();
        if !url.is_empty() {
            cfg.api_url = url.to_owned();
        }
    }
    if let Ok(url) = env::var(WS_URL_ENV) {
        let url = url.trim();
        if !url.is_empty() {
            cfg.ws_url = url.to_owned();
        }
    }
    if let Ok(account) = env::var(ACCOUNT_ID_ENV) {
        let account = account.trim();
        if !account.is_empty() {
            cfg.default_account_id = Some(account.to_owned());
        }
    }
    if let Ok(sub) = env::var(SUB_ACCOUNT_ID_ENV) {
        let sub = sub.trim();
        if !sub.is_empty() {
            cfg.default_sub_account_id = Some(sub.to_owned());
        }
    }

    let key_id = env::var(API_KEY_ID_ENV)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty());
    let private_key = env::var(API_PRIVATE_KEY_ENV)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty());

    match (key_id, private_key) {
        (Some(id), Some(pk)) => {
            cfg.api_key_id = Some(id);
            cfg.api_private_key = Some(pk);
        }
        _ if require_auth => {
            anyhow::bail!(
                "missing credentials. Set {API_KEY_ID_ENV} and {API_PRIVATE_KEY_ENV} \
                 (see .env.example)."
            );
        }
        _ => {}
    }

    Ok(Client::new(cfg)?)
}

/// Wait until catalog hydration finishes (best-effort when enabled).
pub async fn wait_for_catalogs(client: &Client) -> anyhow::Result<()> {
    client.wait_for_catalogs().await?;
    Ok(())
}
