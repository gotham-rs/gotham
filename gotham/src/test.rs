/// Test request behavior, shared between the tls::test and plain::test modules.
pub mod request;

pub use super::plain::test::TestServer;
pub use self::request::TestRequest;
