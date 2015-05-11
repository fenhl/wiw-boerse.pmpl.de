#![feature(plugin)]
#![plugin(regex_macros)]

extern crate iron;
#[macro_use] extern crate lazy_static;
extern crate mysql;
extern crate regex;
extern crate router;
extern crate rustc_serialize as rustc_serialize;
//extern crate static;
extern crate urlencoded;

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

use urlencoded::UrlEncodedBody;

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

fn mysql_escape<S: AsRef<str>>(s: S) -> String {
    regex!("\0|\n|\r|\\|'|\"|\x1a").replace_all(s.as_ref(), "\\$0")
}

fn mysql_connection() -> IronResult<MyConn> {
    MyConn::new(MY_OPTS.clone()).map_err(|_| IronError::new(DbError, (status::InternalServerError, "Konnte die Datenbank nicht laden. Bitte kontaktieren Sie die Administration.")))
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
    let mut conn = try!(mysql_connection());
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

fn new_offer_page(_: &mut Request) -> IronResult<Response> {
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
            <form class="form-horizontal" action="/biete/neu" method="post" enctype="application/x-www-form-urlencoded">
                <div class="form-group">
                    <label for="name" class="col-sm-2 control-label">Eingestellt von</label>
                    <div class="col-sm-10">
                        <input type="text" class="form-control" name="name" id="name" placeholder="Ihr Name" />
                    </div>
                </div>
                <div class="form-group">
                    <label for="mail" class="col-sm-2 control-label">E-Mail</label>
                    <div class="col-sm-10">
                        <input type="email" class="form-control" name="mail" id="mail" placeholder="Eine Mailadresse zur Kontaktaufnahme. Wird in der Liste angezeigt." />
                    </div>
                </div>
                <div class="form-group">
                    <label for="phone" class="col-sm-2 control-label">Telefon</label>
                    <div class="col-sm-10">
                        <input type="tel" class="form-control" name="phone" id="phone" placeholder="Eine Telefonnummer zur Kontaktaufnahme. Wird in der Liste angezeigt." />
                        <p class="help-block">Bitte geben Sie Mailadresse und/oder Telefonnummer an.</p>
                    </div>
                </div>
                <div class="form-group">
                    <label for="description" class="col-sm-2 control-label">Beschreibung</label>
                    <div class="col-sm-10">
                        <textarea rows="3" class="form-control" name="description" id="description" placeholder="Beschreiben Sie das Angebot hier."></textarea>
                    </div>
                </div>
                <div class="form-group">
                    <div class="col-sm-offset-2 col-sm-10">
                        <button type="submit" class="btn btn-primary">Angebot einreichen</button>
                    </div>
                </div>
            </form>
        </div>
    </body>
</html>
        "#,
        header=include_str!("../assets/header.html"),
        nav=include_str!("../assets/nav.html")
    ))))
}

fn add_offer(req: &mut Request) -> IronResult<Response> {
    let form_data = try!(req.get_ref::<UrlEncodedBody>().map_err(|e| IronError::new(e, (status::BadRequest, "Fehlender Formularinhalt. Bitte füllen Sie das Formular erneut aus."))));
    let name = mysql_escape(&form_data["name"][0]);
    if name.len() == 0 { return Err(nyi()) }
    let description = mysql_escape(&form_data["description"][0]);
    if description.len() == 0 { return Err(nyi()) }
    let phone = mysql_escape(&form_data["phone"][0]);
    let mail = mysql_escape(&form_data["mail"][0]);
    if phone.len() == 0 && mail.len() == 0 { return Err(nyi()) }
    let mut conn = try!(mysql_connection());
    let response = try!(conn.query(format!("INSERT INTO offers (name, description, phone, mail) VALUES ({}, {}, {}, {})", mysql_escape(name), mysql_escape(description), mysql_escape(phone), mysql_escape(mail))).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))).collect::<Vec<_>>();
    Ok(Response::with((status::NotImplemented, format!("Diese Seite befindet sich im Aufbau.\nTest:\n{:?}", response))))
    // Ok(Response::with((status::Ok, "Ihr Angebot wurde eingetragen.")))
}

fn nyi() -> IronError {
    IronError::new(Nyi, (status::NotImplemented, "Diese Seite ist noch nicht verfügbar, bitte versuchen Sie es später erneut."))
}

fn nyi_handler(_: &mut Request) -> IronResult<Response> {
    Err(nyi())
}

fn main() {
    // route
    let mut router = Router::new();
    router.get("/", index);
    router.get("/logo.png", nyi_handler);
    router.get("/suche/neu", nyi_handler);
    router.post("/suche/neu", nyi_handler);
    router.get("/suche/:id", nyi_handler);
    router.get("/biete/neu", new_offer_page);
    router.post("/biete/neu", add_offer);
    router.get("/biete/:id", nyi_handler);
    // handle auth
    let mut chain = Chain::new(router);
    chain.link_before(check_auth);
    // serve
    Iron::new(chain).http("0.0.0.0:18800").unwrap();
}
