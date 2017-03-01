use std::{fmt, string};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

use chrono::prelude::*;

use iron::prelude::*;
use iron::{headers, status};
use iron::typemap::{Key, TypeMap};

use plugin;

use rustc_serialize::json;

#[derive(Debug, Clone, Copy)]
pub struct IsTls;

impl Key for IsTls {
    type Value = bool;
}

impl<'a, 'b> plugin::Plugin<Request<'a, 'b>> for IsTls {
    type Error = IsTlsError;

    fn eval(req: &mut Request) -> Result<bool, IsTlsError> {
        Ok(match req.headers.get_raw("X-Fenhl-TLS") {
            Some(header_bytes) => {
                if header_bytes.len() != 1 {
                    return Err(IsTlsError);
                }
                match &*try!(String::from_utf8(header_bytes[0].clone())) {
                    "on" => true,
                    "" => false,
                    _ => { return Err(IsTlsError); }
                }
            }
            None => false
        })
    }
}

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

#[derive(RustcDecodable)]
struct RebootConfig {
    schedule: Option<DateTime<UTC>>
}

lazy_static! {
    pub static ref CONFIG: Config = json::decode(include_str!("../assets/config.json")).unwrap();
    pub static ref MY_OPTS: ::mysql::Opts = {
        let mut builder = ::mysql::OptsBuilder::new();
        builder.user(Some("wiw"))
            .pass(Some(CONFIG.mysql.password.clone()))
            .db_name(Some("wiwboerse"));
        builder.into()
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

pub fn reboot_time() -> Option<DateTime<UTC>> {
    if let Ok(mut f) = File::open("/opt/dev/reboot.json") {
        let mut config_buf = String::default();
        if f.read_to_string(&mut config_buf).is_err() {
            return None;
        }
        if let Ok(conf) = json::decode::<RebootConfig>(&config_buf) {
            conf.schedule
        } else {
            None
        }
    } else {
        None
    }
}

macro_rules! errors {
    ($($name:ident($msg:expr);)*) => {
        $(
            #[derive(Debug, Clone, Copy)]
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

            impl From<$name> for IronError {
                fn from(err: $name) -> IronError {
                    IronError::new(err, (status::BadRequest, $msg))
                }
            }
        )*
    }
}

errors! {
    AuthError("authentication error");
    DbError("database error");
    InternalError("internal server error");
    IsTlsError("failed to determine encryption status");
    Nyi("not yet implemented");
}

impl From<string::FromUtf8Error> for IsTlsError {
    fn from(_: string::FromUtf8Error) -> IsTlsError { IsTlsError }
}
