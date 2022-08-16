use std::io::Result;

fn main() -> Result<()> {
    std::env::set_var("PROTOC", protobuf_src::protoc());
    prost_build::compile_protos(
        &[
            "src/protobuf/client-message.proto",
            "src/protobuf/server-message.proto",
        ],
        &["src/"],
    )?;

    Ok(())
}
