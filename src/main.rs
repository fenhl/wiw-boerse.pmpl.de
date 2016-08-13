extern crate iron;
#[macro_use] extern crate lazy_static;
extern crate mysql;
extern crate plugin;
extern crate regex;
extern crate router;
extern crate rustc_serialize as rustc_serialize;
extern crate staticfile;
extern crate urlencoded;
extern crate wiw;

mod admin;
mod entry;
mod util;

use std::path::Path;
use std::str::FromStr;

use iron::status;
use iron::prelude::*;
use iron::mime::Mime;

use mysql::FromValue;

use regex::Regex;

use router::Router;

use staticfile::Static;

use urlencoded::UrlEncodedBody;

use admin::IsAdmin;
use util::{DbError, InternalError, MY_OPTS, Nyi, check_admin_auth, check_auth};

struct NoticePositions {
    index: bool,
    create_offer: bool,
    create_request: bool
}

impl NoticePositions {
    fn any(&self) -> bool {
        self.index || self.create_offer || self.create_request
    }

    fn contains(&self, pos: Option<entry::Type>) -> bool {
        match pos {
            Some(entry::Type::Offer) => self.create_offer,
            Some(entry::Type::Request) => self.create_request,
            None => self.index
        }
    }

    fn mysql_string(&self) -> String {
        let mut position_strings = Vec::default();
        if self.index { position_strings.push("index"); }
        if self.create_offer { position_strings.push("create_offer"); }
        if self.create_request { position_strings.push("create_request"); }
        position_strings.join(",")
    }
}

impl<S: AsRef<str>> From<S> for NoticePositions {
    fn from(s: S) -> NoticePositions {
        NoticePositions {
            index: s.as_ref().contains("index"),
            create_offer: s.as_ref().contains("create_offer"),
            create_request: s.as_ref().contains("create_request")
        }
    }
}

fn mysql_escape<S: AsRef<str>>(s: S) -> String {
    format!("\"{}\"", Regex::new("\0|\n|\r|\\|'|\"|\x1a").unwrap().replace_all(s.as_ref(), "\\$0"))
}

fn mysql_escape_nullable<S: AsRef<str>>(s: S) -> String {
    if s.as_ref() == "" {
        "NULL".to_owned()
    } else {
        mysql_escape(s)
    }
}

fn mysql_connection() -> IronResult<::mysql::Conn> {
    ::mysql::Conn::new(MY_OPTS.clone()).map_err(|_| IronError::new(DbError, (status::InternalServerError, "Konnte die Datenbank nicht laden. Bitte kontaktieren Sie die Administration.")))
}

fn format_notices(entry_type: Option<entry::Type>, conn: &mut ::mysql::Conn, is_admin: bool) -> Result<String, ::mysql::Error> {
    let entries = try!(conn.query("SELECT * FROM notices")).collect::<Vec<_>>();
    Ok(entries.into_iter().filter_map(|row| match row {
        Ok(values) => {
            if NoticePositions::from(String::from_value(values[1].clone())).contains(entry_type) {
                Some(format!(
                    r#"<div class="alert alert-info">{edit_buttons}{text}</div>"#,
                    text=String::from_value(values[2].clone()),
                    edit_buttons=if is_admin { format!(r#"<div style="float: right;"><a href="/notiz/{}/loeschen" class="btn btn-danger"><i class="fa fa-trash-o"></i></a></div>"#, i32::from_value(values[0].clone())) } else { "".to_owned() }
                ))
            } else {
                None
            }
        }
        Err(_) => Some(r#"<div class="alert alert-danger"><strong>Fehlerhafte Nachricht.</strong></div>"#.to_owned())
    }).collect())
}

fn format_entries(entry_type: entry::Type, conn: &mut ::mysql::Conn, is_admin: bool) -> Result<String, ::mysql::Error> {
    let entries = try!(conn.query(format!("SELECT * FROM {}", entry_type.table()))).collect::<Vec<_>>();
    Ok(if entries.len() > 0 {
        entries.into_iter().map(|row| match row {
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
                    edit_buttons=if is_admin { match Option::<i32>::from_value(values[4].clone()) { Some(i) => format!(r#"<div style="float: right;"><a href="/{}/{}/loeschen" class="btn btn-danger"><i class="fa fa-trash-o"></i></a></div>"#, entry_type.url_part(), i), None => "".to_owned() } } else { "".to_owned() }
                )
            }
            Err(_) => format!(r#"
<tr>
    <td></td>
    <td style="color: gray; font-style: italic;">{}.</td>
</tr>
            "#, entry_type.map("Fehlerhaftes Angebot", "Fehlerhafte Anfrage"))
        }).fold("".to_string(), |text, row| text + &row)
    } else {
        format!(r#"
<tr>
    <td></td>
    <td style="color: gray; font-style: italic;">Keine aktiven {}.</td>
</tr>
        "#, entry_type.german_plural())
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
            {notices}
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
        {footer}
    </body>
</html>
        "#,
        header=include_str!("../assets/header.html"),
        intro=include_str!("../assets/intro.html"),
        nav=wiw::nav("boerse", "/", is_admin),
        notices=try!(format_notices(None, &mut conn, is_admin).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))),
        offers=try!(format_entries(entry::Type::Offer, &mut conn, is_admin).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))),
        requests=try!(format_entries(entry::Type::Request, &mut conn, is_admin).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))),
        footer=include_str!("../assets/footer.html")
    ))))
}

