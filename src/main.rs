extern crate clap;
use clap::{Arg, App};

use std::process;

fn main() {
    let matches = App::new("Rust WebServer")
                        .version("0.1.0")
                        .author("Vincent Gong. <return0xffff@gmail.com>")
                        .about("A tiny webserver with rust")
                        .arg(Arg::with_name("threads")
                            .short("t")
                            .long("threads")
                            .value_name("THREADS")
                            .help("Sets the number of threads in thread pool to handle requests")
                            .takes_value(true))
                        .arg(Arg::with_name("ADDRESS")
                            .help("Sets IP/PORT to bind")
                            .required(true)
                            .index(1))
                        .get_matches();

    let addr = matches.value_of("ADDRESS").unwrap();
    let threads = matches.value_of("THREADS").unwrap_or_default();
    let threads: usize = threads.parse().unwrap_or(8);

    if let Err(e) = rustweb::run(addr, threads) {
        eprintln!("Run webserver {} failed for {}.", addr, e);
        process::exit(1);
    }
}
