use std::{env, fs};
use tempfile::TempDir;

use nkv::srv;

const DEFAULT_URL: &str = "/tmp/nkv/nkv.sock";

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let url = if args.len() > 1 {
        &args[1]
    } else {
        DEFAULT_URL
    };
    if fs::metadata(url).is_ok() {
        fs::remove_file(url).expect("Failed to remove old socket");
    }

    let temp_dir = TempDir::new().expect("Failed to create temporary directory");

    // creates a task where it waits to serve threads
    let (mut srv, _cancel) = srv::Server::new(url.to_string(), temp_dir.path().to_path_buf())
        .await
        .unwrap();

    srv.serve().await;
}