fn new_notice_page_inner(form_error: Option<&'static str>, req: &mut Request) -> IronResult<Response> {
    let is_admin = req.get::<IsAdmin>().unwrap_or(false);
    Ok(Response::with((if form_error.is_some() { status::BadRequest } else { status::Ok }, "text/html".parse::<Mime>().unwrap(), format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    {header}
</head>
<body>
    {nav}
    <div class="container" style="position: relative; top: 71px;">
        {error_message}
        <h2>Neue Notiz</h2>
        <form class="form-horizontal" action="/notiz/neu" method="post" enctype="application/x-www-form-urlencoded">
            <div class="form-group">
                <label for="text" class="col-sm-2 control-label">Text</label>
                <div class="col-sm-10">
                    <input type="text" class="form-control" name="text" id="text" placeholder="Inhalt der Notiz" />
                </div>
            </div>
            <div class="form-group">
                <label for="position" class="col-sm-2 control-label">Anzeigen auf</label>
                <div class="col-sm-10">
                    <div class="checkbox">
                        <label>
                            <input type="checkbox" name="position-index" /> Börsen-Hauptseite
                        </label>
                    </div>
                    <div class="checkbox">
                        <label>
                            <input type="checkbox" name="position-create-offer" /> Formular „neues Angebot“
                        </label>
                    </div>
                    <div class="checkbox">
                        <label>
                            <input type="checkbox" name="position-create-request" /> Formular „neue Anfrage“
                        </label>
                    </div>
                </div>
            </div>
            <div class="form-group">
                <div class="col-sm-offset-2 col-sm-10">
                    <a href="/" style="float: right;" class="btn btn-danger">Abbrechen</a>
                    <button type="submit" class="btn btn-primary">Notiz veröffentlichen</button>
                </div>
            </div>
        </form>
    </div>
    {footer}
</body>
</html>
        "#,
        error_message=if let Some(msg) = form_error { format!(r#"<div class="alert alert-danger"><strong>{}</strong> Bitte füllen Sie das Formular erneut aus.</div>"#, msg) } else { String::default() },
        header=include_str!("../assets/header.html"),
        nav=wiw::nav("boerse", "/notiz/neu", is_admin),
        footer=include_str!("../assets/footer.html")
    ))))
}

fn new_entry_page(entry_type: entry::Type, form_error: Option<&'static str>, req: &mut Request) -> IronResult<Response> {
    let is_admin = req.get::<IsAdmin>().unwrap_or(false);
    let mut conn = try!(mysql_connection());
    Ok(Response::with((if form_error.is_some() { status::BadRequest } else { status::Ok }, "text/html".parse::<Mime>().unwrap(), format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    {header}
</head>
<body>
    {nav}
    <div class="container" style="position: relative; top: 71px;">
        {error_message}
        {notices}
        <h2>{title}</h2>
        <form class="form-horizontal" action="/{url_part}/neu" method="post" enctype="application/x-www-form-urlencoded">
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
                    <textarea rows="3" class="form-control" name="description" id="description" placeholder="Beschreiben Sie {article} {entry_type} hier."></textarea>
                </div>
            </div>
            <div class="form-group">
                <div class="col-sm-offset-2 col-sm-10">
                    <a href="/" style="float: right;" class="btn btn-danger">Abbrechen</a>
                    <button type="submit" class="btn btn-primary">{entry_type} einreichen</button>
                </div>
            </div>
        </form>
    </div>
    {footer}
</body>
</html>
        "#,
        error_message=if let Some(msg) = form_error { format!(r#"<div class="alert alert-danger"><strong>{}</strong> Bitte füllen Sie das Formular erneut aus.</div>"#, msg) } else { String::default() },
        header=include_str!("../assets/header.html"),
        nav=wiw::nav("boerse", &format!("/{}/neu", entry_type.url_part())[..], is_admin),
        notices=try!(format_notices(Some(entry_type), &mut conn, is_admin).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank.")))),
        title=entry_type.map("Neues Angebot", "Neue Anfrage"),
        url_part=entry_type.url_part(),
        article=entry_type.german_article(),
        entry_type=entry_type.german_noun(),
        footer=include_str!("../assets/footer.html")
    ))))
}

fn new_offer_page(req: &mut Request) -> IronResult<Response> {
    new_entry_page(entry::Type::Offer, None, req)
}

fn new_request_page(req: &mut Request) -> IronResult<Response> {
    new_entry_page(entry::Type::Request, None, req)
}

fn new_notice_page(req: &mut Request) -> IronResult<Response> {
    new_notice_page_inner(None, req)
}

fn add_notice_inner(req: &mut Request) -> Result<Response, &'static str> {
    let form_data = try!(req.get_ref::<UrlEncodedBody>().map_err(|_| "Fehlender Formularinhalt."));
    let text = mysql_escape_nullable(&form_data["text"][0]);
    if text == "NULL" { return Err("Fehlender Text."); }
    let positions = NoticePositions {
        index: form_data.contains_key("position-index"),
        create_offer: form_data.contains_key("position-create-offer"),
        create_request: form_data.contains_key("position-create-request")
    };
    if !positions.any() { return Err("Bitte geben Sie mindestens eine Position an, wo die Notiz angezeigt werden soll."); }
    let position_str = mysql_escape_nullable(positions.mysql_string());
    let mut conn = try!(mysql_connection().map_err(|_| "Fehler beim Zugriff auf die Datenbank."));
    try!(conn.query(format!("INSERT INTO notices (position, text) VALUES ({}, {})", position_str, text)).map_err(|_| "Fehler beim Zugriff auf die Datenbank."));
    Ok(Response::with((status::Ok, "text/html".parse::<Mime>().unwrap(),
        r#"
<!DOCTYPE html>
<html>
    <body>
        <p>Die Notiz wurde veröffentlicht.</p>
    </body>
</html>
        "#
    ))) //TODO full HTML page with link to indexs
}

fn add_entry(entry_type: entry::Type, req: &mut Request) -> Result<Response, &'static str> {
    let form_data = try!(req.get_ref::<UrlEncodedBody>().map_err(|_| "Fehlender Formularinhalt."));
    let name = mysql_escape_nullable(&form_data["name"][0]);
    if name == "NULL" { return Err("Fehlender Name.") }
    let description = mysql_escape_nullable(&form_data["description"][0]);
    if description == "NULL" { return Err("Fehlende Beschreibung.") }
    let phone = mysql_escape_nullable(&form_data["phone"][0]);
    let mail = mysql_escape_nullable(&form_data["mail"][0]);
    if phone == "NULL" && mail == "NULL" { return Err("Bitte geben Sie eine Telefonnummer oder Mailadresse an.") }
    let mut conn = try!(mysql_connection().map_err(|_| "Fehler beim Zugriff auf die Datenbank."));
    try!(conn.query(format!("INSERT INTO {} (name, description, phone, mail) VALUES ({}, {}, {}, {})", entry_type.table(), name, description, phone, mail)).map_err(|_| "Fehler beim Zugriff auf die Datenbank."));
    Ok(Response::with((status::Ok, "text/html".parse::<Mime>().unwrap(), format!(
        r#"
<!DOCTYPE html>
<html>
    <body>
        <p>{your_entry} wurde eingetragen.</p>
    </body>
</html>
        "#,
        your_entry=entry_type.map("Ihr Angebot", "Ihre Anfrage")
    )))) //TODO full HTML page with link to index
}

fn add_offer(req: &mut Request) -> IronResult<Response> {
    add_entry(entry::Type::Offer, req).or_else(|e| new_entry_page(entry::Type::Offer, Some(e), req))
}

fn add_request(req: &mut Request) -> IronResult<Response> {
    add_entry(entry::Type::Request, req).or_else(|e| new_entry_page(entry::Type::Request, Some(e), req))
}

fn add_notice(req: &mut Request) -> IronResult<Response> {
    add_notice_inner(req).or_else(|e| new_notice_page_inner(Some(e), req))
}

fn del_entry(entry_type: entry::Type, req: &mut Request) -> IronResult<Response> {
    let mut conn = try!(mysql_connection());
    let err_msg = format!("Fehler beim Lesen der {}nummer.", entry_type.map("Angebots", "Anfragen"));
    let id_str = try!(try!(req.extensions.get::<Router>().ok_or(IronError::new(InternalError, (status::InternalServerError, err_msg.clone())))).find("id").ok_or(IronError::new(InternalError, (status::InternalServerError, err_msg.clone()))));
    let id = try!(i32::from_str(id_str).map_err(|e| IronError::new(e, (status::BadRequest, format!("Die {}nummer {:?} ist keine Nummer.", entry_type.map("Angebots", "Anfragen"), id_str)))));
    try!(conn.query(format!("DELETE FROM {} WHERE id={}", entry_type.table(), id)).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))));
    Ok(Response::with((status::Ok, format!("{} {} wurde gelöscht.", entry_type.german_article_capital(), entry_type.german_noun()))))
}

