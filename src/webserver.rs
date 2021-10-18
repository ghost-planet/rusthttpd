use std::net::{TcpListener, TcpStream, SocketAddr, ToSocketAddrs};
use std::io::{Read, Write};
use std::io::Result as IOResult;
use std::fs;
use std::process::{Command, Stdio};
use std::os::unix::fs::PermissionsExt;
use super::threadpool::ThreadPool;

const SERVER_STRING: &[u8] = b"Server: rustweb/0.1.0\r\n";

pub struct WebServer {
    threadpool: ThreadPool,
}

impl WebServer {
    pub fn new(nthreads: usize) -> Self {
        Self {
            threadpool: ThreadPool::new(nthreads),
        }
    }

    pub fn run<A: ToSocketAddrs>(&self, addr: A) -> IOResult<()> {
        let listener = TcpListener::bind(addr)?;
        loop {
            match listener.accept() {
                Ok((socket, addr)) => {
                    println!("Request from {}", addr);
                    self.handle_client(socket, addr);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn handle_client(&self, mut socket: TcpStream, addr: SocketAddr) {
        let mut f = move || {
            let mut buffer = [0; 1024];
            let request_line = WebServer::read_line(&mut socket, &mut buffer[..])?;
            let (method, url, _protocol, query_string) = WebServer::parse_reqeust_line(request_line);

            if method != "GET" && method != "POST" {
                return WebServer::unimplemented(&mut socket);
            }

            let path = WebServer::url_to_path(&url);
            if path.is_none() {
                WebServer::discard_all_headers(&mut socket)?;
                return WebServer::not_found(&mut socket, &url);
            }

            let path = path.unwrap();
            if WebServer::file_executable(&path) || query_string.is_some() {
                return WebServer::execute_cgi(&mut socket, &path, &method, &query_string.unwrap_or_default());
            } else {
                WebServer::discard_all_headers(&mut socket)?;
                return WebServer::serve_file(&mut socket, &path);
            }
        };
        self.threadpool.execute(move || {
            let result = f();
            if let Err(e) =  result {
                eprintln!("Handle request from {} failed for {}", addr, e);
            } else {
                println!("Responsed request from {}", addr);
            }
        });
    }

    fn unimplemented(socket: &mut TcpStream) -> IOResult<()> {
        // 返回 501
        socket.write_all(b"HTTP/1.1 501 Method Not Implemented\r\n")?;
        socket.write_all(SERVER_STRING)?;
        socket.write_all(b"Content-Type: text/html\r\n")?;
        socket.write_all(b"\r\n")?;

        WebServer::write_file(socket, "assets/501.html")?;

        socket.flush()?;
        Ok(())
    }

    fn not_found(socket: &mut TcpStream, _url: &str) -> IOResult<()> {
        //返回404
        socket.write_all(b"HTTP/1.1 404 NOT FOUND\r\n")?;
        socket.write_all(SERVER_STRING)?;
        socket.write_all(b"Content-Type: text/html\r\n")?;
        socket.write_all(b"\r\n")?;

        WebServer::write_file(socket, "assets/404.html")?;

        socket.flush()?;
        Ok(())
    }

    fn cannot_exectute(socket: &mut TcpStream, _url: &str) -> IOResult<()> {
        //返回500
        socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\n")?;
        socket.write_all(SERVER_STRING)?;
        socket.write_all(b"Content-Type: text/html\r\n")?;
        socket.write_all(b"\r\n")?;

        WebServer::write_file(socket, "assets/500.html")?;

        socket.flush()?;
        Ok(())
    }

    fn serve_file(socket: &mut TcpStream, url: &str) -> IOResult<()> {
        //返回 200
        socket.write_all(b"HTTP/1.1 200 OK\r\n")?;
        socket.write_all(SERVER_STRING)?;
        socket.write_all(b"Content-Type: text/html\r\n")?;
        socket.write_all(b"\r\n")?;

        WebServer::write_file(socket, url)?;

        socket.flush()?;
        Ok(())
    }

    fn bad_request(socket: &mut TcpStream, _url: &str) -> IOResult<()> {
        //返回400
        socket.write_all(b"HTTP/1.1 400 Bad Request\r\n")?;
        socket.write_all(SERVER_STRING)?;
        socket.write_all(b"Content-Type: text/html\r\n")?;
        socket.write_all(b"\r\n")?;

        WebServer::write_file(socket, "assets/400.html")?;

        socket.flush()?;
        Ok(()) 
    }

    fn execute_cgi(socket: &mut TcpStream, url: &str, method: &str, query_string: &str) -> IOResult<()> {
        let mut command = Command::new(url);
        command.env("REQUEST_METHOD", method);

        let output;
        if method == "GET" {
            WebServer::discard_all_headers(socket)?;
            output = command.env("QUERY_STRING", query_string).output();
        } else {
            let mut buffer = [0;1024];

            let mut content_length = loop {
                let line = WebServer::read_line(socket, &mut buffer[..]);
                if let Err(e) = line {
                    return Err(e);
                }

                let line = line.unwrap();
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

            WebServer::discard_all_headers(socket)?;

            if content_length == -1 {
                return WebServer::bad_request(socket, url);
            }

            if let Ok(process) = command.env("CONTENT_LENGTH", content_length.to_string())
                                        .stdin(Stdio::piped())
                                        .stdout(Stdio::piped())
                                        .spawn() {
                let mut stdin = process.stdin.as_ref().unwrap();
                while content_length > 0 {
                    let read = if content_length > 1024 {1024} else {content_length};
                    content_length -= read;
                    
                    let read = read as usize;
                    socket.read_exact(&mut buffer[0..read])?;
                    stdin.write_all(&buffer[0..read])?;
                }

                output = process.wait_with_output();
            } else {
                return WebServer::cannot_exectute(socket, url);
            }
        }
        
        match output {
            Ok(output) if output.status.success() => {
                socket.write_all(b"HTTP/1.1 200 OK\r\n")?;
                socket.write_all(SERVER_STRING)?;
                socket.write_all(&output.stdout)?;
                socket.flush()?;
                Ok(())
            },
            _ => WebServer::cannot_exectute(socket, url),
        }
    }

    fn read_line<'a>(socket: &mut TcpStream, buf: &'a mut [u8]) -> IOResult<&'a [u8]> {
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
                Some(Ok(_)) => break,
                Some(Err(e)) => return Err(e),
                _ => {
                    // It must never reach here
                    assert!(false);
                    break;
                },
            }
        }

        Ok(&buf[0..i])
    }

    fn discard_all_headers(socket: &mut TcpStream) -> IOResult<()> {
        let mut buffer = [0; 1024];
        loop {
            let line = WebServer::read_line(socket, &mut buffer[..])?;
            if line.len() == 0 {
                break;
            }
        }

        Ok(())
    }

    fn write_file(socket: &mut TcpStream, url: &str) -> IOResult<()> {
        let mut f = fs::File::open(url)?;
        let mut buf: [u8; 1024] = [0; 1024];
        loop {
            let size = f.read(&mut buf)?;
            if size == 0 {
                break;
            }
            socket.write_all(&buf[0..size])?;
        }
        Ok(())
    }

    fn url_to_path(url: &str) -> Option<String> {
        let mut path = format!("assets{}", url);
        if path.ends_with("/") {
            path = path + "index.html";
        }

        let mut metadata = fs::metadata(&path);
        match metadata {
            Ok(ref data) if data.is_dir() => {
                path = path + "/index.html";
                metadata = fs::metadata(&path);
            }
            _ => (),
        };

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
        
        let mode = metadata.unwrap().permissions().mode();

        const IXUSR: u32 =  0o100;
        const IXGRP: u32 =  0o010;
        const IXOTH: u32 =  0o001;
        
        (mode & IXUSR) > 0 || (mode & IXGRP) > 0 || (mode & IXOTH) > 0
    }

    fn parse_reqeust_line(line: &[u8]) -> (String, String, String, Option<String>) {
        let line = line.iter()
                                .map(|c| {
                                    let c = *c as char;
                                    c.to_ascii_uppercase()
                                });

        let method: String = line.clone().take_while(|c| !c.is_whitespace()).collect();
        
        let skip = method.len() + 1;
        let mut url: String = line.clone().skip(skip).take_while(|c| !c.is_whitespace()).collect();

        let mut query_string: Option<String> = None;
        if let Some(pos) = url.find("?") {
            query_string = Some(url[pos+1..].to_string());
            url = url[..pos].to_string();
        }

        let skip = skip + url.len() + 1;
        let protocol: String = line.clone().skip(skip).take_while(|c| !c.is_whitespace()).collect();
        
        (method, url, protocol, query_string)
    }
}