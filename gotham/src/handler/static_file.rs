use error::Result;
use helpers::http::response::{create_response, extend_response};
use hyper::{body::Payload, Body, Response, StatusCode};
use mime::{self, Mime};
use mime_guess::guess_mime_type_opt;
use router::response::extender::StaticResponseExtender;
use state::{FromState, State, StateData};
use std::convert::From;
use std::fs;
use std::io::{self, Read};
use std::iter::FromIterator;
use std::path::{Component, Path, PathBuf};

use futures::future;
use handler::{Handler, HandlerFuture, NewHandler};

/// Represents a handler for any files under the path `root`.
#[derive(Clone)]
pub struct FileSystemHandler {
    root: PathBuf,
}

/// Represents a handler for a single file at `path`.
#[derive(Clone)]
pub struct FileHandler {
    path: PathBuf,
}

impl FileHandler {
    /// Create a new `FileHandler` for the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> FileHandler
    where
        PathBuf: From<P>,
    {
        FileHandler {
            path: PathBuf::from(path),
        }
    }
}

impl FileSystemHandler {
    /// Create a new `FileSystemHandler` with the given root path.
    pub fn new<P: AsRef<Path>>(root: P) -> FileSystemHandler
    where
        PathBuf: From<P>,
    {
        FileSystemHandler {
            root: PathBuf::from(root),
        }
    }
}

impl NewHandler for FileHandler {
    type Instance = Self;

    fn new_handler(&self) -> Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl NewHandler for FileSystemHandler {
    type Instance = Self;

    fn new_handler(&self) -> Result<Self::Instance> {
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

fn create_file_response(path: PathBuf, state: &State) -> Response<Body> {
    path.metadata()
        .and_then(|meta| {
            let mut contents = Vec::with_capacity(meta.len() as usize);
            fs::File::open(&path).and_then(|mut f| f.read_to_end(&mut contents))?;
            Ok(contents)
        })
        .map(|contents| {
            let mime_type = mime_for_path(&path);
            create_response(state, StatusCode::OK, Some((contents, mime_type)))
        })
        .unwrap_or_else(|err| error_response(state, err))
}

fn mime_for_path(path: &Path) -> Mime {
    guess_mime_type_opt(path).unwrap_or_else(|| mime::APPLICATION_OCTET_STREAM)
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

fn error_response(state: &State, e: io::Error) -> Response<Body> {
    let status = match e.kind() {
        io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    create_response(
        &state,
        status,
        Some((format!("{}", status).into_bytes(), mime::TEXT_PLAIN)),
    )
}

/// Responsible for extracting the file path matched by the glob segment from the URL.
#[derive(Debug, Deserialize)]
pub struct FilePathExtractor {
    #[serde(rename = "*")]
    parts: Vec<String>,
}

impl StateData for FilePathExtractor {}

impl StaticResponseExtender for FilePathExtractor {
    type ResBody = Body;
    fn extend(_state: &mut State, _res: &mut Response<Self::ResBody>) {}
}

#[cfg(test)]
mod tests {
    // use helpers::http::header::*;
    use hyper::header::CONTENT_TYPE;
    use hyper::StatusCode;
    use mime;
    use router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
    use router::Router;
    use std::str;
    use test::TestServer;

    #[test]
    fn static_files_guesses_content_type() {
        let expected_docs = vec![
            ("doc.html", mime::TEXT_HTML, "<html>I am a doc.</html>"),
            ("file.txt", mime::TEXT_PLAIN, "I am a file"),
            (
                "styles/style.css",
                mime::TEXT_CSS,
                ".styled { border: none; }",
            ),
            (
                "scripts/script.js",
                "application/javascript".parse().unwrap(),
                "console.log('I am javascript!');",
            ),
        ];

        for doc in expected_docs {
            let response = test_server()
                .client()
                .get(&format!("http://localhost/{}", doc.0))
                .perform()
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), doc.1);

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
            "/etc/passwd",
        ];
        for attempt in traversal_attempts {
            let response = test_server()
                .client()
                .get(&format!("http://localhost/{}", attempt))
                .perform()
                .unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    #[test]
    fn static_single_file() {
        let test_server = TestServer::new(build_simple_router(|route| {
            route
                .get("/")
                .to_file("resources/test/static_files/doc.html")
        })).unwrap();

        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "text/html");

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"<html>I am a doc.</html>");
    }

    fn test_server() -> TestServer {
        TestServer::new(static_router("/*", "resources/test/static_files")).unwrap()
    }

    fn static_router(mount: &str, path: &str) -> Router {
        build_simple_router(|route| route.get(mount).to_filesystem(path))
    }
}
