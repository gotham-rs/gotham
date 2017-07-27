use std::collections::HashMap;

use hyper::{StatusCode, Body};
use hyper::header::Location;
use hyper::server::{Request, Response};
use futures::{Future, Stream};

use gotham::state::State;
use gotham::middleware::session::SessionData;

use apps::todo::Session;

pub fn index(state: State, _req: Request) -> (State, Response) {
    let response = {
        if let Some(session) = state.borrow::<SessionData<Session>>() {
            Response::new()
                .with_status(StatusCode::Ok)
                .with_body(index_body(session.todo_list.clone()))
        } else {
            Response::new()
                .with_status(StatusCode::InternalServerError)
                .with_body(Body::from("no session"))
        }
    };

    (state, response)
}

// TODO: This is full of CSRF holes. Don't be full of holes.
pub fn add(mut state: State, req: Request) -> (State, Response) {
    let response = {
        if let Some(ref mut session) = state.borrow_mut::<SessionData<Session>>() {
            let req_body = ugly_request_body_reader(req.body());
            let data = ugly_form_body_parser(&req_body);

            if let Some(item) = data.get("item") {
                session.todo_list.push((*item).to_owned());
            }

            let mut response = Response::new().with_status(StatusCode::SeeOther);
            response.headers_mut().set(Location::new("/"));

            response
        } else {
            Response::new()
                .with_status(StatusCode::InternalServerError)
                .with_body(Body::from("no session"))
        }
    };

    (state, response)
}

pub fn reset(mut state: State, _req: Request) -> (State, Response) {
    if let Some(session) = state.take::<SessionData<Session>>() {
        session.discard().unwrap();
    }

    let mut response = Response::new().with_status(StatusCode::SeeOther);
    response.headers_mut().set(Location::new("/"));

    (state, response)
}

// TODO: Use a template library instead. This is ugly.
fn index_body(items: Vec<String>) -> Body {
    let mut out = String::new();

    let part = r#"
        <!doctype html>
        <html>
            <head>
                <title>Todo (Session-backed)</title>
            </head>
            <body>
                <h1>Todo list</h1>
    "#;
    out.extend(part.chars());

    // TODO: This allows HTML injection by the user, huge potential for XSS if copied.
    // Tidy this up sooner rather than later. Currently only thwarted by us not decoding the
    // URL-encoded body.
    if items.len() > 0 {
        out.extend("<ul>".chars());
        for item in items {
            let part = format!("<li>{}</li>", item);
            out.extend(part.chars());
        }
        out.extend("</ul>".chars());
    }

    let part = r#"
                <form method="post">
                    <input type="text" name="item"/>
                    <button type="submit">Add</button>
                </form>
                <script type="text/javascript">
                    document.forms[0].getElementsByTagName('input')[0].focus()
                </script>

                <form method="post" action="/reset">
                    <button type="submit">Reset</button>
                </form>
            </body>
        </html>
        "#;
    out.extend(part.chars());

    Body::from(out)
}

// TODO: Remove gratuitous unwrapping
fn ugly_request_body_reader(body: Body) -> String {
    let mut req_body = Vec::new();
    for part in body.collect().wait().unwrap() {
        req_body.extend(part);
    }

    String::from_utf8(req_body).unwrap()
}

// TODO: Remove gratuitous unwrapping
fn ugly_form_body_parser<'a>(body: &'a str) -> HashMap<&'a str, &'a str> {
    let pairs = body.split("&").filter(|pair| pair.contains("="));
    let mut data = HashMap::new();

    data.extend(pairs.map(|p| {
                              let mut iter = p.split("=");
                              (iter.next().unwrap(), iter.next().unwrap())
                          }));

    data
}
