use duckdb::{Connection, Result};

fn main() -> Result<()> {
    let _conn = Connection::open_in_memory()?;
    Ok(())
}
