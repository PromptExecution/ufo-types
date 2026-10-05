//! Emit the portable wire schema for browser/code generation from the Rust contract.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(
        std::io::stdout(),
        &schemars::schema_for!(ufo_types::revision::PortableBundle),
    )?;
    Ok(())
}
