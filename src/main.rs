use std::env;
use std::io::Write;
use std::sync::Arc;
use std::{fs::File, io::Read};

use anyhow::Result;

use nom::AsBytes;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

enum HttpMethod {
    GET,
    POST,
}

fn parse_method(content: &str) -> Option<HttpMethod> {
    match content {
        "GET" => Some(HttpMethod::GET),
        "POST" => Some(HttpMethod::POST),
        _ => None,
    }
}

struct StartLine {
    method: HttpMethod,
    path: String,
    version: String,
}

async fn handle_connection(
    mut stream: TcpStream,
    directory: Arc<String>,
) -> Result<(), anyhow::Error> {
    // Read from the stream.
    let mut buffer = [0_u8; 4096];

    let read = stream.read(&mut buffer).await?;

    let content = std::str::from_utf8(&buffer[0..read]).unwrap().to_string();

    let (start_line, headers) = content.split_once("\r\n").unwrap();
    let (headers, body) = headers.split_once("\r\n\r\n").unwrap();
    let mut parts = start_line.splitn(3, ' ');

    let start_line = StartLine {
        method: parse_method(parts.next().unwrap()).unwrap(),
        path: parts.next().unwrap().to_string(),
        version: parts.next().unwrap().to_string(),
    };

    // Handle the files path.
    if start_line.path.starts_with("/files/") {
        let (_, filename) = start_line.path.split_once("/files/").unwrap();
        match start_line.method {
            HttpMethod::GET => {
                let file = File::open(format!("{}/{}", directory, filename));

                match file {
                    Ok(mut file) => {
                        let mut s = String::new();
                        file.read_to_string(&mut s)?;
                        stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length:{}\r\n\r\n{}",
                        s.len(),
                        s
                    )
                    .as_bytes(),
                )
                .await
                .expect("Couldn't write bytes!");
                    }
                    Err(_) => {
                        stream
                            .write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                            .await
                            .expect("Couldn't write bytes!");
                    }
                }
            }
            HttpMethod::POST => {
                let mut file = File::create(format!("{}/{}", directory, filename)).unwrap();
                file.write_all(body.as_bytes()).unwrap();
                stream
                    .write_all("HTTP/1.1 201 Created\r\n\r\n".as_bytes())
                    .await
                    .expect("Couldn't write bytes!");
            }
        }
    }

    // Handle the echo path.
    if start_line.path.starts_with("/echo/") {
        let (_, response) = start_line.path.split_once("/echo/").unwrap();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length:{}\r\n\r\n{}",
                    response.len(),
                    response
                )
                .as_bytes(),
            )
            .await
            .expect("Couldn't write bytes!");
    }

    match start_line.path.as_str() {
        "/user-agent" => {
            // Send back the user agent
            for line in headers.split("\r\n") {
                if line.starts_with("User-Agent: ") {
                    let (_, response) = line.split_once("User-Agent: ").unwrap();
                    stream
                        .write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length:{}\r\n\r\n{}", response.len(), response).as_bytes())
                        .await
                        .expect("Couldn't write bytes!");
                    break;
                }
            }
        }
        "/" => {
            stream
                .write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes())
                .await
                .expect("Couldn't write bytes!");
        }
        _ => {
            stream
                .write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                .await
                .expect("Couldn't write bytes!");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    let mut directory: Arc<String> = Arc::new("./".to_string());
    for (i, arg) in args.iter().enumerate() {
        if arg == "--directory" {
            if let Some(next) = args.get(i + 1) {
                directory = Arc::new(next.clone());
                break;
            }
        }
    }

    // Uncomment this block to pass the first stage
    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let d_clone = directory.clone();
        tokio::spawn(async move {
            handle_connection(stream, d_clone).await.unwrap();
        });
    }
}
