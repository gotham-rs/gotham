use http::response::{create_response, extend_response};
use router::response::extender::StaticResponseExtender;
use state::{FromState, State, StateData};
use hyper;
use mime::{self, Mime};
use mime_guess::guess_mime_type_opt;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::iter::FromIterator;

use futures::future;
use handler::{Handler, HandlerFuture, NewHandler};

#[derive(Clone)]
pub struct FileSystemHandler {
    root: PathBuf,
}

#[derive(Clone)]
pub struct FileHandler {
    path: PathBuf,
}

impl FileHandler {
    pub fn new(path: &str) -> FileHandler {
        FileHandler { path: PathBuf::from(path) }
    }
}

impl FileSystemHandler {
    pub fn new(root: &str) -> FileSystemHandler {
        FileSystemHandler { root: PathBuf::from(root) }
    }
}

impl NewHandler for FileHandler {
    type Instance = Self;

    fn new_handler(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl NewHandler for FileSystemHandler {
    type Instance = Self;

    fn new_handler(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Handler for FileSystemHandler {
    fn handle(self, state: State) -> Box<HandlerFuture> {
        let path = {
            let mut base_path = PathBuf::from(self.root);
            let file_path = PathBuf::from_iter(&FilePathExtractor::borrow_from(&state).parts);
            base_path.extend(&normalize_path(&file_path));
            base_path
        };
        let response = create_file_response(path, &state);
        Box::new(future::ok((state, response)))
    }
}

impl Handler for FileHandler {
    fn handle(self, state: State) -> Box<HandlerFuture> {
        let response = create_file_response(self.path, &state);
        Box::new(future::ok((state, response)))
    }
}

fn create_file_response(path: PathBuf, state: &State) -> hyper::Response {
    path.metadata()
            .and_then(|meta| {
                let mut contents = Vec::with_capacity(meta.len() as usize);
                fs::File::open(&path).and_then(|mut f| f.read_to_end(&mut contents))?;
                Ok(contents)
            })
            .map(|contents| {
                let mime_type = mime_for_path(&path);
                create_response(state, hyper::StatusCode::Ok, Some((contents, mime_type)))
            })
            .unwrap_or_else(|err| error_response(state, err))
}

fn mime_for_path(path: &Path) -> Mime {
    guess_mime_type_opt(path).unwrap_or_else(|| mime::TEXT_PLAIN)
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(),  |mut result, p| match p {
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
    use super::*;
    use test::TestServer;
    use router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
    use router::Router;
    use hyper::StatusCode;
    use hyper::header::{ContentType};
    use mime::{self, Mime};
    use std::str;

    #[test]
    fn static_files_guesses_content_type() {
        let expected_docs = vec![
            ("doc.html", mime::TEXT_HTML, "<html>I am a doc.</html>"),
            ("styles/style.css", mime::TEXT_CSS, ".styled { border: none; }"),
            ("scripts/script.js", "application/javascript".parse().unwrap(), "console.log('I am javascript!');")
        ];

        for doc in expected_docs {
            let response = test_server()
                .client()
                .get(&format!("http://localhost/{}", doc.0))
                .perform()
                .unwrap();

            assert_eq!(response.status(), StatusCode::Ok);
            assert_eq!(response.headers().get::<ContentType>().unwrap(), &ContentType(doc.1));

            let body = response.read_body().unwrap();
            assert_eq!(&body[..], doc.2.as_bytes());
        }
    }

    // Examples derived from https://www.owasp.org/index.php/Path_Traversal
    #[test]
    fn static_path_traversal() {
        let traversal_attempts = vec![
           r"../private_files/secret.txt",
           r"%2e%2e%2fprivate_files/secret.txt",
           r"%2e%2e/private_files/secret.txt",
           r"..%2fprivate_files/secret.txt",
           r"%2e%2e%5cprivate_files/secret.txt",
           r"%2e%2e\private_files/secret.txt",
           r"..%5cprivate_files/secret.txt",
           r"%252e%252e%255cprivate_files/secret.txt",
           r"..%255cprivate_files/secret.txt",
           r"..%c0%afprivate_files/secret.txt",
           r"..%c1%9cprivate_files/secret.txt",
           "/etc/passwd"
        ];
        for attempt in traversal_attempts {
            let response = test_server()
                .client()
                .get(&format!("http://localhost/{}", attempt))
                .perform()
                .unwrap();

            assert_eq!(response.status(), StatusCode::NotFound);
        }
    }

    #[test]
    fn static_single_file() {
        let test_server = TestServer::new(
            build_simple_router(|route| {
                route
                    .get("/")
                    .to_file("resources/test/static_files/doc.html")
            })
        ).unwrap();

        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.headers().get::<ContentType>().unwrap(), &ContentType::html());

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"<html>I am a doc.</html>");
    }

    fn test_server() -> TestServer {
        TestServer::new(static_router("/*", "resources/test/static_files")).unwrap()
    }

    fn static_router(mount: &str, path: &str) -> Router {
        build_simple_router(|route| {
            route
                .get(mount)
                .to_filesystem(path)
        })
    }
}
