use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

pub struct PanClient {
    base: String,
    client: reqwest::Client,
}

impl PanClient {
    pub fn new(server: &str) -> Self {
        Self {
            base: server.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("could not connect to server at {}: {}", self.base, e))?;

        let status = resp.status();
        let text = resp.text().await?;

        // 200 (duplicate) and 201 (created) are both success.
        if status.is_success() {
            let v: Value = serde_json::from_str(&text)
                .unwrap_or(Value::Null);
            Ok(v)
        } else {
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            let msg = v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or(text.as_str());
            anyhow::bail!("{}", msg)
        }
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("could not connect to server at {}: {}", self.base, e))?;

        let status = resp.status();
        let text = resp.text().await?;

        if status.is_success() {
            let v: Value = serde_json::from_str(&text)?;
            Ok(v)
        } else {
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            let msg = v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or(text.as_str());
            anyhow::bail!("{}", msg)
        }
    }
}
