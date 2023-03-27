use core::str;
use reqwest::{blocking::Client, header::USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{fs::File, io, process::exit};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use urlencoding::decode;

#[derive(Serialize, Deserialize)]
struct PostBody {
    url: String,
}

pub static USER_AGENT_STRING: &str = "Mozilla/5.0 (Linux; Android 6.0.1; Nexus 5X Build/MMB29P) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/W.X.Y.Z Mobile Safari/537.36 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
pub static SERVER_ADDRESS: &str = "127.0.0.1:1337";

fn serve_404(req: Request) -> io::Result<()> {
    req.respond(Response::from_string("404").with_status_code(StatusCode(404)))
}

fn serve_500(req: Request, e: Box<dyn std::error::Error>) -> io::Result<()> {
    eprintln!("ERROR: {e}");
    req.respond(Response::from_string(format!("500: {e}")).with_status_code(StatusCode(500)))
}

fn serve_400(req: Request, msg: &str) -> io::Result<()> {
    req.respond(Response::from_string(format!("500: {msg}")).with_status_code(StatusCode(500)))
}

fn serve_static_file(req: Request, path: &str, type_: &str) -> io::Result<()> {
    let header = Header::from_bytes("Content-Type", type_)
        .map_err(|e| eprintln!("ERROR: failed to set content type: {e:?}"))
        .unwrap();

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ERROR: could not serve file {path}: {e}");
            if e.kind() == io::ErrorKind::NotFound {
                return serve_404(req);
            }
            return serve_500(req, Box::new(e));
        }
    };

    req.respond(Response::from_file(file).with_header(header))
}

fn serve_article(mut req: Request) -> io::Result<()> {
    let mut buf = String::new();
    if let Err(e) = req.as_reader().read_to_string(&mut buf) {
        eprintln!("ERROR: could not read the request body: {e:?}");
        return serve_400(req, "Body must be a valid UTF-8 string");
    }

    let url = match decode(&buf[4..buf.len()]) {
        Ok(url) => url.to_string(),
        Err(e) => return serve_500(req, Box::new(e)),
    };
    println!("INFO: fetching {url} ...");

    let client = Client::new();
    match client.get(url).header(USER_AGENT, USER_AGENT_STRING).send() {
        Ok(res) => {
            let text = res
                .text()
                .map_err(|e| eprintln!("ERROR: failed to read response text: {e:?}"))
                .unwrap();

            let header = Header::from_bytes("Content-Type", "text/html; charset=utf-8")
                .map_err(|e| eprintln!("ERROR: failed to set content type: {e:?}"))
                .unwrap();

            return req.respond(Response::from_string(text).with_header(header));
        }
        Err(e) => {
            return serve_500(req, Box::new(e));
        }
    }
}

fn handle_request(req: Request) -> io::Result<()> {
    match (req.method(), req.url()) {
        (Method::Get, "/") => serve_static_file(req, "index.html", "text/html; charset=utf-8")?,
        (Method::Get, "/main.css") => {
            serve_static_file(req, "main.css", "text/css; charset=utf-8")?
        }
        (Method::Post, "/a") => serve_article(req)?,
        _ => serve_404(req)?,
    }
    Ok(())
}

fn main() {
    let server = match Server::http(SERVER_ADDRESS) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: Failed to start server at {SERVER_ADDRESS}: {e:?}");
            exit(1);
        }
    };
    println!("INFO: Listening at http://{SERVER_ADDRESS}/");

    for req in server.incoming_requests() {
        handle_request(req)
            .map_err(|e| {
                eprintln!("ERROR: Failed to handle request: {e}");
            })
            .ok();
    }
}
