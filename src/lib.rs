use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write};

pub struct WebServer {
    addr: String,
}

impl WebServer {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
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
        let mut buffer = [0; 1024];
        socket.read(&mut buffer).unwrap();
        println!("Client '{}' Request:\n\n{}", addr, String::from_utf8_lossy(&buffer));

        let response = "HTTP/1.1 200 OK\r\n\r\n";
        socket.write(response.as_bytes()).unwrap();
        socket.flush().unwrap();
    }
}