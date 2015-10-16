#![feature(plugin)]
#![plugin(regex_macros)]

extern crate iron;
#[macro_use] extern crate lazy_static;
extern crate mysql;
extern crate plugin;
extern crate regex;
extern crate router;
extern crate rustc_serialize as rustc_serialize;
//extern crate static;
extern crate urlencoded;

mod admin;
mod util;

use std::str::FromStr;

use iron::status;
use iron::prelude::*;
use iron::mime::Mime;

use mysql::conn::MyConn;
use mysql::error::MyError;
use mysql::value::FromValue;

use router::Router;

use urlencoded::UrlEncodedBody;

use admin::IsAdmin;
use util::{DbError, InternalError, MY_OPTS, Nyi, check_admin_auth, check_auth};

fn mysql_escape<S: AsRef<str>>(s: S) -> String {
    format!("\"{}\"", regex!("\0|\n|\r|\\|'|\"|\x1a").replace_all(s.as_ref(), "\\$0"))
}

fn mysql_escape_nullable<S: AsRef<str>>(s: S) -> String {
    if s.as_ref() == "" {
        "NULL".to_owned()
    } else {
        mysql_escape(s)
    }
}

fn mysql_connection() -> IronResult<MyConn> {
    MyConn::new(MY_OPTS.clone()).map_err(|_| IronError::new(DbError, (status::InternalServerError, "Konnte die Datenbank nicht laden. Bitte kontaktieren Sie die Administration.")))
}

