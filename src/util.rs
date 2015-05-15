use std::fmt;
use std::error::Error;

use iron::prelude::*;
use iron::{headers, status};
use iron::typemap::TypeMap;

use mysql::conn::MyOpts;

use rustc_serialize::json;

#[derive(RustcDecodable)]
pub struct ConfigMy {
    password: String
}

#[derive(RustcDecodable)]
pub struct Config {
    username: String,
    password: String,
    admin_name: String,
    admin_pass: String,
    mysql: ConfigMy
}

lazy_static! {
    pub static ref CONFIG: Config = json::decode(include_str!("../assets/config.json")).unwrap();
    pub static ref MY_OPTS: MyOpts = MyOpts {
        user: Some("wiw".to_string()),
        pass: Some(CONFIG.mysql.password.clone()),
        db_name: Some("wiwboerse".to_string()),
        ..Default::default()
    };
}

pub fn check_admin_auth(req: &mut Request) -> IronResult<()> {
    match req.headers.get::<headers::Authorization<headers::Basic>>() {
        Some(&headers::Authorization(headers::Basic { ref username, password: Some(ref password) })) => {
            if *username == CONFIG.admin_name && *password == CONFIG.admin_pass {
                Ok(())
            } else {
                Err(IronError::new(AuthError, (status::Unauthorized, "Zugriff nur für die Administration. Benutzername oder Passwort falsch.")))
            }
        }
        Some(&headers::Authorization(headers::Basic { username: _, password: None })) => {
            Err(IronError::new(AuthError, (status::Unauthorized, "Kein Passwort gefunden.")))
        }
        None => {
            let mut hs = headers::Headers::new();
            hs.set_raw("WWW-Authenticate", vec![b"Basic realm=\"Anmeldung fuer die WiW-Boerse (nur fuer die Administration)\"".to_vec()]);
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

pub fn check_auth(req: &mut Request) -> IronResult<()> {
    match req.headers.get::<headers::Authorization<headers::Basic>>() {
        Some(&headers::Authorization(headers::Basic { ref username, password: Some(ref password) })) => {
            if (*username == CONFIG.username && *password == CONFIG.password) || (*username == CONFIG.admin_name && *password == CONFIG.admin_pass) {
                Ok(())
            } else {
                Err(IronError::new(AuthError, (status::Unauthorized, "Benutzername oder Passwort falsch.")))
            }
        }
        Some(&headers::Authorization(headers::Basic { username: _, password: None })) => {
            Err(IronError::new(AuthError, (status::Unauthorized, "Kein Passwort gefunden.")))
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

macro_rules! errors {
    ($($name:ident($msg:expr);)*) => {
        $(
            #[derive(Debug)]
            pub struct $name;

            impl fmt::Display for $name {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::Display::fmt($msg, f)
                }
            }

            impl Error for $name {
                fn description(&self) -> &str {
                    $msg
                }
            }
        )*
    }
}

errors! {
    AuthError("authentication error");
    DbError("database error");
    InternalError("internal server error");
    Nyi("not yet implemented");
}