fn del_offer(req: &mut Request) -> IronResult<Response> {
    del_entry(entry::Type::Offer, req)
}

fn del_request(req: &mut Request) -> IronResult<Response> {
    del_entry(entry::Type::Request, req)
}

fn del_notice(req: &mut Request) -> IronResult<Response> {
    let mut conn = try!(mysql_connection());
    let err_msg = "Fehler beim Lesen der Notiznummer.";
    let id_str = try!(try!(req.extensions.get::<Router>().ok_or(IronError::new(InternalError, (status::InternalServerError, err_msg)))).find("id").ok_or(IronError::new(InternalError, (status::InternalServerError, err_msg))));
    let id = try!(i32::from_str(id_str).map_err(|e| IronError::new(e, (status::BadRequest, format!("Die Notiznummer {:?} ist keine Nummer.", id_str)))));
    try!(conn.query(format!("DELETE FROM notices WHERE id={}", id)).map_err(|e| IronError::new(e, (status::InternalServerError, "Fehler beim Zugriff auf die Datenbank."))));
    Ok(Response::with((status::Ok, "Die Notiz wurde gelöscht.")))
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
    router.get("/static", Static::new(Path::new("static")));
    router.get("/biete/neu", new_offer_page);
    router.post("/biete/neu", add_offer);
    router.get("/biete/:id", nyi_handler);
    router.get("/biete/:id/loeschen", { let mut c = Chain::new(del_offer); c.link_before(check_admin_auth); c });
    router.get("/notiz/neu", { let mut c = Chain::new(new_notice_page); c.link_before(check_admin_auth); c });
    router.post("/notiz/neu", { let mut c = Chain::new(add_notice); c.link_before(check_admin_auth); c });
    router.get("/notiz/:id", nyi_handler);
    router.get("/notiz/:id/loeschen", { let mut c = Chain::new(del_notice); c.link_before(check_admin_auth); c });
    router.get("/suche/neu", new_request_page);
    router.post("/suche/neu", add_request);
    router.get("/suche/:id", nyi_handler);
    router.get("/suche/:id/loeschen", { let mut c = Chain::new(del_request); c.link_before(check_admin_auth); c });
    // handle auth
    let mut chain = Chain::new(router);
    chain.link_before(check_auth);
    // serve
    Iron::new(chain).http("0.0.0.0:18800").unwrap();
}
