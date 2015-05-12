use iron::prelude::*;
use iron::typemap::Key;

use plugin;

use util::check_admin_auth;

pub struct IsAdmin;

impl Key for IsAdmin {
    type Value = bool;
}

impl<'a, 'b> plugin::Plugin<Request<'a, 'b>> for IsAdmin {
    type Error = ();

    fn eval(req: &mut Request) -> Result<bool, ()> {
        Ok(check_admin_auth(req).is_ok())
    }
}
