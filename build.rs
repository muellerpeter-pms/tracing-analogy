use std::fs::File;
use std::io::copy;

use reqwest::Url;

const PROTO_FILE_NAME: &str = "./analogy/analogy.proto";
fn main() {
    // url to proto file
    let url = Url::parse("https://raw.githubusercontent.com/Analogy-LogViewer/Real-Time-Log-Server/main/Analogy.LogServer/Protos/Analogy.proto")
        .expect("Invalid URL");

    // path for storing the proto file
    let output_path = std::path::Path::new(PROTO_FILE_NAME);
    std::fs::create_dir_all("./analogy").expect("Failed to create path.");

    // send request and download file
    let response = reqwest::blocking::get(url).expect("Failed to send request");
    let mut output_file = File::create(output_path).expect("Failed to create file");
    copy(
        &mut response
            .bytes()
            .expect("Failed to get responses bytes")
            .as_ref(),
        &mut output_file,
    )
    .expect("Failed to save file");

    tonic_build::configure()
        .build_server(false)
        .out_dir("src/analogy")
        .compile(&[PROTO_FILE_NAME], &["analogy/"])
        .expect("Failed to build tonic gRpc client");
}
