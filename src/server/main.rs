use std::{env, fs, path::PathBuf};
use tempfile::TempDir;

use nkv::flag_parser::FlagParser;
use nkv::srv;

const DEFAULT_URL: &str = "/tmp/nkv/nkv.sock";

const HELP_MESSAGE: &str = "nkv-server [OPTIONS]
Run the notify key-value (nkv) server.

OPTIONS:
  --dir <path-to-store-data>
      Specify the directory where data files will be stored.
      If not provided, it defaults to the user’s data directory under the `nkv` folder.

  --addr <path-to-socket>
      Define the address to listen for connections.
      Supports UNIX socket path.

  --logs <path-to-logs>
      Specify the directory where logs should be stored.
      Defaults to `./log` if not provided.

  --level <info|debug|trace>
      Set the logging level:
        - `info`  (default): Standard operational messages.
        - `debug` : More detailed debugging information.
        - `trace` : Maximum level of detailed logs.

  --help
      Display this help message and exit.";

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let allowed_flags = vec!["dir".to_string(), "help".to_string(), "sock".to_string()];
    let args: Vec<String> = env::args().collect();
    let flags = match FlagParser::new(args, Some(allowed_flags)) {
        Ok(res) => res,
        Err(err) => {
            println!("error: {}", err);
            println!("{}", HELP_MESSAGE);
            return;
        }
    };

    if flags.get("help").is_some() {
        println!("{}", HELP_MESSAGE);
        return;
    }

    let sock_path = match flags.get("sock") {
        Some(&Some(ref val)) => val.clone(),
        _ => DEFAULT_URL.to_string(),
    };

    if fs::metadata(&sock_path).is_ok() {
        fs::remove_file(&sock_path).expect("Failed to remove old socket");
    }

    let dir = match flags.get("dir") {
        Some(&Some(ref val)) => {
            fs::create_dir_all(&val).expect(&format!("Failed to create directory {}", &val));
            PathBuf::from(val)
        }
        _ => TempDir::new()
            .expect("Failed to create temporary directory")
            .into_path(),
    };

    // creates a task where it waits to serve threads
    let (mut srv, _cancel) = srv::Server::new(sock_path.to_string(), dir).await.unwrap();

    srv.serve().await;
}
