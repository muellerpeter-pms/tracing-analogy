use std::io::copy;
use std::{fs::File, io::Read, path::Path};

use reqwest::Url;

const PROTO_FILE_NAME: &str = "./analogy/analogy.proto";
fn main() {
    // url to proto file
    let url = Url::parse("https://raw.githubusercontent.com/Analogy-LogViewer/Real-Time-Log-Server/main/Analogy.LogServer/Protos/Analogy.proto")
        .expect("Invalid URL");

    // path for storing the proto file
    let output_path = Path::new(PROTO_FILE_NAME);
    std::fs::create_dir_all("./analogy").expect("Failed to create path.");

    // send request and download file
    let response = reqwest::blocking::get(url)
        .expect("Failed to send request")
        .bytes()
        .expect("Failed to get responses bytes");

    if need_to_rewrite_file(output_path, &response.as_ref()) {
        let mut output_file = File::create(output_path).expect("Failed to create file");
        copy(&mut response.as_ref(), &mut output_file).expect("Failed to save file");
    }

    tonic_build::configure()
        .build_server(false)
        .out_dir("src/analogy")
        .compile(&[PROTO_FILE_NAME], &["analogy/"])
        .expect("Failed to build tonic gRpc client");
}

/// returns true if file needs to be rewritten
fn need_to_rewrite_file(filename: &Path, new_content: &[u8]) -> bool {
    if !filename.exists() {
        return true;
    }
    let len = filename
        .metadata()
        .expect("Failed to get files metadata")
        .len() as usize;

    if len != new_content.len() {
        return true;
    }

    let mut buffer = Vec::with_capacity(len);
    File::options()
        .read(true)
        .open(filename)
        .expect("Failed to open existing proto file")
        .read_exact(&mut buffer)
        .expect("Failed to read existing file");

    for cmp in new_content.iter().zip(buffer) {
        if cmp.0 != &cmp.1 {
            return true;
        }
    }

    false
}
