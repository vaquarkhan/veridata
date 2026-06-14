# P4 cloud connector examples (require `cargo build -p veridata-cli --features cloud`)

## AWS — MSK + Iceberg-on-S3 + AWS KMS + S3 proof store

```yaml
producer: veridata/0.1.0
source:
  type: msk
  topic: orders
  bootstrap_servers: b-1.example.kafka.us-east-1.amazonaws.com:9098
  region: us-east-1
  boundary:
    partitions: [{ id: 0, start: 0, end: 9999 }]
sink:
  type: iceberg_s3
  warehouse_uri: s3://my-warehouse/iceberg
  table: orders
  boundary:
    snapshot_from: 1
    snapshot_to: 1
policy:
  identity_rule: "composite:[order_id,line_id]"
  hash_algorithm: sha256
  content_fields: [order_id, line_id, amount, status]
  tolerances: { max_drops: 0, max_mutations: 0, duplicates: FORBID }
  late_arrival_window: 900s
crypto:
  kms_provider: aws
  kms_key_id: arn:aws:kms:us-east-1:123456789012:key/abc
  aws_region: us-east-1
  private_key_file: .veridata/keys/signing.key.b64   # fallback for local verify
  public_key_file: .veridata/keys/signing.pub.b64
store:
  kind: s3
  bucket: my-proofs
  prefix: vrp/
  region: us-east-1
  proofs_dir: .veridata/proofs
```

## GCP — Pub/Sub + BigQuery SQL pushdown + Cloud KMS + GCS proofs

```yaml
source:
  type: pubsub
  topic: orders-sub
  project: my-gcp-project
  subscription: orders-veridata
  boundary:
    partitions: [{ id: 0, start: 0, end: 0 }]  # use max_messages in boundary JSON for pubsub
sink:
  type: bigquery
  table: orders
  dataset: analytics
  boundary:
    snapshot_from: 0
    snapshot_to: 0
    sql_filter: "ingest_date = CURRENT_DATE()"
crypto:
  kms_provider: gcp
  kms_key_id: projects/p/locations/us/keyRings/r/cryptoKeys/k/cryptoKeyVersions/1
store:
  kind: gcs
  bucket: my-proofs
  prefix: vrp/
```

## Azure — Event Hubs + Delta-on-ADLS + Key Vault

```yaml
source:
  type: eventhubs
  topic: orders
  connection_string: "Endpoint=sb://....servicebus.windows.net/;..."
sink:
  type: iceberg_s3   # ADLS via abfs — use warehouse_uri with azure store config
  warehouse_uri: s3://container@account.dfs.core.windows.net/delta/orders
  table: orders
crypto:
  kms_provider: azure
  kms_key_id: veridata-signing
  azure_vault_url: https://myvault.vault.azure.net/
store:
  kind: adls
  account: mystorage
  container: proofs
  prefix: vrp/
```

## Databricks — Delta / Unity Catalog

```yaml
sink:
  type: databricks
  warehouse_uri: s3://unity-catalog-bucket/delta/orders
  table: orders
  catalog: main
  schema: default
```

Build with cloud features:

```bash
cargo build -p veridata-cli --features cloud
```

KMS features (enable per cloud):

```bash
cargo build -p veridata-cloud --features aws
cargo build -p veridata-cloud --features gcp
cargo build -p veridata-cloud --features azure
cargo build -p veridata-cloud --features all
```
