//! BigQuery sink with warehouse SQL hashing pushdown (P4).

use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    read_boundary_json, PushdownMode, SinkConnector, SinkFingerprintBatch, ConnectorError,
    ConnectorResult,
};

/// Fingerprint rows via BigQuery `SHA256` SQL — rows never leave the warehouse.
pub struct BigQuerySinkConnector {
    pub project: String,
    pub dataset: String,
    pub table: String,
}

#[derive(serde::Deserialize)]
struct BqBoundary {
    sql_filter: Option<String>,
}

impl BigQuerySinkConnector {
    pub fn new(project: impl Into<String>, dataset: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            project: project.into(),
            dataset: dataset.into(),
            table: table.into(),
        }
    }

    fn hash_sql(&self, content_fields: &[String], filter: Option<&str>) -> String {
        let concat = content_fields
            .iter()
            .map(|f| format!("COALESCE(CAST(`{f}` AS STRING), '')"))
            .collect::<Vec<_>>()
            .join(" || '|' || ");
        let mut sql = format!(
            "SELECT TO_HEX(SHA256({concat})) AS row_hash FROM `{0}.{1}.{2}`",
            self.project, self.dataset, self.table
        );
        if let Some(f) = filter {
            sql.push_str(&format!(" WHERE {f}"));
        }
        sql
    }
}

impl SinkConnector for BigQuerySinkConnector {
    fn sink_ref(&self) -> &str {
        "bigquery"
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
        mode: PushdownMode,
    ) -> ConnectorResult<SinkFingerprintBatch> {
        if boundary.mode != BoundaryMode::BatchId {
            return Err(ConnectorError::InvalidBoundary(
                "bigquery uses BATCH_ID with optional sql_filter".into(),
            ));
        }
        let spec: BqBoundary = serde_json::from_slice(&read_boundary_json(boundary)?)
            .map_err(|e| ConnectorError::BigQuery(e.to_string()))?;
        let sql = self.hash_sql(content_fields, spec.sql_filter.as_deref());
        let rows = run_bq_query(&self.project, &sql)?;
        let mut fingerprints = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            let hash = row
                .get("row_hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ConnectorError::BigQuery("missing row_hash column".into()))?;
            let record = veridata_core::model::Record {
                fields: content_fields
                    .iter()
                    .map(|f| (f.clone(), veridata_core::model::FieldValue::Str(hash.to_string())))
                    .collect(),
            };
            let pos = Position {
                kind: PositionKind::IcebergRow,
                value: serde_json::to_vec(&serde_json::json!({
                    "warehouse": "bigquery",
                    "table": self.table,
                    "row_index": idx
                }))
                .unwrap_or_default(),
            };
            fingerprints.push(build_fingerprint(
                &record,
                content_fields,
                salt,
                pos,
                policy,
            )?);
        }
        Ok(SinkFingerprintBatch {
            fingerprints,
            pushdown_used: mode == PushdownMode::Pushdown,
        })
    }
}

fn run_bq_query(project: &str, sql: &str) -> ConnectorResult<Vec<serde_json::Map<String, serde_json::Value>>> {
    let token = gcp_token()?;
    let url = format!(
        "https://bigquery.googleapis.com/bigquery/v2/projects/{project}/queries"
    );
    let body = serde_json::json!({ "query": sql, "useLegacySql": false });
    let rt = tokio::runtime::Runtime::new().map_err(|e| ConnectorError::BigQuery(e.to_string()))?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ConnectorError::BigQuery(e.to_string()))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ConnectorError::BigQuery(e.to_string()))?;
        let fields: Vec<String> = json
            .get("schema")
            .and_then(|s| s.get("fields"))
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| f.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let rows = json
            .get("rows")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for row in rows {
            let cells = row.get("f").and_then(|f| f.as_array()).cloned().unwrap_or_default();
            let mut map = serde_json::Map::new();
            for (i, field) in fields.iter().enumerate() {
                if let Some(cell) = cells.get(i) {
                    if let Some(v) = cell.get("v") {
                        map.insert(field.clone(), v.clone());
                    }
                }
            }
            out.push(map);
        }
        Ok(out)
    })
}

fn gcp_token() -> ConnectorResult<String> {
    std::env::var("GOOGLE_APPLICATION_CREDENTIALS").map_err(|_| {
        ConnectorError::BigQuery("set GOOGLE_APPLICATION_CREDENTIALS for BigQuery".into())
    })?;
    let out = std::process::Command::new("gcloud")
        .args(["auth", "application-default", "print-access-token"])
        .output()
        .map_err(|e| ConnectorError::BigQuery(e.to_string()))?;
    if !out.status.success() {
        return Err(ConnectorError::BigQuery(
            "gcloud auth application-default login required".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_sql_includes_sha256() {
        let c = BigQuerySinkConnector::new("p", "d", "t");
        let sql = c.hash_sql(&["a".into(), "b".into()], None);
        assert!(sql.contains("SHA256"));
        assert!(sql.contains("p.d.t"));
    }

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_sink<T: SinkConnector>() {}
        assert_sink::<BigQuerySinkConnector>();
    }
}
