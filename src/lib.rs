//! A tiny webserver with rust.
//!
//! The whole implement is mimic from tinyhttpd.

mod threadpool;
mod webserver;

use std::net::ToSocketAddrs;
use webserver::WebServer;

/// Startup a webserver with a threadpool allocates n threads.
///
/// # Examples
///
/// ```
/// if let Err(e) = rustweb::run("localhost:8000", 8) {
///     eprintln!("Run webserver failed for {}.", e);
///     process::exit(1);
/// }
/// ```
pub fn run<A: ToSocketAddrs>(addr: A, nthreads: usize) -> std::io::Result<()> {
    WebServer::new(nthreads).run(addr)
}