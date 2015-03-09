#![feature(core)]

extern crate iron;
#[macro_use] extern crate lazy_static;
extern crate router;
extern crate "rustc-serialize" as rustc_serialize;
//extern crate static;

use std::error::Error;

use iron::{headers, status};
use iron::prelude::*;
use iron::mime::Mime;
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

#[derive(Debug)]
struct Nyi;

impl std::fmt::Display for Nyi {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt("not yet implemented", f)
    }
}

impl Error for Nyi {
    fn description(&self) -> &str {
        "not yet implemented"
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
            hs.set_raw("WWW-Authenticate", vec![b"Basic realm=\"Anmeldung fuer die WiW-Boerse\"".to_vec()]);
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

fn index(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "text/html".parse::<Mime>().unwrap(), format!(r#"<!DOCTYPE html>
<html>
    <head>
        {header}
    </head>
    <body>
        {nav}
        <div class="container" style="position: relative; top: 71px;">
            <div class="panel panel-default">
                {intro}
            </div>
            <div class="row">
                <div class="col-lg-6 col-sm-12">
                    <h2>Ich habe/biete</h2>
                    <table class="table table-responsive">
                        <thead>
                            <tr>
                                <th>Eingestellt von</th>
                                <th>Beschreibung</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td></td>
                                <td style="color: gray; font-style: italic;">Keine aktiven Angebote.</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <div class="col-lg-6 col-sm-12">
                    <h2>Wir suchen</h2>
                    <table class="table table-responsive">
                        <thead>
                            <tr>
                                <th>Eingestellt von</th>
                                <th>Beschreibung</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td></td>
                                <td style="color: gray; font-style: italic;">Keine aktiven Anfragen.</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </body>
</html>
"#,
        header=include_str!("../assets/header.html"),
        intro=include_str!("../assets/intro.html"),
        nav=include_str!("../assets/nav.html")
    ))))
}

fn logo(_: &mut Request) -> IronResult<Response> {
    Err(IronError {
        error: Box::new(Nyi),
        response: Response {
            status: Some(status::NotImplemented),
            headers: headers::Headers::new(),
            extensions: TypeMap::new(),
            body: None
        }
    })
}

fn main() {
    // route
    let mut router = Router::new();
    router.get("/", index);
    router.get("/logo.png", logo);
    // handle auth
    let mut chain = Chain::new(router);
    chain.link_before(check_auth);
    // serve
    Iron::new(chain).http("0.0.0.0:18800").unwrap();
}
