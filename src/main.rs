#![feature(core)]

extern crate iron;
#[macro_use] extern crate lazy_static;
extern crate router;
extern crate "rustc-serialize" as rustc_serialize;

use std::error::Error;

use iron::{headers, status};
use iron::prelude::*;
use iron::typemap::TypeMap;
use router::Router;

use rustc_serialize::json;

#[derive(RustcDecodable)]
struct Config {
    username: String,
    password: String
}

lazy_static! {
    static ref CONFIG: Config = json::decode(include_str!("../assets/config.json")).unwrap();
}

#[derive(Debug)]
struct AuthError;

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt("authentication error", f)
    }
}

impl Error for AuthError {
    fn description(&self) -> &str {
        "authentication error"
    }
}

fn check_auth(req: &mut Request) -> IronResult<()> {
    match req.headers.get::<headers::Authorization<headers::Basic>>() {
        Some(&headers::Authorization(headers::Basic { ref username, password: Some(ref password) })) => {
            if *username == CONFIG.username && *password == CONFIG.password {
                Ok(())
            } else {
                Err(IronError {
                    error: Box::new(AuthError),
                    response: Response::with((status::Unauthorized, "Benutzername oder Passwort falsch."))
                })
            }
        }
        Some(&headers::Authorization(headers::Basic { username: _, password: None })) => {
            Err(IronError {
                error: Box::new(AuthError),
                response: Response::with((status::Unauthorized, "Kein Passwort gefunden."))
            })
        }
        None => {
            let mut hs = headers::Headers::new();
            hs.set_raw("WWW-Authenticate", vec![b"Basic realm=\"main\"".to_vec()]);
            Err(IronError {
                error: Box::new(AuthError),
                response: Response {
                    status: Some(status::Unauthorized),
                    headers: hs,
                    extensions: TypeMap::new(),
                    body: None
                }
            })
        }
    }
}

fn hello_world(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Hello, world!")))
}

fn main() {
    // route
    let mut router = Router::new();
    router.get("/", hello_world);
    // handle auth
    let mut chain = Chain::new(router);
    chain.link_before(check_auth);
    // serve
    Iron::new(chain).http("0.0.0.0:18800").unwrap();
}