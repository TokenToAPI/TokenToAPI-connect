//! Website handoffs keep credentials native and require a separate import confirmation.
use crate::models;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, time::Duration};

const RETRY: &str =
    "Return to the platform and prepare a new import link, then open Connect again.";

#[derive(Clone)]
pub struct HandoffLink {
    pub issuer: String,
    code: String,
}

pub fn parse(input: &str, allow_loopback: bool) -> Result<HandoffLink, String> {
    if input.len() > 4096 {
        return Err(RETRY.into());
    }
    let url = reqwest::Url::parse(input).map_err(|_| RETRY)?;
    if url.scheme() != "tokentoapi-connect"
        || url.host_str() != Some("v1")
        || url.path() != "/import"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(RETRY.into());
    }
    let mut issuer = None;
    let mut code = None;
    for (name, value) in url.query_pairs() {
        match name.as_ref() {
            "issuer" if issuer.is_none() => issuer = Some(value.into_owned()),
            "code" if code.is_none() => code = Some(value.into_owned()),
            _ => return Err(RETRY.into()),
        }
    }
    let issuer = issuer.ok_or(RETRY)?;
    let origin = reqwest::Url::parse(&issuer).map_err(|_| RETRY)?;
    validate_https(&origin, allow_loopback)?;
    if origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
        || !origin.username().is_empty()
        || origin.password().is_some()
    {
        return Err(RETRY.into());
    }
    let code = code.ok_or(RETRY)?;
    if code.len() != 43
        || !code
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || !"AEIMQUYcgkosw048".contains(code.as_bytes()[42] as char)
    {
        return Err(RETRY.into());
    }
    Ok(HandoffLink {
        issuer: origin.origin().ascii_serialization(),
        code,
    })
}

fn validate_https(url: &reqwest::Url, allow_loopback: bool) -> Result<(), String> {
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.host_str().is_none()
        || !(url.scheme() == "https" || (allow_loopback && local && url.scheme() == "http"))
    {
        return Err(RETRY.into());
    }
    Ok(())
}

// No Debug or Serialize: a redeemed key must never be sent to renderer events/storage.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffSetup {
    version: u32,
    pub provider_name: String,
    pub base_url: String,
    pub api_key: String,
    pub suggested_client: models::Client,
}

pub async fn redeem(link: &HandoffLink, allow_loopback: bool) -> Result<HandoffSetup, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| RETRY)?;
    let mut response = client
        .post(format!("{}/api/v1/connect/handoff/redeem", link.issuer))
        .json(&serde_json::json!({"code": link.code}))
        .send()
        .await
        .map_err(|_| RETRY)?;
    if !response.status().is_success() {
        return Err(RETRY.into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| RETRY)? {
        if bytes.len() + chunk.len() > 64 * 1024 {
            return Err(RETRY.into());
        }
        bytes.extend_from_slice(&chunk);
    }
    #[derive(Deserialize)]
    struct Envelope {
        code: u32,
        data: HandoffSetup,
    }
    let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| RETRY)?;
    let mut setup = envelope.data;
    if envelope.code != 0
        || setup.version != 1
        || setup.provider_name.is_empty()
        || setup.provider_name.len() > 128
        || setup.provider_name.chars().any(char::is_control)
        || setup.api_key.is_empty()
        || setup.api_key.len() > 4096
        || setup
            .api_key
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        || !matches!(
            setup.suggested_client,
            models::Client::Claude | models::Client::Codex
        )
    {
        return Err(RETRY.into());
    }
    if setup.base_url.len() > 2048 {
        return Err(RETRY.into());
    }
    setup.base_url = models::normalize_root(&setup.base_url).map_err(|_| RETRY)?;
    validate_https(
        &reqwest::Url::parse(&setup.base_url).map_err(|_| RETRY)?,
        allow_loopback,
    )?;
    Ok(setup)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingHandoff {
    pub request_id: String,
    pub issuer: String,
    pub known_issuer: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupView {
    pub request_id: String,
    pub provider_name: String,
    pub base_url: String,
    pub suggested_client: models::Client,
    pub gateway_needs_review: bool,
}

#[derive(Default)]
pub struct HandoffState {
    incoming: Option<(String, HandoffLink)>,
    accepted: Option<(String, HandoffSetup)>,
    seen: VecDeque<String>,
}
impl HandoffState {
    pub fn receive(&mut self, url: &str, allow_loopback: bool) -> bool {
        let Ok(link) = parse(url, allow_loopback) else {
            return false;
        };
        let id = format!(
            "{:x}",
            Sha256::digest(format!("{}:{}", link.issuer, link.code))
        );
        if self.seen.contains(&id) {
            return false;
        }
        self.seen.push_back(id.clone());
        if self.seen.len() > 32 {
            self.seen.pop_front();
        }
        self.incoming = Some((id, link));
        true
    }
    pub fn pending(&self) -> Option<PendingHandoff> {
        self.incoming.as_ref().map(|(id, link)| PendingHandoff {
            request_id: id.clone(),
            issuer: link.issuer.clone(),
            known_issuer: link.issuer == "https://tokentoapi.com",
        })
    }
    pub fn link(&self, request_id: &str) -> Result<HandoffLink, String> {
        self.incoming
            .as_ref()
            .filter(|(id, _)| id == request_id)
            .map(|(_, link)| link.clone())
            .ok_or_else(|| RETRY.into())
    }
    pub fn accept(&mut self, request_id: &str, setup: HandoffSetup) -> Result<SetupView, String> {
        let link = self.link(request_id)?;
        let origin = reqwest::Url::parse(&setup.base_url)
            .map_err(|_| RETRY)?
            .origin()
            .ascii_serialization();
        let view = SetupView {
            request_id: request_id.into(),
            provider_name: setup.provider_name.clone(),
            base_url: setup.base_url.clone(),
            suggested_client: setup.suggested_client,
            gateway_needs_review: origin != link.issuer,
        };
        self.accepted = Some((request_id.into(), setup));
        self.incoming = None;
        Ok(view)
    }
    pub fn discovery_input(
        &self,
        request_id: &str,
        client: models::Client,
    ) -> Result<models::DiscoveryInput, String> {
        let (_, setup) = self
            .accepted
            .as_ref()
            .filter(|(id, _)| id == request_id)
            .ok_or(RETRY)?;
        if setup.suggested_client.is_codex() != client.is_codex() {
            return Err("Choose a client compatible with this API key.".into());
        }
        Ok(models::DiscoveryInput {
            client,
            api_key: setup.api_key.clone(),
            base_url: setup.base_url.clone(),
        })
    }
    pub fn cancel(&mut self, request_id: &str) -> bool {
        if self
            .incoming
            .as_ref()
            .is_some_and(|(id, _)| id == request_id)
        {
            self.incoming = None;
        }
        let cleared = self
            .accepted
            .as_ref()
            .is_some_and(|(id, _)| id == request_id);
        if cleared {
            self.accepted = None;
        }
        cleared
    }
    pub fn clear_accepted(&mut self) {
        self.accepted = None;
    }
    pub fn completion_provider(&self, base_url: &str) -> Option<String> {
        self.accepted
            .as_ref()
            .filter(|(_, setup)| setup.base_url == base_url)
            .map(|(_, setup)| setup.provider_name.clone())
    }
}
