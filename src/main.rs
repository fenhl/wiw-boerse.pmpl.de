extern crate iron;
#[macro_use] extern crate lazy_static;
extern crate mysql;
extern crate router;
extern crate rustc_serialize as rustc_serialize;
//extern crate static;

use std::default::Default;
use std::error::Error;

use iron::{headers, status};
use iron::prelude::*;
use iron::mime::Mime;
use iron::typemap::TypeMap;

use mysql::conn::{MyConn, MyOpts};
use mysql::error::MyError;
use mysql::value::FromValue;

use router::Router;

use rustc_serialize::json;

#[derive(RustcDecodable)]
struct ConfigMy {
    password: String
}

#[derive(RustcDecodable)]
struct Config {
    username: String,
    password: String,
    mysql: ConfigMy
}

lazy_static! {
    static ref CONFIG: Config = json::decode(include_str!("../assets/config.json")).unwrap();
    static ref MY_OPTS: MyOpts = MyOpts {
        user: Some("wiw".to_string()),
        pass: Some(CONFIG.mysql.password.clone()),
        db_name: Some("wiwboerse".to_string()),
        ..Default::default()
    };
}

macro_rules! errors {
    ($($name:ident($msg:expr);)*) => {
        $(
            #[derive(Debug)]
            struct $name;

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    std::fmt::Display::fmt($msg, f)
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
    Nyi("not yet implemented");
}

fn check_auth(req: &mut Request) -> IronResult<()> {
    match req.headers.get::<headers::Authorization<headers::Basic>>() {
        Some(&headers::Authorization(headers::Basic { ref username, password: Some(ref password) })) => {
            if *username == CONFIG.username && *password == CONFIG.password {
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

fn format_offers(conn: &mut mysql::conn::MyConn) -> Result<String, MyError> {
    let offers = try!(conn.query("SELECT * FROM offers")).collect::<Vec<_>>();
    Ok(if offers.len() > 0 {
        offers.into_iter().map(|row| match row {
            Ok(values) => {
                // name, description, phone, mail
                format!(
                    r#"
<tr>
    <td>{name}{mail}{phone}</td>
    <td>{description}</td>
</tr>
                    "#,
                    name=String::from_value(&values[0]),
                    description=String::from_value(&values[1]),
                    phone=match Option::<String>::from_value(&values[2]) { Some(phone) => format!(r#"<br /><a href="tel:{0}">{0}</a>"#, phone), None => "".to_string() },
                    mail=match Option::<String>::from_value(&values[3]) { Some(mail) => format!(r#"<br /><a href="mailto:{0}">{0}</a>"#, mail), None => "".to_string() },
                )
            }
            Err(_) => format!(
                r#"
<tr>
    <td></td>
    <td style="color: gray; font-style: italic;">Fehlerhaftes Angebot.</td>
</tr>
                "#
            )
        }).fold("".to_string(), |text, row| text + &row)
    } else {
        format!(r#"
<tr>
    <td></td>
    <td style="color: gray; font-style: italic;">Keine aktiven Angebote.</td>
</tr>
        "#)
    })
}

fn format_requests(conn: &mut mysql::conn::MyConn) -> Result<String, MyError> {
    let requests = try!(conn.query("SELECT * FROM requests")).collect::<Vec<_>>();
    Ok(if requests.len() > 0 {
        requests.into_iter().map(|row| match row {
            Ok(values) => {
                // name, description, phone, mail
                format!(
                    r#"
<tr>
    <td>{name}{mail}{phone}</td>
    <td>{description}</td>
</tr>
                    "#,
                    name=String::from_value(&values[0]),
                    description=String::from_value(&values[1]),
                    phone=match Option::<String>::from_value(&values[2]) { Some(phone) => format!(r#"<br /><a href="tel:{0}">{0}</a>"#, phone), None => "".to_string() },
                    mail=match Option::<String>::from_value(&values[3]) { Some(mail) => format!(r#"<br /><a href="mailto:{0}">{0}</a>"#, mail), None => "".to_string() },
                )
            }
            Err(_) => format!(
                r#"
<tr>
    <td></td>
    <td style="color: gray; font-style: italic;">Fehlerhafte Anfrage.</td>
</tr>
                "#
            )
        }).fold("".to_string(), |text, row| text + &row)
    } else {
        format!(r#"
<tr>
    <td></td>
    <td style="color: gray; font-style: italic;">Keine aktiven Anfragen.</td>
</tr>
        "#)
    })
}

fn index(_: &mut Request) -> IronResult<Response> {
    let mut conn = try!(MyConn::new(MY_OPTS.clone()).map_err(|_| IronError::new(DbError, (status::InternalServerError, "Konnte die Datenbank nicht laden. Bitte kontaktieren Sie die Administration."))));
    Ok(Response::with((status::Ok, "text/html".parse::<Mime>().unwrap(), format!(
        r#"
<!DOCTYPE html>
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
                    <h2>Ich habe/biete <a href="/biete/neu" class="btn btn-success"><i class="fa fa-plus"></i> Angebot hinzufügen</a></h2>
                    <table class="table table-responsive">
                        <thead>
                            <tr>
                                <th>Eingestellt von</th>
                                <th>Beschreibung</th>
                            </tr>
                        </thead>
                        <tbody>
                            {offers}
                        </tbody>
                    </table>
                </div>
                <div class="col-lg-6 col-sm-12">
                    <h2>Ich suche <a href="/suche/neu" class="btn btn-success"><i class="fa fa-plus"></i> Anfrage hinzufügen</a></h2>
                    <table class="table table-responsive">
                        <thead>
                            <tr>
                                <th>Eingestellt von</th>
                                <th>Beschreibung</th>
                            </tr>
                        </thead>
                        <tbody>
                            {requests}
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
        nav=include_str!("../assets/nav.html"),
        offers=try!(format_offers(&mut conn).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))),
        requests=try!(format_requests(&mut conn).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))))
    ))))
}

fn nyi(_: &mut Request) -> IronResult<Response> {
    Err(IronError::new(Nyi, (status::NotImplemented, "Diese Seite ist noch nicht verfügbar, bitte versuchen Sie es später erneut.")))
}

fn main() {
    // route
    let mut router = Router::new();
    router.get("/", index);
    router.get("/logo.png", nyi);
    router.get("/suche/neu", nyi);
    router.get("/suche/:id", nyi);
    router.get("/biete/neu", nyi);
    router.get("/biete/:id", nyi);
    // handle auth
    let mut chain = Chain::new(router);
    chain.link_before(check_auth);
    // serve
    Iron::new(chain).http("0.0.0.0:18800").unwrap();
}
