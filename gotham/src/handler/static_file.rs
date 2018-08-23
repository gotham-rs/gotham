use bytes::{BufMut, BytesMut};
use error::Result;
use futures::{stream, Future, Stream};
use handler::{Handler, HandlerFuture, IntoHandlerError, NewHandler};
use helpers::http::response::create_response;
use http;
use hyper::header::{
    HeaderMap, HeaderValue, ACCEPT_ENCODING, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
};
use hyper::{Body, Chunk, Response, StatusCode};
use mime::{self, Mime};
use mime_guess::guess_mime_type_opt;
use router::response::extender::StaticResponseExtender;
use state::{FromState, State, StateData};
use std::cmp;
use std::convert::From;
use std::fs::Metadata;
use std::io;
use std::iter::FromIterator;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::fs::File;
use tokio::io::AsyncRead;

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
        create_file_response(path, state)
    }
}

impl Handler for FileHandler {
    fn handle(self, state: State) -> Box<HandlerFuture> {
        create_file_response(self.path, state)
    }
}

fn create_file_response(path: PathBuf, state: State) -> Box<HandlerFuture> {
    let mime_type = mime_for_path(&path);
    let headers = HeaderMap::borrow_from(&state).clone();

    let response_future = File::open(path).and_then(|file| file.metadata()).and_then(
        move |(file, meta)| {
            if not_modified(&meta, &headers) {
                Ok(http::Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body("".into())
                    .unwrap())
            } else {
                let len = meta.len();
                let buf_size = optimal_buf_size(&meta);

                let stream = file_stream(file, buf_size, len);
                let body = Body::wrap_stream(stream);

                Ok(http::Response::builder()
                    .status(StatusCode::OK)
                    .header("content-length", len)
                    .header("content-type", mime_type.as_ref())
                    .body(body)
                    .unwrap())
            }
        },
    );
    Box::new(response_future.then(|result| match result {
        Ok(response) => Ok((state, response)),
        Err(err) => {
            let status = error_status(&err);
            Err((state, err.into_handler_error().with_status(status)))
        }
    }))
}

fn error_status(e: &io::Error) -> StatusCode {
    match e.kind() {
        io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
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

fn not_modified(metadata: &Metadata, headers: &HeaderMap) -> bool {
    // If-None-Match header takes precedence over If-Modified-Since
    if let Some(etag) = entity_tag(&metadata) {
        let if_none_match = headers.get_all(IF_NONE_MATCH);
        if if_none_match.iter().any(|v| v == &etag) {
            return true;
        }
    };
    // } else if let Some(IfModifiedSince(if_modified_time)) = if_modified_since {
    //     metadata
    //         .modified()
    //         .map(|modified| HttpDate::from(modified) <= if_modified_time)
    //         .unwrap_or(false)
    // } else {
    //     false
    // }
    false
}

fn entity_tag(metadata: &Metadata) -> Option<String> {
    metadata.modified().ok().and_then(|modified| {
        modified.duration_since(UNIX_EPOCH).ok().map(|duration| {
            format!(
                "W/\"{0:x}-{1:x}.{2:x}\"",
                metadata.len(),
                duration.as_secs(),
                duration.subsec_nanos()
            )
        })
    })
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

fn file_stream(
    mut f: File,
    buf_size: usize,
    mut len: u64,
) -> impl Stream<Item = Chunk, Error = io::Error> + Send {
    let mut buf = BytesMut::new();
    stream::poll_fn(move || {
        if len == 0 {
            return Ok(None.into());
        }
        if buf.remaining_mut() < buf_size {
            buf.reserve(buf_size);
        }
        let n = try_ready!(f.read_buf(&mut buf).map_err(|err| {
            debug!("file read error: {}", err);
            err
        })) as u64;

        if n == 0 {
            debug!("file read found EOF before expected length");
            return Ok(None.into());
        }

        let mut chunk = buf.take().freeze();
        if n > len {
            chunk = chunk.split_to(len as usize);
            len = 0;
        } else {
            len -= n;
        }

        Ok(Some(Chunk::from(chunk)).into())
    })
}

fn optimal_buf_size(metadata: &Metadata) -> usize {
    let block_size = get_block_size(metadata);

    // If file length is smaller than block size, don't waste space
    // reserving a bigger-than-needed buffer.
    cmp::min(block_size as u64, metadata.len()) as usize
}

#[cfg(unix)]
fn get_block_size(metadata: &Metadata) -> usize {
    use std::os::unix::fs::MetadataExt;
    metadata.blksize() as usize
}

#[cfg(not(unix))]
fn get_block_size(metadata: &Metadata) -> usize {
    8_192
}

#[cfg(test)]
mod tests {
    use http::header::HeaderValue;
    use hyper::header::CONTENT_TYPE;
    use hyper::StatusCode;
    use router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
    use router::Router;
    use std::str;
    use test::TestServer;

    #[test]
    fn static_files_guesses_content_type() {
        let expected_docs = vec![
            (
                "doc.html",
                HeaderValue::from_static("text/html"),
                "<html>I am a doc.</html>",
            ),
            (
                "file.txt",
                HeaderValue::from_static("text/plain"),
                "I am a file",
            ),
            (
                "styles/style.css",
                HeaderValue::from_static("text/css"),
                ".styled { border: none; }",
            ),
            (
                "scripts/script.js",
                HeaderValue::from_static("application/javascript"),
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
            r"%2e%2e/private_files/secret.txt",
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

    #[test]
    fn static_not_modified_etag() {
        use std::fs::File;

        let path = "resources/test/static_files/doc.html";
        let test_server =
            TestServer::new(build_simple_router(|route| route.get("/").to_file(path))).unwrap();

        let etag = File::open(path)
            .and_then(|file| file.metadata())
            .map(|meta| super::entity_tag(&meta).expect("entity tag"))
            .unwrap();

        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(
                "If-None-Match",
                HeaderValue::from_bytes(etag.as_bytes()).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }

    fn test_server() -> TestServer {
        TestServer::new(static_router("/*", "resources/test/static_files")).unwrap()
    }

    fn static_router(mount: &str, path: &str) -> Router {
        build_simple_router(|route| route.get(mount).to_filesystem(path))
    }
}
