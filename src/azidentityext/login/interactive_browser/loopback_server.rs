use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use anyhow::Result;

pub struct LoopbackServer {
    pub success_template: String,
    pub error_template: String,
    pub listener: TcpListener,
}

impl LoopbackServer {
    pub fn new(port: u16, success_template: String, error_template: String) -> Result<Self> {
        let listener = TcpListener::bind(("localhost", port))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            success_template,
            error_template,
            listener,
        })
    }

    pub fn listen_for_code(self, timeout: Duration, state: &str) -> Result<String> {
        let start_time = Instant::now();

        let mut last_hint_time = Instant::now();
        let hint_interval = Duration::from_secs(30);

        loop {
            if start_time.elapsed() > timeout {
                anyhow::bail!("Timeout({:?}) waiting for authorization code", timeout);
            }

            if last_hint_time.elapsed() > hint_interval {
                println!("Still waiting for authentication response... (press Ctrl+C to cancel)");
                last_hint_time = Instant::now();
            }

            match self.listener.accept() {
                Ok((stream, addr)) => {
                    tracing::info!("Received connection from {addr}");
                    return self.handle_stream(stream, state);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    tracing::error!("Error accepting connection: {}", e);
                    return Err(e.into());
                }
            }
        }
    }

    fn handle_stream(&self, mut stream: std::net::TcpStream, state: &str) -> Result<String> {
        let mut buf_reader = BufReader::new(&stream);
        let mut request_line = String::new();
        if buf_reader.read_line(&mut request_line)? == 0 {
            anyhow::bail!("Connection closed by peer");
        }
        if request_line.ends_with("\r\n") {
            request_line.truncate(request_line.len() - 2);
        } else if request_line.ends_with('\n') {
            request_line.truncate(request_line.len() - 1);
        }
        let mut header_lines = vec![];
        loop {
            let mut line = String::new();
            let n = buf_reader.read_line(&mut line)?;
            if n == 0 || line == "\r\n" || line == "\n" {
                tracing::trace!("End of headers");
                break;
            }
            if line.ends_with("\r\n") {
                line.truncate(line.len() - 2);
            } else if line.ends_with('\n') {
                line.truncate(line.len() - 1);
            }
            header_lines.push(line);
        }
        let content_length = header_lines.iter().find_map(|line| {
            if line.to_lowercase().starts_with("content-length:") {
                Some(line.split(':').nth(1)?.trim().parse::<usize>())
            } else {
                None
            }
        });
        let content_length = content_length.transpose()?.unwrap_or(0);
        tracing::debug!("Content-Length: {}", content_length);
        // Read the body based on Content-Length
        // Since we don't know which line separators are used, we read the exact number of bytes
        let body_bytes = self.read_body_nonblocking(&mut buf_reader, content_length)?;
        let body = String::from_utf8_lossy(&body_bytes);
        let code_result = Self::get_code_from_body(&body, state);
        // TODO: customize the response page
        let response = code_result.as_ref().map(|_code| {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
                self.success_template.len(),
                self.success_template
            )
        }).unwrap_or_else(|_e| {
            format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
                self.error_template.len(),
                self.error_template
            )
        });
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        stream.shutdown(std::net::Shutdown::Both)?;
        code_result
    }

    fn read_body_nonblocking(&self, reader: &mut BufReader<&std::net::TcpStream>, content_length: usize) -> Result<Vec<u8>> {
        let mut body_bytes = vec![0; content_length];
        let mut bytes_read = 0;
        let timeout = Duration::from_secs(10); // TODO: make this configurable
        let start_time = Instant::now();

        while bytes_read < content_length {
            if start_time.elapsed() > timeout {
                anyhow::bail!("Timeout reading request body after {:?}", timeout);
            }
            match reader.read(&mut body_bytes[bytes_read..]) {
                Ok(0) => {
                    // Connection closed
                    anyhow::bail!("Connection closed while reading body");
                }
                Ok(n) => {
                    bytes_read += n;
                    tracing::debug!("Read {} bytes, total: {}/{}", n, bytes_read, content_length);
                    if bytes_read >= content_length {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tracing::debug!("WouldBlock error while reading body; retrying...");
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Ok(body_bytes)
    }
    
    fn get_code_from_body(body: &str, state: &str) -> Result<String> {
        let form_data = body.split('&').into_iter()
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    Some((key.to_string(), value.to_string()))
                } else {
                    None
                }
            })
            .collect::<std::collections::HashMap<String, String>>();
        if let Some(code) = form_data.get("code") {
            if let Some(loopback_state) = form_data.get("state") {
                if loopback_state != state {
                    anyhow::bail!("State mismatch: expected {}, got {}", state, loopback_state);
                }
                return Ok(code.to_string());
            }
            anyhow::bail!("Missing state in the response");
        } else if let Some(error) = form_data.get("error") {
            let error_description = form_data.get("error_description").map_or("", |s| s.as_str());
            let error_url = form_data.get("error_uri").map_or("", |s| s.as_str());
            anyhow::bail!("Error in authentication response: {} - {} ({})", error, error_description, error_url);
        } else {
            anyhow::bail!("No code or error in the authentication response");
        }
    }
}