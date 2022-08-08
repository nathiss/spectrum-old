use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/protobuf/*.proto"], &["src/"])?;

    Ok(())
}
