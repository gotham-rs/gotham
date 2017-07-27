//! Defines the X-Runtime-Microseconds header.

header! {
    /// Defines the X-Runtime header used to hold the number of microseconds taken for a response
    /// to be sent to the client.
    ///
    /// No formal specification/RFC exists for this header.
    ///
    /// # Example
    /// ```
    /// # extern crate hyper;
    /// # extern crate gotham;
    ///
    /// use hyper::header::Headers;
    /// use gotham::http::header::XRuntimeMicroseconds;
    ///
    /// # fn main () {
    /// let mut headers = Headers::new();
    /// headers.set(XRuntimeMicroseconds(91));
    /// # }
    /// ```
    (XRuntimeMicroseconds, "X-Runtime-Microseconds") => [i64]
}
