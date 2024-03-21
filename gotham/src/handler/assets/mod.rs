//! Defines handlers for static assets, used by `to_file` and `to_dir` routes.
//! Both 'If-None-Match' (etags) and 'If-Modified-Since' are supported to check
//! file modification.
//! Side-by-side compressed files for gzip and brotli are supported if enabled
//! See 'FileOptions' for more details.

mod accepted_encoding;

use bytes::{BufMut, Bytes, BytesMut};
use futures_util::stream::{self, TryStream, TryStreamExt};
use futures_util::{ready, FutureExt, TryFutureExt};
use httpdate::parse_http_date;
use hyper::header::*;
use hyper::{Body, Response, StatusCode};
use log::debug;
use mime::{self, Mime};
use mime_guess::from_path;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncSeekExt, ReadBuf};

use self::accepted_encoding::accepted_encodings;
use crate::handler::{Handler, HandlerError, HandlerFuture, NewHandler};
use crate::router::response::StaticResponseExtender;
use crate::state::{FromState, State, StateData};

use std::convert::From;
use std::fs::Metadata;
use std::io::{ErrorKind, SeekFrom};
use std::iter::FromIterator;
use std::mem::MaybeUninit;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::task::Poll;
use std::time::UNIX_EPOCH;
use std::{cmp, io};

/// Represents a handler for any files under a directory.
#[derive(Clone)]
pub struct DirHandler {
    options: FileOptions,
}

/// Represents a handler for a single file.
#[derive(Clone)]
pub struct FileHandler {
    options: FileOptions,
}

/// Options to pass to file or dir handlers.
/// Allows overriding default behaviour for compression, cache control headers, etc.
///
/// `FileOptions` implements `From` for `String` and `PathBuf` (and related reference types) - so that a
/// path can be passed to router builder methods if only default options are required.
///
/// For overridding default options, `FileOptions` provides builder methods. The default
/// values and use of the builder methods are shown in the example below.
///
///
/// ```rust
/// # use gotham::handler::FileOptions;
///
/// let default_options = FileOptions::from("my_static_path");
/// let from_builder = FileOptions::new("my_static_path")
///     .with_cache_control("public")
///     .with_gzip(false)
///     .with_brotli(false)
///     .build();
///
/// assert_eq!(default_options, from_builder);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOptions {
    path: PathBuf,
    cache_control: String,
    gzip: bool,
    brotli: bool,
    buffer_size: Option<usize>,
}

impl FileOptions {
    /// Create a new `FileOptions` with default values.
    pub fn new<P: AsRef<Path>>(path: P) -> Self
    where
        PathBuf: From<P>,
    {
        FileOptions {
            path: PathBuf::from(path),
            cache_control: "public".to_string(),
            gzip: false,
            brotli: false,
            buffer_size: None,
        }
    }

    /// Sets the "cache_control" header in static file responses to the given value.
    pub fn with_cache_control(&mut self, cache_control: &str) -> &mut Self {
        self.cache_control = cache_control.to_owned();
        self
    }

    /// If `true`, given a request for FILE, serves FILE.gz if it exists in the static directory and
    /// if the accept-encoding header is set to allow gzipped content (defaults to false).
    pub fn with_gzip(&mut self, gzip: bool) -> &mut Self {
        self.gzip = gzip;
        self
    }

    /// If `true`, given a request for FILE, serves FILE.br if it exists in the static directory and
    /// if the accept-encoding header is set to allow brotli content (defaults to false).
    pub fn with_brotli(&mut self, brotli: bool) -> &mut Self {
        self.brotli = brotli;
        self
    }

    /// Sets the maximum buffer size to be used when serving the file.
    /// If unset, the default maximum buffer size corresponding to file system block size will be used.
    pub fn with_buffer_size(&mut self, buf_sz: usize) -> &mut Self {
        self.buffer_size = Some(buf_sz);
        self
    }

    /// Clones `self` to return an owned value for passing to a handler.
    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

/// Create a `FileOptions` from various types, used in
/// the router builder `to_file` and `to_dir` implementations
/// which have a constraint `FileOptions: From<P>` for default options.
macro_rules! derive_from {
    ($type:ty) => {
        impl From<$type> for FileOptions {
            fn from(t: $type) -> FileOptions {
                FileOptions::new(t)
            }
        }
    };
}

derive_from!(&Path);
derive_from!(PathBuf);
derive_from!(&str);
derive_from!(&String);
derive_from!(String);

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

impl DirHandler {
    /// Create a new `DirHandler` with the given root path.
    pub fn new<P>(path: P) -> DirHandler
    where
        FileOptions: From<P>,
    {
        DirHandler {
            options: FileOptions::from(path),
        }
    }
}

impl NewHandler for FileHandler {
    type Instance = Self;

