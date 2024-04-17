
use std::fs::File;
use std::io::copy;
use reqwest::Url;

fn main() {
    // url to proto file 
    let url = Url::parse("https://raw.githubusercontent.com/Analogy-LogViewer/Real-Time-Log-Server/main/Analogy.LogServer/Protos/Analogy.proto")
        .expect("Invalid URL");

    // path for storing the proto file 
    let output_path = std::path::Path::new("./analogy/analogy.proto");
    std::fs::create_dir_all("./analogy").expect("Failed to create path.");
    
    // send request and download file
    let response = reqwest::blocking::get(url).expect("Failed to send request");
    let mut output_file = File::create(output_path).expect("Failed to create file");
    copy(&mut response.bytes().expect("Failed to get responses bytes").as_ref(), &mut output_file).expect("Failed to save file");
}