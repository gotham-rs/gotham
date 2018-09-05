use bytes::{BufMut, BytesMut};
use error::Result;
use futures::{stream, Future, Stream};
use handler::accepted_encoding::accepted_encodings;
use handler::{Handler, HandlerFuture, IntoHandlerError, NewHandler};
use http;
use httpdate::parse_http_date;
use hyper::header::*;
use hyper::{Body, Chunk, Response, StatusCode};
use mime::{self, Mime};
use mime_guess::guess_mime_type_opt;
use router::response::extender::StaticResponseExtender;
use state::{FromState, State, StateData};
use std::cmp;
use std::collections::HashMap;
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
    options: FileOptions,
}

/// Represents a handler for a single file at `path`.
#[derive(Clone)]
pub struct FileHandler {
    options: FileOptions,
}

#[derive(Clone)]
pub struct FileOptions {
    path: PathBuf,
    cache_control: String,
    gzip: bool,
    brotli: bool,
}

impl FileOptions {
    pub fn new<P: AsRef<Path>>(path: P) -> Self
    where
        PathBuf: From<P>,
    {
        FileOptions {
            path: PathBuf::from(path),
            cache_control: "public".to_string(),
            gzip: false,
            brotli: false,
        }
    }

    pub fn with_cache_control(&mut self, cache_control: String) -> &mut Self {
        self.cache_control = cache_control;
        self
    }

    pub fn with_gzip(&mut self, gzip: bool) -> &mut Self {
        self.gzip = gzip;
        self
    }

