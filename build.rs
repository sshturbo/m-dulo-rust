fn main() {
    tonic_build::configure()
        .compile(
            &["grpcurl/command.proto", "grpcurl/typed_message.proto"],
            &["grpcurl"],
        )
        .unwrap();
}