fn format_offers(conn: &mut mysql::conn::MyConn, is_admin: bool) -> Result<String, MyError> {
    let offers = try!(conn.query("SELECT * FROM offers")).collect::<Vec<_>>();
    Ok(if offers.len() > 0 {
        offers.into_iter().map(|row| match row {
            Ok(values) => {
                // name, description, phone, mail, id
                format!(
                    r#"
<tr>
    <td>{name}{mail}{phone}</td>
    <td>{edit_buttons}{description}</td>
</tr>
                    "#,
                    name=String::from_value(values[0].clone()),
                    description=String::from_value(values[1].clone()).replace("\n", "<br />"),
                    phone=match Option::<String>::from_value(values[2].clone()) { Some(phone) => format!(r#"<br /><a href="tel:{0}">{0}</a>"#, phone), None => "".to_owned() },
                    mail=match Option::<String>::from_value(values[3].clone()) { Some(mail) => format!(r#"<br /><a href="mailto:{0}">{0}</a>"#, mail), None => "".to_owned() },
                    edit_buttons=if is_admin { match Option::<i32>::from_value(values[4].clone()) { Some(i) => format!(r#"<div style="float: right;"><a href="/biete/{}/loeschen" class="btn btn-danger"><i class="fa fa-trash-o"></i></a></div>"#, i), None => "".to_owned() } } else { "".to_owned() }
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

fn format_requests(conn: &mut mysql::conn::MyConn, is_admin: bool) -> Result<String, MyError> {
    let requests = try!(conn.query("SELECT * FROM requests")).collect::<Vec<_>>();
    Ok(if requests.len() > 0 {
        requests.into_iter().map(|row| match row {
            Ok(values) => {
                // name, description, phone, mail, id
                format!(
                    r#"
<tr>
    <td>{name}{mail}{phone}</td>
    <td>{edit_buttons}{description}</td>
</tr>
                    "#,
                    name=String::from_value(values[0].clone()),
                    description=String::from_value(values[1].clone()).replace("\n", "<br />"),
                    phone=match Option::<String>::from_value(values[2].clone()) { Some(phone) => format!(r#"<br /><a href="tel:{0}">{0}</a>"#, phone), None => "".to_owned() },
                    mail=match Option::<String>::from_value(values[3].clone()) { Some(mail) => format!(r#"<br /><a href="mailto:{0}">{0}</a>"#, mail), None => "".to_owned() },
                    edit_buttons=if is_admin { match Option::<i32>::from_value(values[4].clone()) { Some(i) => format!(r#"<div style="float: right;"><a href="/suche/{}/loeschen" class="btn btn-danger"><i class="fa fa-trash-o"></i></a></div>"#, i), None => "".to_owned() } } else { "".to_owned() }
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

fn index(req: &mut Request) -> IronResult<Response> {
    let is_admin = req.get::<IsAdmin>().unwrap_or(false);
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
        offers=try!(format_offers(&mut conn, is_admin).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))),
        requests=try!(format_requests(&mut conn, is_admin).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))))
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
            <h2>Neues Angebot</h2>
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
    let phone = mysql_escape_nullable(&form_data["phone"][0]);
    let mail = mysql_escape_nullable(&form_data["mail"][0]);
    if phone == "NULL" && mail == "NULL" { return Err(nyi()) }
    let mut conn = try!(mysql_connection());
    try!(conn.query(format!("INSERT INTO offers (name, description, phone, mail) VALUES ({}, {}, {}, {})", name, description, phone, mail)).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))));
    Ok(Response::with((status::Ok, "Ihr Angebot wurde eingetragen.")))
}

fn new_request_page(_: &mut Request) -> IronResult<Response> {
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
            <h2>Neue Anfrage</h2>
            <form class="form-horizontal" action="/suche/neu" method="post" enctype="application/x-www-form-urlencoded">
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
                        <textarea rows="3" class="form-control" name="description" id="description" placeholder="Beschreiben Sie die Anfrage hier."></textarea>
                    </div>
                </div>
                <div class="form-group">
                    <div class="col-sm-offset-2 col-sm-10">
                        <button type="submit" class="btn btn-primary">Anfrage einreichen</button>
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

fn add_request(req: &mut Request) -> IronResult<Response> {
    let form_data = try!(req.get_ref::<UrlEncodedBody>().map_err(|e| IronError::new(e, (status::BadRequest, "Fehlender Formularinhalt. Bitte füllen Sie das Formular erneut aus."))));
    let name = mysql_escape(&form_data["name"][0]);
    if name.len() == 0 { return Err(nyi()) }
    let description = mysql_escape(&form_data["description"][0]);
    if description.len() == 0 { return Err(nyi()) }
    let phone = mysql_escape_nullable(&form_data["phone"][0]);
    let mail = mysql_escape_nullable(&form_data["mail"][0]);
    if phone == "NULL" && mail == "NULL" { return Err(nyi()) }
    let mut conn = try!(mysql_connection());
    try!(conn.query(format!("INSERT INTO requests (name, description, phone, mail) VALUES ({}, {}, {}, {})", name, description, phone, mail)).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))));
    Ok(Response::with((status::Ok, "Ihre Anfrage wurde eingetragen.")))
}

fn del_offer(req: &mut Request) -> IronResult<Response> {
    let mut conn = try!(mysql_connection());
    let id_str = try!(try!(req.extensions.get::<Router>().ok_or(IronError::new(InternalError, (status::InternalServerError, "Fehler beim Lesen der Angebotsnummer.")))).find("id").ok_or(IronError::new(InternalError, (status::InternalServerError, "Fehler beim Lesen der Angebotsnummer."))));
    let id = try!(i32::from_str(id_str).map_err(|e| IronError::new(e, (status::BadRequest, format!("Die Angebotsnummer {:?} ist keine Nummer.", id_str)))));
    try!(conn.query(format!("DELETE FROM offers WHERE id={}", id)).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))));
    Ok(Response::with((status::Ok, "Das Angebot wurde gelöscht.")))
}

fn del_request(req: &mut Request) -> IronResult<Response> {
    let mut conn = try!(mysql_connection());
    let id_str = try!(try!(req.extensions.get::<Router>().ok_or(IronError::new(InternalError, (status::InternalServerError, "Fehler beim Lesen der Anfragennummer.")))).find("id").ok_or(IronError::new(InternalError, (status::InternalServerError, "Fehler beim Lesen der Anfragennummer."))));
    let id = try!(i32::from_str(id_str).map_err(|e| IronError::new(e, (status::BadRequest, format!("Die Anfragennummer {:?} ist keine Nummer.", id_str)))));
    try!(conn.query(format!("DELETE FROM requests WHERE id={}", id)).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))));
    Ok(Response::with((status::Ok, "Die Anfrage wurde gelöscht.")))
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
    router.get("/suche/neu", new_request_page);
    router.post("/suche/neu", add_request);
    router.get("/suche/:id", nyi_handler);
    router.get("/biete/neu", new_offer_page);
    router.post("/biete/neu", add_offer);
    router.get("/biete/:id", nyi_handler);
    // handle admin auth
    let mut del_request_chain = Chain::new(del_request);
    del_request_chain.link_before(check_admin_auth);
    router.get("/suche/:id/loeschen", del_request_chain);
    let mut del_offer_chain = Chain::new(del_offer);
    del_offer_chain.link_before(check_admin_auth);
    router.get("/biete/:id/loeschen", del_offer_chain);
    // handle auth
    let mut chain = Chain::new(router);
    chain.link_before(check_auth);
    // serve
    Iron::new(chain).http("0.0.0.0:18800").unwrap();
}
