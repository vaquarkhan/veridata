//! GCP Pub/Sub source connector (P4).

use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    read_boundary_json, SourceConnector, ConnectorError, ConnectorResult,
};

/// Pull messages from a Pub/Sub subscription for the declared boundary window.
pub struct PubSubSourceConnector {
    pub project: String,
    pub subscription: String,
}

#[derive(serde::Deserialize)]
struct PubSubBoundary {
    max_messages: usize,
}

impl PubSubSourceConnector {
    pub fn new(project: impl Into<String>, subscription: impl Into<String>) -> Self {
        Self {
            project: project.into(),
            subscription: subscription.into(),
        }
    }
}

impl SourceConnector for PubSubSourceConnector {
    fn source_ref(&self) -> &str {
        "pubsub"
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
    ) -> ConnectorResult<Vec<Fingerprint>> {
        if boundary.mode != BoundaryMode::OffsetRange {
            return Err(ConnectorError::InvalidBoundary(
                "pubsub uses OFFSET_RANGE with {\"max_messages\":N}".into(),
            ));
        }
        let spec: PubSubBoundary = serde_json::from_slice(&read_boundary_json(boundary)?)
            .map_err(|e| ConnectorError::Gcp(e.to_string()))?;
        let token = gcp_token()?;
        let url = format!(
            "https://pubsub.googleapis.com/v1/projects/{}/subscriptions/{}:pull",
            self.project, self.subscription
        );
        let body = serde_json::json!({
            "maxMessages": spec.max_messages,
            "returnImmediately": true
        });
        let rt = tokio::runtime::Runtime::new().map_err(|e| ConnectorError::Gcp(e.to_string()))?;
        let messages: Vec<serde_json::Value> = rt.block_on(async {
            let client = reqwest::Client::new();
            let resp = client
                .post(&url)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ConnectorError::Gcp(e.to_string()))?;
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| ConnectorError::Gcp(e.to_string()))?;
            json.get("receivedMessages")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
        });

        let mut out = Vec::new();
        for (idx, msg) in messages.iter().enumerate() {
            let data_b64 = msg
                .get("message")
                .and_then(|m| m.get("data"))
                .and_then(|d| d.as_str())
                .ok_or_else(|| ConnectorError::Gcp("pubsub message missing data".into()))?;
            let payload = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                data_b64,
            )
            .map_err(|e| ConnectorError::Gcp(e.to_string()))?;
            let record = veridata_connector_kafka::parse_message(&payload)?;
            let pos = Position {
                kind: PositionKind::KafkaOffset,
                value: serde_json::to_vec(&serde_json::json!({"subscription": self.subscription, "index": idx}))
                    .unwrap_or_default(),
            };
            out.push(build_fingerprint(
                &record,
                content_fields,
                salt,
                pos,
                policy,
            )?);
        }
        Ok(out)
    }
}

fn gcp_token() -> ConnectorResult<String> {
    std::env::var("GOOGLE_APPLICATION_CREDENTIALS").map_err(|_| {
        ConnectorError::Gcp("set GOOGLE_APPLICATION_CREDENTIALS for Pub/Sub".into())
    })?;
    let out = std::process::Command::new("gcloud")
        .args(["auth", "application-default", "print-access-token"])
        .output()
        .map_err(|e| ConnectorError::Gcp(e.to_string()))?;
    if !out.status.success() {
        return Err(ConnectorError::Gcp(
            "gcloud auth application-default login required".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_source<T: SourceConnector>() {}
        assert_source::<PubSubSourceConnector>();
    }
}
