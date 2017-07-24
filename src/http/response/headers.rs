//! Defines HTTP headers which are set by Gotham for various (often security) purposes that are not
//! defined in Hyper.

header! {
    /// Defines the X-Request-ID header for use with Hyper functions.
    ///
    /// If present in a request this value will be assigned into to the requests unique id. Formats
    /// are not dicated when upstream systems provide this value however they must be unique.
    (XRequestId, "X-Request-ID") => [String]
}