    fn new_handler(&self) -> anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl NewHandler for DirHandler {
    type Instance = Self;

    fn new_handler(&self) -> anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Handler for DirHandler {
    fn handle(self, state: State) -> Pin<Box<HandlerFuture>> {
        let path = {
            let mut base_path = self.options.path;
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
    fn handle(self, state: State) -> Pin<Box<HandlerFuture>> {
        create_file_response(self.options, state)
    }
}

// Creates the `HandlerFuture` response based on the given `FileOptions`.
fn create_file_response(options: FileOptions, state: State) -> Pin<Box<HandlerFuture>> {
    let mime_type = mime_for_path(&options.path);
    let headers = HeaderMap::borrow_from(&state).clone();

    let (path, encoding) = check_compressed_options(&options, &headers);

    let response_future = File::open(path).and_then(move |mut file| async move {
        let meta = file.metadata().await?;
        if not_modified(&meta, &headers) {
            return Ok(hyper::Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())
                .unwrap());
        }
        let buf_size = options.buffer_size.unwrap_or_else(|| optimal_buf_size(&meta));
        let (len, range_start) = match resolve_range(meta.len(), &headers) {
            Ok((len, range_start)) => (len, range_start),
            Err(e) => {
                return Ok(hyper::Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .body(Body::from(e))
                    .unwrap());
            }
        };
        if let Some(seek_to) = range_start {
            file.seek(SeekFrom::Start(seek_to)).await?;
        };

        let stream = file_stream(file, cmp::min(buf_size, len as usize), len);
        let body = Body::wrap_stream(stream.into_stream());
        let mut response = hyper::Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_LENGTH, len)
            .header(CONTENT_TYPE, mime_type.as_ref())
            .header(CACHE_CONTROL, options.cache_control);

        if let Some(etag) = entity_tag(&meta) {
            response = response.header(ETAG, etag);
        }
        if let Some(content_encoding) = encoding {
            response = response.header(CONTENT_ENCODING, content_encoding);
        }

        if let Some(range_start) = range_start {
            let val = format!(
                "bytes {}-{}/{}",
                range_start,
                (range_start + len).saturating_sub(1),
                meta.len()
            );
            response = response.status(StatusCode::PARTIAL_CONTENT).header(
                CONTENT_RANGE,
                HeaderValue::from_str(&val).map_err(|e| io::Error::new(ErrorKind::Other, e))?,
            );
        }

        Ok(response.body(body).unwrap())
    });

    response_future
        .map(|result| match result {
            Ok(response) => Ok((state, response)),
            Err(err) => {
                let status = match err.kind() {
                    io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
                    io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                let err: HandlerError = err.into();
                Err((state, err.with_status(status)))
            }
        })
        .boxed()
}

/// Checks for existence of "Range" header and whether it is in supported format
/// This implementations only supports single part ranges.
/// Returns a result of length and optional starting position, or an error if range value is invalid
/// If range header does not exist or is unsupported the length is the whole file length and start position is none.
fn resolve_range(len: u64, headers: &HeaderMap) -> Result<(u64, Option<u64>), &'static str> {
    let Some(range_val) = headers.get(RANGE) else {
        return Ok((len, None));
    };
    range_val
        .to_str()
        .ok()
        .and_then(|range_val| {
            regex::Regex::new(r"^bytes=(\d*)-(\d*)$")
                .unwrap()
                .captures(range_val)
                .map(|captures| {
                    let begin = captures
                        .get(1)
                        .and_then(|digits| digits.as_str().parse::<u64>().ok());
                    let end = captures
                        .get(2)
                        .and_then(|digits| digits.as_str().parse::<u64>().ok());
                    match (begin, end) {
                        (Some(begin), Some(end)) => {
                            let end = cmp::min(end, len.saturating_sub(1));
                            if end < begin {
                                Err("invalid range")
                            } else {
                                let begin = cmp::min(begin, end);
                                Ok(((1 + end).saturating_sub(begin), Some(begin)))
                            }
                        }
                        (Some(begin), None) => {
                            let end = len.saturating_sub(1);
                            let begin = cmp::min(begin, len);
                            Ok((1 + end.saturating_sub(begin), Some(begin)))
                        }
                        (None, Some(end)) => {
                            let begin = len.saturating_sub(end);
                            Ok((end, Some(begin)))
                        }
                        (None, None) => Err("invalid range"),
                    }
                })
        })
        .unwrap_or(Ok((len, None)))
}

// Checks for existence of compressed files if `FileOptions` and
// "Accept-Encoding" headers allow. Returns the final path to read,
// along with an optional encoding to return as the "Content-Encoding".
fn check_compressed_options(
    options: &FileOptions,
    headers: &HeaderMap,
) -> (PathBuf, Option<String>) {
    options
        .path
        .file_name()
        .and_then(|filename| {
            accepted_encodings(headers)
                .iter()
                .filter_map(|e| {
                    get_extension(&e.encoding, options).map(|ext| (e.encoding.to_string(), ext))
                })
                .find_map(|(encoding, ext)| {
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
        })
        .unwrap_or((options.path.clone(), None))
}

// Gets the file extension for the compressed version of a file
// for a given encoding, if allowed by `FileOptions`.
fn get_extension(encoding: &str, options: &FileOptions) -> Option<String> {
    if encoding == "gzip" && options.gzip {
        return Some("gz".to_string());
    }
    if encoding == "br" && options.brotli {
        return Some("br".to_string());
    }
    None
}

fn mime_for_path(path: &Path) -> Mime {
    from_path(path).first_or_octet_stream()
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

// Checks whether a file is modified based on metadata and request headers.
fn not_modified(metadata: &Metadata, headers: &HeaderMap) -> bool {
    // If-None-Match header takes precedence over If-Modified-Since
    match headers.get(IF_NONE_MATCH) {
        Some(_) => entity_tag(metadata)
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

// Creates a Stream from the given file, for streaming as part of the Response.
// Inspired by Warp https://github.com/seanmonstar/warp/blob/master/src/filters/fs.rs
// Inspired by tokio https://github.com/tokio-rs/tokio/blob/master/tokio/src/io/util/read_buf.rs
// Thanks @seanmonstar and @carllerche.
fn file_stream(
    mut f: File,
    buf_size: usize,
    mut len: u64,
) -> impl TryStream<Ok = Bytes, Error = io::Error> + Send {
    let mut buf = BytesMut::with_capacity(buf_size);
    stream::poll_fn(move |cx| {
        if len == 0 {
            return Poll::Ready(None);
        }
        if buf.remaining_mut() < buf_size {
            buf.reserve(buf_size);
        }

        let dst = buf.chunk_mut();
        let dst = unsafe { &mut *(dst as *mut _ as *mut [MaybeUninit<u8>]) };
        let mut read_buf = ReadBuf::uninit(dst);
        let read = Pin::new(&mut f).poll_read(cx, &mut read_buf);
        ready!(read).map_err(|err| {
            debug!("file read error: {}", err);
            err
        })?;

        if read_buf.filled().is_empty() {
            debug!("file read found EOF before expected length");
            return Poll::Ready(Some(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "file read found EOF before expected length",
            ))));
        }

        let n = read_buf.filled().len();
        // Safety: This is guaranteed to be the number of initialized (and read)
        // bytes due to the invariants provided by `ReadBuf::filled`.
        unsafe {
            buf.advance_mut(n);
        }
        let n = n as u64;

        let chunk = if n > len {
            let chunk = buf.split_to(len as usize);
            len = 0;
            chunk
        } else {
            len -= n;
            buf.split()
        };

        Poll::Ready(Some(Ok(chunk.freeze())))
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
    use crate::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
    use crate::router::Router;
    use crate::test::TestServer;
    use hyper::header::*;
    use hyper::StatusCode;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::path::PathBuf;
    use std::{fs, str};
    #[test]
    fn assets_guesses_content_type() {
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
    fn assets_path_traversal() {
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
    fn assets_single_file() {
        let test_server = TestServer::new(build_simple_router(|route| {
            route.get("/").to_file("resources/test/assets/doc.html")
        }))
        .unwrap();

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
    fn assets_if_none_match_etag() {
        use hyper::header::{ETAG, IF_NONE_MATCH};
        use std::fs::File;

        let path = "resources/test/assets/doc.html";
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
            .with_header(IF_NONE_MATCH, HeaderValue::from_bytes(b"bogus").unwrap())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(ETAG).unwrap().to_str().unwrap(),
            etag
        );
    }

    #[test]
    fn assets_if_modified_since() {
        use httpdate::fmt_http_date;
        use hyper::header::IF_MODIFIED_SINCE;
        use std::fs::File;
        use std::time::Duration;

        let path = "resources/test/assets/doc.html";
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
    fn assets_with_cache_control() {
        let router = build_simple_router(|route| {
            route.get("/*").to_dir(
                FileOptions::new("resources/test/assets")
                    .with_cache_control("no-cache")
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
    fn assets_default_cache_control() {
        let router = build_simple_router(|route| route.get("/*").to_dir("resources/test/assets"));
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
    fn assets_compressed_if_accept_and_exists() {
        let compressed_options = vec![
            (
                "gzip",
                ".gz",
                FileOptions::new("resources/test/assets")
                    .with_gzip(true)
                    .build(),
            ),
            (
                "br",
                ".br",
                FileOptions::new("resources/test/assets")
                    .with_brotli(true)
                    .build(),
            ),
        ];

        for (encoding, extension, options) in compressed_options {
            let router = build_simple_router(|route| route.get("/*").to_dir(options));
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
                fs::read(format!("resources/test/assets/doc.html{}", extension)).unwrap();
            assert_eq!(response.read_body().unwrap(), expected_body);
        }
    }

    #[test]
    fn assets_no_compression_if_not_accepted() {
        let router = build_simple_router(|route| {
            route.get("/*").to_dir(
                FileOptions::new("resources/test/assets")
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

        let expected_body = fs::read("resources/test/assets/doc.html").unwrap();
        assert_eq!(response.read_body().unwrap(), expected_body);
    }

    #[test]
    fn assets_no_compression_if_not_exists() {
        let router = build_simple_router(|route| {
            route.get("/*").to_dir(
                FileOptions::new("resources/test/assets_uncompressed")
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

        let expected_body = fs::read("resources/test/assets_uncompressed/doc.html").unwrap();
        assert_eq!(response.read_body().unwrap(), expected_body);
    }

    #[test]
    fn assets_weighted_accept_encoding() {
        let router = build_simple_router(|route| {
            route.get("/*").to_dir(
                FileOptions::new("resources/test/assets")
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
        let expected_body = fs::read("resources/test/assets/doc.html.br").unwrap();
        assert_eq!(response.read_body().unwrap(), expected_body);
    }

    #[test]
    fn assets_range_request() {
        let root = PathBuf::from("resources/test/assets");
        let file_name = "doc.html";
        let mut file = File::open(root.join(file_name)).unwrap();
        let file_len = file.metadata().unwrap().len();
        let router = build_simple_router(|route| route.get("/*").to_dir(root));
        let server = TestServer::new(router).unwrap();

        let tests = [
            (Some(1), Some(123456789), 1, file_len - 1),
            (None, Some(5), file_len - 5, 5),
            (Some(5), None, 5, file_len - 5),
            (Some(5), Some(5), 5, 1),
            (Some(6), Some(5), 0, 0),
        ];

        for (range_begin, range_end, range_start, range_len) in tests {
            let range_header = format!(
                "bytes={}-{}",
                range_begin.map(|i| i.to_string()).unwrap_or("".to_string()),
                range_end.map(|i| i.to_string()).unwrap_or("".to_string())
            );
            let response = server
                .client()
                .get(format!("http://localhost/{file_name}"))
                .with_header(RANGE, HeaderValue::from_str(&range_header).unwrap())
                .perform()
                .unwrap();
            if range_start == 0 && range_len == 0 {
                assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
                break;
            }
            assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
            file.seek(SeekFrom::Start(range_start)).unwrap();

            let expected_content_range = format!(
                "bytes {}-{}/{}",
                range_start,
                range_start + range_len - 1,
                file_len
            );
            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_RANGE)
                    .unwrap()
                    .to_str()
                    .unwrap(),
                expected_content_range
            );
            let mut expected_body = vec![0; range_len as usize];
            file.read_exact(&mut expected_body).unwrap();
            assert_eq!(response.read_body().unwrap(), expected_body);
        }
    }

    fn test_server() -> TestServer {
        TestServer::new(static_router("/*", "resources/test/assets")).unwrap()
    }

    fn static_router(mount: &str, path: &str) -> Router {
        build_simple_router(|route| route.get(mount).to_dir(path))
    }
}
