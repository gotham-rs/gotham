//! Defines the X-Request-ID header.

header! {
    /// Defines the X-Request-ID header.
    ///
    /// If present in a request this value will be assigned into to the requests unique id. Formats
    /// are not dicated when upstream systems provide this value however they must be unique.
    (XRequestId, "X-Request-ID") => [String]
}