    pub fn with_brotli(&mut self, brotli: bool) -> &mut Self {
        self.brotli = brotli;
        self
    }

    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl From<String> for FileOptions {
    fn from(path: String) -> Self {
        FileOptions::new(path)
    }
}

impl<'a> From<&'a String> for FileOptions {
    fn from(path: &'a String) -> Self {
        FileOptions::new(path)
    }
}

impl<'a> From<&'a str> for FileOptions {
    fn from(path: &'a str) -> Self {
        FileOptions::new(path)
    }
}

impl FileHandler {
    /// Create a new `FileHandler` for the given path.
    pub fn new<P>(path: P) -> FileHandler
    where
        FileOptions: From<P>,
    {
        FileHandler {
            options: FileOptions::from(path),
        }
    }
}

impl FileSystemHandler {
    /// Create a new `FileSystemHandler` with the given root path.
    pub fn new<P>(path: P) -> FileSystemHandler
    where
        FileOptions: From<P>,
    {
        FileSystemHandler {
            options: FileOptions::from(path),
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
            let mut base_path = PathBuf::from(self.options.path);
            let file_path = PathBuf::from_iter(&FilePathExtractor::borrow_from(&state).parts);
            base_path.extend(&normalize_path(&file_path));
            base_path
        };
        create_file_response(
            FileOptions {
                path,
                ..self.options
            },
            state,
        )
    }
}

impl Handler for FileHandler {
    fn handle(self, state: State) -> Box<HandlerFuture> {
        create_file_response(self.options, state)
    }
}

fn create_file_response(options: FileOptions, state: State) -> Box<HandlerFuture> {
    let mime_type = mime_for_path(&options.path);
    let headers = HeaderMap::borrow_from(&state).clone();

    let (path, encoding) = check_compressed_options(&options, &headers);

    let response_future = File::open(path).and_then(|file| file.metadata()).and_then(
        move |(file, meta)| {
            if not_modified(&meta, &headers) {
                Ok(http::Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(Body::empty())
                    .unwrap())
            } else {
                let len = meta.len();
                let buf_size = optimal_buf_size(&meta);

                let stream = file_stream(file, buf_size, len);
                let body = Body::wrap_stream(stream);
                let mut response = http::Response::builder();
                response.status(StatusCode::OK);
                response.header(CONTENT_LENGTH, len);
                response.header(CONTENT_TYPE, mime_type.as_ref());
                response.header(CACHE_CONTROL, options.cache_control);

                if let Some(etag) = entity_tag(&meta) {
                    response.header(ETAG, etag);
                }
                if let Some(content_encoding) = encoding {
                    response.header(CONTENT_ENCODING, content_encoding);
                }

                Ok(response.body(body).unwrap())
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

fn check_compressed_options(
    options: &FileOptions,
    headers: &HeaderMap,
) -> (PathBuf, Option<String>) {
    options
        .path
        .file_name()
        .and_then(|filename| {
            let supported = supported_encodings(options);
            accepted_encodings(headers)
                .iter()
                .filter_map(|e| {
                    supported
                        .get(&e.encoding)
                        .map(|ext| (e.encoding.clone(), ext))
                })
                .filter_map(|(encoding, ext)| {
                    let path = options.path.with_file_name(format!(
                        "{}.{}",
                        filename.to_string_lossy(),
                        ext
                    ));
                    if path.exists() {
                        Some((path, Some(encoding)))
                    } else {
                        None
                    }
                })
                .next()
        })
        .unwrap_or((options.path.clone(), None))
}

fn supported_encodings(options: &FileOptions) -> HashMap<String, String> {
    let mut encodings = HashMap::new();
    if options.gzip {
        encodings.insert("gzip".to_string(), "gz".to_string());
    }
    if options.brotli {
        encodings.insert("br".to_string(), "br".to_string());
    }
    encodings
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
    match headers.get(IF_NONE_MATCH) {
        Some(_) => entity_tag(&metadata)
            .map(|etag| headers.get_all(IF_NONE_MATCH).iter().any(|v| v == &etag))
            .unwrap_or(false),
        _ => headers
            .get(IF_MODIFIED_SINCE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| parse_http_date(v).ok())
            .and_then(|if_modified_time| {
                metadata
                    .modified()
                    .map(|modified| modified <= if_modified_time)
                    .ok()
            })
            .unwrap_or(false),
    }
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
    use super::FileOptions;
    use http::header::HeaderValue;
    use hyper::header::*;
    use hyper::StatusCode;
    use router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
    use router::Router;
    use std::{fs, str};
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
    fn static_if_none_match_etag() {
        use hyper::header::{ETAG, IF_NONE_MATCH};
        use std::fs::File;

        let path = "resources/test/static_files/doc.html";
        let test_server =
            TestServer::new(build_simple_router(|route| route.get("/").to_file(path))).unwrap();

        let etag = File::open(path)
            .and_then(|file| file.metadata())
            .map(|meta| super::entity_tag(&meta).expect("entity tag"))
            .unwrap();

        // matching etag
        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(
                IF_NONE_MATCH,
                HeaderValue::from_bytes(etag.as_bytes()).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

        // not matching etag
        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(
                IF_NONE_MATCH,
                HeaderValue::from_bytes("bogus".as_bytes()).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(ETAG).unwrap().to_str().unwrap(),
            etag
        );
    }

    #[test]
    fn static_if_modified_since() {
        use httpdate::fmt_http_date;
        use hyper::header::IF_MODIFIED_SINCE;
        use std::fs::File;
        use std::time::Duration;

        let path = "resources/test/static_files/doc.html";
        let test_server =
            TestServer::new(build_simple_router(|route| route.get("/").to_file(path))).unwrap();

        let modified = File::open(path)
            .and_then(|file| file.metadata())
            .and_then(|meta| meta.modified())
            .unwrap();

        // if-modified-since a newer date
        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(
                IF_MODIFIED_SINCE,
                HeaderValue::from_bytes(fmt_http_date(modified + Duration::new(5, 0)).as_bytes())
                    .unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

        // if-modified-since a older date
        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(
                IF_MODIFIED_SINCE,
                HeaderValue::from_bytes(fmt_http_date(modified - Duration::new(5, 0)).as_bytes())
                    .unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn static_with_cache_control() {
        let router = build_simple_router(|route| {
            route.get("/*").to_filesystem(
                FileOptions::new("resources/test/static_files")
                    .with_cache_control("no-cache".to_string())
                    .build(),
            )
        });
        let server = TestServer::new(router).unwrap();

        let response = server
            .client()
            .get("http://localhost/doc.html")
            .perform()
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .unwrap()
                .to_str()
                .unwrap(),
            "no-cache"
        );
    }

    #[test]
    fn static_default_cache_control() {
        let router = build_simple_router(|route| {
            route.get("/*").to_filesystem("resources/test/static_files")
        });
        let server = TestServer::new(router).unwrap();

        let response = server
            .client()
            .get("http://localhost/doc.html")
            .perform()
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .unwrap()
                .to_str()
                .unwrap(),
            "public"
        );
    }

    #[test]
    fn static_compressed_if_accept_and_exists() {
        let compressed_options = vec![
            (
                "gzip",
                ".gz",
                FileOptions::new("resources/test/static_files")
                    .with_gzip(true)
                    .build(),
            ),
            (
                "br",
                ".br",
                FileOptions::new("resources/test/static_files")
                    .with_brotli(true)
                    .build(),
            ),
        ];

        for (encoding, extension, options) in compressed_options {
            let router = build_simple_router(|route| route.get("/*").to_filesystem(options));
            let server = TestServer::new(router).unwrap();

            let response = server
                .client()
                .get("http://localhost/doc.html")
                .with_header(ACCEPT_ENCODING, HeaderValue::from_str(encoding).unwrap())
                .perform()
                .unwrap();

            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_ENCODING)
                    .unwrap()
                    .to_str()
                    .unwrap(),
                encoding
            );
            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "text/html"
            );

            let expected_body =
                fs::read(format!("resources/test/static_files/doc.html{}", extension)).unwrap();
            assert_eq!(response.read_body().unwrap(), expected_body);
        }
    }

    #[test]
    fn static_no_compression_if_not_accepted() {
        let router = build_simple_router(|route| {
            route.get("/*").to_filesystem(
                FileOptions::new("resources/test/static_files")
                    .with_gzip(true)
                    .with_brotli(true)
                    .build(),
            )
        });
        let server = TestServer::new(router).unwrap();

        let response = server
            .client()
            .get("http://localhost/doc.html")
            .with_header(ACCEPT_ENCODING, HeaderValue::from_str("identity").unwrap())
            .perform()
            .unwrap();

        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html"
        );

        let expected_body = fs::read("resources/test/static_files/doc.html").unwrap();
        assert_eq!(response.read_body().unwrap(), expected_body);
    }

    #[test]
    fn static_no_compression_if_not_exists() {
        let router = build_simple_router(|route| {
            route.get("/*").to_filesystem(
                FileOptions::new("resources/test/static_files_uncompressed")
                    .with_gzip(true)
                    .with_brotli(true)
                    .build(),
            )
        });
        let server = TestServer::new(router).unwrap();

        let response = server
            .client()
            .get("http://localhost/doc.html")
            .with_header(ACCEPT_ENCODING, HeaderValue::from_str("gzip").unwrap())
            .with_header(ACCEPT_ENCODING, HeaderValue::from_str("brotli").unwrap())
            .perform()
            .unwrap();

        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html"
        );

        let expected_body = fs::read("resources/test/static_files_uncompressed/doc.html").unwrap();
        assert_eq!(response.read_body().unwrap(), expected_body);
    }

    #[test]
    fn static_weighted_accept_encoding() {
        let router = build_simple_router(|route| {
            route.get("/*").to_filesystem(
                FileOptions::new("resources/test/static_files")
                    .with_gzip(true)
                    .with_brotli(true)
                    .build(),
            )
        });
        let server = TestServer::new(router).unwrap();

        let response = server
            .client()
            .get("http://localhost/doc.html")
            .with_header(
                ACCEPT_ENCODING,
                HeaderValue::from_str("*;q=0.1, br;q=1.0, gzip;q=0.8").unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html"
        );

        assert_eq!(
            response
                .headers()
                .get(CONTENT_ENCODING)
                .unwrap()
                .to_str()
                .unwrap(),
            "br"
        );
        let expected_body = fs::read("resources/test/static_files/doc.html.br").unwrap();
        assert_eq!(response.read_body().unwrap(), expected_body);
    }

    fn test_server() -> TestServer {
        TestServer::new(static_router("/*", "resources/test/static_files")).unwrap()
    }

    fn static_router(mount: &str, path: &str) -> Router {
        build_simple_router(|route| route.get(mount).to_filesystem(path))
    }
}
