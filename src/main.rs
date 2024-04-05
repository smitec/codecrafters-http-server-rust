// Uncomment this block to pass the first stage
use anyhow::{Error, Ok, Result};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

enum HttpMethod {
    GET,
}

fn parse_method(content: &str) -> Option<HttpMethod> {
    match content {
        "GET" => Some(HttpMethod::GET),
        _ => None,
    }
}

struct StartLine {
    method: HttpMethod,
    path: String,
    version: String,
}

async fn handle_connection(mut stream: TcpStream) -> Result<(), Error> {
    // Read from the stream.
    let mut buffer = [0_u8; 4096];

    // TODO: Partial reads?
    let read = stream.read(&mut buffer).await?;

    let content = std::str::from_utf8(&buffer).unwrap().to_string();

    let (start_line, headers) = content.split_once("\r\n").unwrap();
    let mut parts = start_line.splitn(3, ' ');

    // TODO: less lazy error handling
    let start_line = StartLine {
        method: parse_method(parts.next().unwrap()).unwrap(),
        path: parts.next().unwrap().to_string(),
        version: parts.next().unwrap().to_string(),
    };

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
async fn main() -> Result<(), Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_connection(stream).await.unwrap();
        });
    }
}
