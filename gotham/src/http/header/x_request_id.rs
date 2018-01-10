//! Defines the X-Request-ID header.

header! {
    /// Defines the X-Request-ID header.
    ///
    /// If present in a request this value will be assigned into to the requests unique id. Formats
    /// are not dicated when upstream systems provide this value however they must be unique.
    ///
    /// No formal specification/RFC exists for this header.
    ///
    /// # Example
    /// ```
    /// # extern crate hyper;
    /// # extern crate gotham;
    ///
    /// use hyper::header::Headers;
    /// use gotham::http::header::XRequestId;
    ///
    /// # fn main () {
    /// let mut headers = Headers::new();
    /// headers.set(XRequestId(String::from("123")));
    /// # }
    /// ```
    (XRequestId, "X-Request-ID") => [String]
}
