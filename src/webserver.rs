use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write};
use std::fs;
use std::process::{Command, Stdio};
use std::os::unix::fs::PermissionsExt;
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
            let request_line = request_line.iter()
                                            .map(|c| {
                                                    let c = *c as char;
                                                    c.to_ascii_uppercase()
                                                });

            let method: String = request_line.clone().take_while(|c| !c.is_whitespace()).collect();
            if method != "GET" && method != "POST" {
                WebServer::unimplemented(&mut socket);
                return;
            }

            let skip = method.len() + 1;
            let mut url: String = request_line.clone().skip(skip).take_while(|c| !c.is_whitespace()).collect();

            let mut query_string: Option<String> = None;
            if let Some(pos) = url.find("?") {
                query_string = Some(url[pos+1..].to_string());
                url = url[..pos].to_string();
            }

            let skip = skip + url.len() + 1;
            let _protocol: String = request_line.clone().skip(skip).take_while(|c| !c.is_whitespace()).collect();
            
            let path = WebServer::url_to_path(&url);
            
            if path.is_none() {
                WebServer::discard_all_headers(&mut socket);
                WebServer::not_found(&mut socket, &url);
            } else {
                let path = path.unwrap();
                let cgi = WebServer::file_executable(&path) || query_string.is_some();
                
                if cgi {
                    WebServer::execute_cgi(&mut socket, &path, &method, &query_string.unwrap_or_default());
                } else {
                    WebServer::discard_all_headers(&mut socket);
                    WebServer::serve_file(&mut socket, &path);
                }
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
        // 返回 501
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

    fn cannot_exectute(socket: &mut TcpStream, _url: &str) {
        //返回500
        socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\n").unwrap();
        socket.write_all(SERVER_STRING).unwrap();
        socket.write_all(b"Content-Type: text/html\r\n").unwrap();
        socket.write_all(b"\r\n").unwrap();

        WebServer::write_file(socket, "assets/500.html");

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

    fn bad_request(socket: &mut TcpStream, _url: &str) {
        //返回400
        socket.write_all(b"HTTP/1.1 400 Bad Request\r\n").unwrap();
        socket.write_all(SERVER_STRING).unwrap();
        socket.write_all(b"Content-Type: text/html\r\n").unwrap();
        socket.write_all(b"\r\n").unwrap();

        WebServer::write_file(socket, "assets/400.html");

        socket.flush().unwrap(); 
    }

    fn execute_cgi(socket: &mut TcpStream, url: &str, method: &str, query_string: &str) {
        let mut command = Command::new(url);
        command.env("REQUEST_METHOD", method);

        let output;
        if method == "GET" {
            WebServer::discard_all_headers(socket);
            output = command.env("QUERY_STRING", query_string).output();
        } else {
            let mut buffer = [0;1024];

            let mut content_length = loop {
                let line = WebServer::read_line(socket, &mut buffer[..]);
                
                if line.len() == 0 {
                    break -1;
                }

                let line: String = line.iter()
                                        .map(|c| {
                                            let c = *c as char;
                                            c.to_ascii_uppercase()
                                        })
                                        .collect();
                if line.contains("CONTENT-LENGTH:") {
                    break line[16..].parse::<i32>().unwrap_or(-1);
                }
            };

            if content_length == -1 {
                WebServer::discard_all_headers(socket);
                WebServer::bad_request(socket, url);
                return;
            }

            WebServer::discard_all_headers(socket);
            if let Ok(process) = command.env("CONTENT_LENGTH", content_length.to_string())
                                        .stdin(Stdio::piped())
                                        .stdout(Stdio::piped())
                                        .spawn() {
                let mut stdin = process.stdin.as_ref().unwrap();
                while content_length > 0 {
                    let read = if content_length > 1024 {1024} else {content_length};
                    content_length -= read;
                    
                    let read = read as usize;
                    socket.read_exact(&mut buffer[0..read]).unwrap();
                    stdin.write_all(&buffer[0..read]).unwrap();
                }

                output = process.wait_with_output();
            } else {
                WebServer::cannot_exectute(socket, url);
                return;
            }
        }
        
        match output {
            Ok(output) if output.status.success() => {
                socket.write_all(b"HTTP/1.1 200 OK\r\n").unwrap();
                socket.write_all(SERVER_STRING).unwrap();
                socket.write_all(&output.stdout).unwrap();
            },
            _ => WebServer::cannot_exectute(socket, url),
        };
        
        socket.flush().unwrap();
    }

    fn discard_all_headers(socket: &mut TcpStream) {
        let mut buffer = [0; 1024];
        loop {
            let line = WebServer::read_line(socket, &mut buffer[..]);
            if line.len() == 0 {
                break;
            }
        }
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

    fn file_executable(url: &str) -> bool {
        let metadata = fs::metadata(url);
        if metadata.is_err() {
            return false;
        }
        
        let metadata = metadata.unwrap();
        let mode = metadata.permissions().mode();

        const IXUSR: u32 =  0o100;
        const IXGRP: u32 =  0o010;
        const IXOTH: u32 =  0o001;
        
        (mode & IXUSR) > 0 || (mode & IXGRP) > 0 || (mode & IXOTH) > 0
    }
}