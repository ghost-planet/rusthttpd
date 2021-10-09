use std::process;

use rustweb::WebServer;

fn main() {
    const ADDR: &str = "127.0.0.1:8000";
    if let Err(e) = WebServer::new(ADDR).run() {
        eprintln!("Run webserver {} failed for {}.", ADDR, e);
        process::exit(1);
    }
}
