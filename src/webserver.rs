use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write};
use std::fs;
use super::threadpool::ThreadPool;

const SERVER_STRING: &[u8] = b"Server: rustweb/0.1.0\r\n";

pub struct WebServer {
    addr: String,
    threadpool: ThreadPool,
}

impl WebServer {
    pub fn new(addr: &str, threads: usize) -> Self {
        Self {
            addr: addr.to_string(),
            threadpool: ThreadPool::new(threads),
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.addr)?;
        loop {
            match listener.accept() {
                Ok((socket, addr)) => self.handle_client(socket, addr),
                Err(e) => eprintln!("Client connect failed for {}", e),
            }
        }
    }

    fn handle_client(&self, mut socket: TcpStream, addr: SocketAddr) {
        self.threadpool.execute(move || {
            println!("Request from {}", addr);

            let mut buffer = [0; 1024];
            let request_line = WebServer::read_line(&mut socket, &mut buffer[..]);
            let request_line = request_line.iter().map(|c| *c as char);

            let method: String = request_line.clone().take_while(|c| !c.is_whitespace()).collect();
            if method.to_uppercase() != "GET" {
                WebServer::unimplemented(&mut socket);
                return;
            }

            let skip = method.len() + 1;
            let url: String = request_line.clone().skip(skip).take_while(|c| !c.is_whitespace()).collect();

            let skip = skip + url.len() + 1;
            let _protocol: String = request_line.clone().skip(skip).take_while(|c| !c.is_whitespace()).collect(); 

            let path = WebServer::url_to_path(&url);
            
            if path.is_none() {
                WebServer::not_found(&mut socket, &url);
            } else {
                WebServer::serve_file(&mut socket, &path.unwrap());
            }
        });
    }

    fn read_line<'a>(socket: &mut TcpStream, buf: &'a mut [u8]) -> &'a [u8] {
        let mut iter = socket.bytes().peekable();
        let mut i = 0;
        while i < buf.len() {
            let next = iter.next();
            match next {
                Some(Ok(v)) if v != b'\n' && v != b'\r' => {
                    buf[i] = v;
                    i += 1;
                },
                Some(Ok(v)) if v == b'\r' => {
                    if let Some(Ok(b'\n')) = iter.peek() {
                        iter.next();
                    }
                    break;
                }
                _ => break,
            }
        }

        &buf[0..i]
    }

    fn unimplemented(socket: &mut TcpStream) {
        socket.write_all(b"HTTP/1.1 501 Method Not Implemented\r\n").unwrap();
        socket.write_all(SERVER_STRING).unwrap();
        socket.write_all(b"Content-Type: text/html\r\n").unwrap();
        socket.write_all(b"\r\n").unwrap();

        WebServer::write_file(socket, "assets/501.html");

        socket.flush().unwrap();
    }

    fn not_found(socket: &mut TcpStream, _url: &str) {
        //返回404
        socket.write_all(b"HTTP/1.1 404 NOT FOUND\r\n").unwrap();
        socket.write_all(SERVER_STRING).unwrap();
        socket.write_all(b"Content-Type: text/html\r\n").unwrap();
        socket.write_all(b"\r\n").unwrap();

        WebServer::write_file(socket, "assets/404.html");

        socket.flush().unwrap();
    }

    fn serve_file(socket: &mut TcpStream, url: &str) {
        //返回 200
        socket.write_all(b"HTTP/1.1 200 OK\r\n").unwrap();
        socket.write_all(SERVER_STRING).unwrap();
        socket.write_all(b"Content-Type: text/html\r\n").unwrap();
        socket.write_all(b"\r\n").unwrap();

        WebServer::write_file(socket, url);

        socket.flush().unwrap();
    }

    fn write_file(socket: &mut TcpStream, url: &str) {
        let mut f = fs::File::open(url).unwrap();
        let mut buf: [u8; 1024] = [0; 1024];
        while let Ok(size) = f.read(&mut buf) {
            if size == 0 {
                break;
            }
            socket.write_all(&buf[0..size]).unwrap();
        }
    }

    fn url_to_path(url: &str) -> Option<String> {
        let mut path = format!("assets{}", url);
        if path.ends_with("/") {
            path = path + "index.html";
        }

        let mut metadata = fs::metadata(path.clone());
        if let Ok(ref data) = metadata {
            if data.is_dir() {
                path = path + "/index.html";
                metadata = fs::metadata(path.clone());
            }
        }

        match metadata {
            Ok(ref data) if data.is_file() => Some(path),
            _ => None,
        }
    }
}