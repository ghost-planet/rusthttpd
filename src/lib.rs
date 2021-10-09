mod threadpool;
mod webserver;

use webserver::WebServer;

pub fn run(addr: &str, threads: usize) -> std::io::Result<()> {
    WebServer::new(addr, threads).run()
}