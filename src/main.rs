extern crate iron;
extern crate router;

use iron::prelude::*;
use iron::status;
use router::Router;

fn hello_world(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Hello, world!")))
}

fn main() {
    let mut router = Router::new();
    router.get("/", hello_world);
    Iron::new(router).http("0.0.0.0:18800").unwrap();
}
