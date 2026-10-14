//! Emit the portable wire schema for browser/code generation from the Rust contract.
use ufo_types::revision::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let contract = args.next().unwrap_or_else(|| "bundle".into());
    if args.next().is_some() {
        return Err("expected at most one contract name".into());
    }
    let schema = match contract.as_str() {
        "bundle" => schemars::schema_for!(PortableBundle),
        "model" => schemars::schema_for!(PortableModel),
        "changeset" => schemars::schema_for!(ChangeSet),
        "conflict" => schemars::schema_for!(Conflict),
        "merge" => schemars::schema_for!(MergeOutcome),
        "receipt" => schemars::schema_for!(OperationReceipt),
        "checkpoint" => schemars::schema_for!(IndexCheckpoint),
        "status" => schemars::schema_for!(SyncStatus),
        "expected-head" => schemars::schema_for!(ExpectedHead),
        "fidelity" => schemars::schema_for!(FidelityReport),
        "capabilities" => schemars::schema_for!(AdapterCapabilities),
        "query-request" => schemars::schema_for!(RevisionQueryRequest),
        "query-response" => schemars::schema_for!(RevisionQueryResponse),
        "graph-descriptor" => schemars::schema_for!(RevisionGraphDescriptor),
        _ => return Err(format!("unknown revision contract: {contract}").into()),
    };
    serde_json::to_writer_pretty(std::io::stdout(), &schema)?;
    Ok(())
}
