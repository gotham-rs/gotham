use http::response::{create_response, extend_response};
use router::response::extender::StaticResponseExtender;
use state::{FromState, State, StateData};
use hyper;
use mime::{self, Mime};
use mime_guess::guess_mime_type_opt;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use futures::future;
use handler::{Handler, HandlerFuture, NewHandler};

#[derive(Clone)]
pub struct StaticFileHandler {
    root: PathBuf,
}

impl StaticFileHandler {
    pub fn new(root: PathBuf) -> StaticFileHandler {
        StaticFileHandler { root }
    }
}

impl NewHandler for StaticFileHandler {
    type Instance = Self;

    fn new_handler(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Handler for StaticFileHandler {
    fn handle(self, state: State) -> Box<HandlerFuture> {
        debug!("Handling static request");
        let path = {
            debug!("Root path {:?}", self.root);
            let mut path_buf = PathBuf::from(self.root);
            path_buf.extend(&FilePathExtractor::borrow_from(&state).parts);
            debug!("Has path {:?}", path_buf);
            normalize_path(&path_buf)
        };
        debug!("Normalised path {:?}", path);
        let response = path.metadata()
            .and_then(|meta| {
                let mut contents = Vec::with_capacity(meta.len() as usize);
                fs::File::open(&path).and_then(|mut f| f.read_to_end(&mut contents))?;
                Ok(contents)
            })
            .map(|contents| {
                let mime_type = mime_for_path(&path);
                create_response(&state, hyper::StatusCode::Ok, Some((contents, mime_type)))
            })
            .unwrap_or_else(|err| error_response(&state, err));

        Box::new(future::ok((state, response)))
    }
}

fn mime_for_path(path: &Path) -> Mime {
    guess_mime_type_opt(path).unwrap_or_else(|| mime::TEXT_PLAIN)
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut result, p| match p {
            Component::Normal(x) => {
                result.push(x);
                result
            }
            Component::ParentDir => {
                result.pop();
                result
            }
            _ => result,
        })
}

fn error_response(state: &State, e: io::Error) -> hyper::Response {
    let status = match e.kind() {
        io::ErrorKind::NotFound => hyper::StatusCode::NotFound,
        io::ErrorKind::PermissionDenied => hyper::StatusCode::Forbidden,
        _ => hyper::StatusCode::InternalServerError,
    };
    create_response(
        &state,
        status,
        Some((format!("{}", status).into_bytes(), mime::TEXT_PLAIN)),
    )
}

#[derive(Debug, Deserialize)]
pub struct FilePathExtractor {
    #[serde(rename = "*")]
    parts: Vec<String>,
}

impl StateData for FilePathExtractor {}

impl StaticResponseExtender for FilePathExtractor {
    fn extend(state: &mut State, res: &mut hyper::Response) {
        extend_response(state, res, ::hyper::StatusCode::BadRequest, None);
    }
}

#[cfg(test)]
mod tests {
    use test::TestServer;
    use router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
    use router::Router;
    use std::path::PathBuf;
    use handler::static_file::StaticFileHandler;
    use hyper::StatusCode;
    use hyper::header::{ContentType};
    use mime::{self, Mime};


    #[test]
    fn get_static_html() {
        let response = test_server()
            .client()
            .get("http://localhost/doc.html")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.headers().get::<ContentType>().unwrap(), &ContentType::html());

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"<html>I am a doc.</html>");
    }

    #[test]
    fn get_static_css() {
        let response = test_server()
            .client()
            .get("http://localhost/styles/style.css")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.headers().get::<ContentType>().unwrap(), &ContentType(mime::TEXT_CSS));

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b".styled { border: none; }");
    }

    #[test]
    fn get_static_js() {
        let response = test_server()
            .client()
            .get("http://localhost/scripts/script.js")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let application_javascript: mime::Mime = "application/javascript".parse().unwrap();
        assert_eq!(response.headers().get::<ContentType>().unwrap(), &ContentType(application_javascript));

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"console.log('I am javascript!');");
    }

    fn test_server() -> TestServer {
        TestServer::new(static_router("/*", "resources/test/static_files")).unwrap()
    }

    fn static_router(mount: &str, path: &str) -> Router {
        let path_buf = PathBuf::from(path);
        build_simple_router(|route| {
            route
                .get(mount)
                .to_filesystem(StaticFileHandler::new(path_buf))
        })
    }
}
