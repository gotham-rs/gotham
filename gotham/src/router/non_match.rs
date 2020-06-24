//! Defines the types used to indicate a non-matching route, and associated metadata.

use std::collections::HashSet;

use hyper::{Method, StatusCode};

/// The error type used for a non-matching route, as returned by `RouteMatcher::is_match`. Multiple
/// values of this type can be combined by matchers that are wrapping other matchers, using the
/// `intersection` / `union` methods.  The data within is used by the `Router` to create a
/// `Response` when no routes were successfully matched.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Method, StatusCode};
/// # use gotham::router::non_match::RouteNonMatch;
/// # use gotham::router::route::matcher::RouteMatcher;
/// # use gotham::state::State;
/// #
/// #[derive(Clone)]
/// struct MyRouteMatcher;
///
/// impl RouteMatcher for MyRouteMatcher {
///     fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
///         match state.borrow::<Method>() {
///             &Method::GET => Ok(()),
///             _ => Err(
///                 RouteNonMatch::new(StatusCode::METHOD_NOT_ALLOWED)
///                     .with_allow_list(&[Method::GET]),
///             ),
///         }
///     }
/// }
/// #
/// # fn main() {
/// #     State::with_new(|state| {
/// #         state.put(Method::POST);
/// #         assert!(MyRouteMatcher.is_match(&state).is_err());
/// #     });
/// # }
/// ```
#[derive(Clone)]
pub struct RouteNonMatch {
    status: StatusCode,
    allow: MethodSet,
}

impl RouteNonMatch {
    /// Creates a new `RouteNonMatch` value with the given HTTP status.
    pub fn new(status: StatusCode) -> RouteNonMatch {
        RouteNonMatch {
            status,
            allow: MethodSet::default(),
        }
    }

    /// Adds an allow list to a `RouteNonMatch`. Typically this is used with a `405 Method Not
    /// Allowed` status code, so the `Router` can accurately populate the `Allow` header in the
    /// response. It must be populated by any `RouteMatcher` which restricts the HTTP method.
    pub fn with_allow_list(self, allow: &[Method]) -> RouteNonMatch {
        RouteNonMatch {
            allow: allow.into(),
            ..self
        }
    }

    /// Takes the intersection of two `RouteNonMatch` values, producing a single result.  This is
    /// intended for use in cases where two `RouteMatcher` instances with a logical **AND**
    /// connection have both indicated a non-match, and their results need to be aggregated.
    ///
    /// This is typically for Gotham internal use, but may be useful for implementors of matchers
    /// which wrap other `RouteMatcher` instances. See the `AndRouteMatcher` implementation (in
    /// `gotham::router::route::matcher::and`) for an example.
    pub fn intersection(self, other: RouteNonMatch) -> RouteNonMatch {
        let status = match (self.status, other.status) {
            // A mismatched method combined with another error doesn't make the method match.
            (StatusCode::METHOD_NOT_ALLOWED, _) | (_, StatusCode::METHOD_NOT_ALLOWED) => {
                StatusCode::METHOD_NOT_ALLOWED
            }
            // For 404, prefer routes that indicated *some* kind of match.
            (StatusCode::NOT_FOUND, rhs) if rhs.is_client_error() => rhs,
            (lhs, StatusCode::NOT_FOUND) if lhs.is_client_error() => lhs,
            // For 406, allow "harder" errors to overrule.
            (StatusCode::NOT_ACCEPTABLE, rhs) if rhs.is_client_error() => rhs,
            (lhs, StatusCode::NOT_ACCEPTABLE) if lhs.is_client_error() => lhs,
            // This is a silly safeguard that prefers errors over non-errors. This should never be
            // needed, but guards against strange custom `RouteMatcher` impls in applications.
            (lhs, _) if lhs.is_client_error() => lhs,
            (_, rhs) if rhs.is_client_error() => rhs,
            (lhs, _) => lhs,
        };
        let allow = self.allow.intersection(other.allow);
        RouteNonMatch { status, allow }
    }

    /// Takes the union of two `RouteNonMatch` values, producing a single result. This is intended
    /// for use in cases where two `RouteMatcher` instances with a logical **OR** connection have
    /// both indicated a non-match, and their results need to be aggregated.
    ///
    /// This is typically for Gotham internal use, but may be useful for implementors of matchers
    /// which wrap other `RouteMatcher` instances. See the `Node::select_route` implementation (in
    /// `gotham::router::tree`) for an example.
    pub fn union(self, other: RouteNonMatch) -> RouteNonMatch {
        let status = match (self.status, other.status) {
            // For 405, prefer routes that matched the HTTP method, even if they haven't found the
            // requested resource and returned a 404.
            (StatusCode::METHOD_NOT_ALLOWED, rhs) if rhs.is_client_error() => rhs,
            (lhs, StatusCode::METHOD_NOT_ALLOWED) if lhs.is_client_error() => lhs,
            // For 404, prefer routes that indicated *some* kind of match.
            (StatusCode::NOT_FOUND, rhs) if rhs.is_client_error() => rhs,
            (lhs, StatusCode::NOT_FOUND) if lhs.is_client_error() => lhs,
            // For 406, allow "harder" errors to overrule.
            (StatusCode::NOT_ACCEPTABLE, rhs) if rhs.is_client_error() => rhs,
            (lhs, StatusCode::NOT_ACCEPTABLE) if lhs.is_client_error() => lhs,
            // This is a silly safeguard that prefers errors over non-errors. This should never be
            // needed, but guards against strange custom `RouteMatcher` impls in applications.
            (lhs, _) if lhs.is_client_error() => lhs,
            (_, rhs) if rhs.is_client_error() => rhs,
            (lhs, _) => lhs,
        };
        let allow = self.allow.union(other.allow);
        RouteNonMatch { status, allow }
    }

    pub(super) fn deconstruct(self) -> (StatusCode, Vec<Method>) {
        (self.status, self.allow.into())
    }
}

impl From<RouteNonMatch> for StatusCode {
    fn from(val: RouteNonMatch) -> StatusCode {
        val.status
    }
}

// This customised set prevents memory allocations while computing `Allow` lists, except in the
// case where extension methods are provided using the `hyper::Method::Extension` variant.
#[derive(Clone, Default)]
struct MethodSet {
    connect: bool,
    delete: bool,
    get: bool,
    head: bool,
    options: bool,
    patch: bool,
    post: bool,
    put: bool,
    trace: bool,
    other: HashSet<Method>,
}

impl MethodSet {
    fn is_empty(&self) -> bool {
        !self.connect
            && !self.delete
            && !self.get
            && !self.head
            && !self.options
            && !self.patch
            && !self.post
            && !self.put
            && !self.trace
            && self.other.is_empty()
    }

    fn intersection(self, other: MethodSet) -> MethodSet {
        if self.is_empty() {
            other
        } else if other.is_empty() {
            self
        } else {
            MethodSet {
                connect: self.connect && other.connect,
                delete: self.delete && other.delete,
                get: self.get && other.get,
                head: self.head && other.head,
                options: self.options && other.options,
                patch: self.patch && other.patch,
                post: self.post && other.post,
                put: self.put && other.put,
                trace: self.trace && other.trace,
                other: self.other.intersection(&other.other).cloned().collect(),
            }
        }
    }

    fn union(self, other: MethodSet) -> MethodSet {
        MethodSet {
            connect: self.connect || other.connect,
            delete: self.delete || other.delete,
            get: self.get || other.get,
            head: self.head || other.head,
            options: self.options || other.options,
            patch: self.patch || other.patch,
            post: self.post || other.post,
            put: self.put || other.put,
            trace: self.trace || other.trace,
            other: self.other.union(&other.other).cloned().collect(),
        }
    }
}

impl<'a> From<&'a [Method]> for MethodSet {
    fn from(methods: &[Method]) -> MethodSet {
        let (
            mut connect,
            mut delete,
            mut get,
            mut head,
            mut options,
            mut patch,
            mut post,
            mut put,
            mut trace,
        ) = (
            false, false, false, false, false, false, false, false, false,
        );

        let mut other = HashSet::new();

        for method in methods {
            match *method {
                Method::CONNECT => {
                    connect = true;
                }
                Method::DELETE => {
                    delete = true;
                }
                Method::GET => {
                    get = true;
                }
                Method::HEAD => {
                    head = true;
                }
                Method::OPTIONS => {
                    options = true;
                }
                Method::PATCH => {
                    patch = true;
                }
                Method::POST => {
                    post = true;
                }
                Method::PUT => {
                    put = true;
                }
                Method::TRACE => {
                    trace = true;
                }
                _ => {
                    other.insert(method.clone());
                }
            }
        }

        MethodSet {
            connect,
            delete,
            get,
            head,
            options,
            patch,
            post,
            put,
            trace,
            other,
        }
    }
}

impl From<MethodSet> for Vec<Method> {
    fn from(method_set: MethodSet) -> Vec<Method> {
        let methods_with_flags: [(Method, bool); 9] = [
            (Method::CONNECT, method_set.connect),
            (Method::DELETE, method_set.delete),
            (Method::GET, method_set.get),
            (Method::HEAD, method_set.head),
            (Method::OPTIONS, method_set.options),
            (Method::PATCH, method_set.patch),
            (Method::POST, method_set.post),
            (Method::PUT, method_set.put),
            (Method::TRACE, method_set.trace),
        ];

        let mut result = methods_with_flags
            .iter()
            .filter_map(|&(ref method, flag)| if flag { Some(method.clone()) } else { None })
            .chain(method_set.other.into_iter())
            .collect::<Vec<Method>>();

        result.sort_unstable_by(|a, b| a.as_ref().cmp(b.as_ref()));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::{Method, StatusCode};

    trait AllowList {
        fn apply_allow_list(self, list: Option<&[Method]>) -> Self;
    }

    impl AllowList for RouteNonMatch {
        fn apply_allow_list(self, list: Option<&[Method]>) -> Self {
            match list {
                Some(list) => self.with_allow_list(list),
                None => self,
            }
        }
    }

    const ALL: [Method; 7] = [
        Method::DELETE,
        Method::GET,
        Method::HEAD,
        Method::OPTIONS,
        Method::PATCH,
        Method::POST,
        Method::PUT,
    ];

    fn intersection_assert_status_code(code1: StatusCode, code2: StatusCode, expected: StatusCode) {
        let (status, _) = RouteNonMatch::new(code1)
            .intersection(RouteNonMatch::new(code2))
            .deconstruct();
        assert_eq!(status, expected);
        let (status, _) = RouteNonMatch::new(code2)
            .intersection(RouteNonMatch::new(code1))
            .deconstruct();
        assert_eq!(status, expected);
    }

    #[test]
    fn intersection_test_status_code() {
        intersection_assert_status_code(
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::NOT_FOUND,
            StatusCode::METHOD_NOT_ALLOWED,
        );
        intersection_assert_status_code(
            StatusCode::NOT_ACCEPTABLE,
            StatusCode::NOT_FOUND,
            StatusCode::NOT_ACCEPTABLE,
        );
        intersection_assert_status_code(
            StatusCode::NOT_ACCEPTABLE,
            StatusCode::FORBIDDEN,
            StatusCode::FORBIDDEN,
        );
        intersection_assert_status_code(
            StatusCode::OK,
            StatusCode::NOT_FOUND,
            StatusCode::NOT_FOUND,
        );
        intersection_assert_status_code(
            StatusCode::OK,
            StatusCode::NOT_ACCEPTABLE,
            StatusCode::NOT_ACCEPTABLE,
        );

        let (status, _) = RouteNonMatch::new(StatusCode::OK)
            .intersection(RouteNonMatch::new(StatusCode::NO_CONTENT))
            .deconstruct();
        assert_eq!(status, StatusCode::OK);
        let (status, _) = RouteNonMatch::new(StatusCode::NO_CONTENT)
            .intersection(RouteNonMatch::new(StatusCode::OK))
            .deconstruct();
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    fn intersection_assert_allow_list(
        list1: Option<&[Method]>,
        list2: Option<&[Method]>,
        expected: &[Method],
    ) {
        let status = StatusCode::BAD_REQUEST;
        let (_, allow_list) = RouteNonMatch::new(status)
            .apply_allow_list(list1)
            .intersection(RouteNonMatch::new(status).apply_allow_list(list2))
            .deconstruct();
        assert_eq!(&allow_list, &expected);
        let (_, allow_list) = RouteNonMatch::new(status)
            .apply_allow_list(list2)
            .intersection(RouteNonMatch::new(status).apply_allow_list(list1))
            .deconstruct();
        assert_eq!(&allow_list, &expected);
    }

    #[test]
    fn intersection_test_allow_list() {
        intersection_assert_allow_list(None, None, &[]);
        intersection_assert_allow_list(Some(&ALL), None, &ALL);
        intersection_assert_allow_list(Some(&ALL), Some(&[Method::GET]), &[Method::GET]);
        intersection_assert_allow_list(None, Some(&[Method::GET]), &[Method::GET]);
        intersection_assert_allow_list(
            Some(&[Method::GET, Method::POST]),
            Some(&[Method::POST, Method::PUT]),
            &[Method::POST],
        );
    }

    fn union_assert_status_code(code1: StatusCode, code2: StatusCode, expected: StatusCode) {
        let (status, _) = RouteNonMatch::new(code1)
            .union(RouteNonMatch::new(code2))
            .deconstruct();
        assert_eq!(status, expected);
        let (status, _) = RouteNonMatch::new(code2)
            .union(RouteNonMatch::new(code1))
            .deconstruct();
        assert_eq!(status, expected);
    }

    #[test]
    fn union_test_status_code() {
        union_assert_status_code(
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::NOT_FOUND,
            StatusCode::NOT_FOUND,
        );
        union_assert_status_code(
            StatusCode::NOT_ACCEPTABLE,
            StatusCode::NOT_FOUND,
            StatusCode::NOT_ACCEPTABLE,
        );
        union_assert_status_code(
            StatusCode::NOT_ACCEPTABLE,
            StatusCode::FORBIDDEN,
            StatusCode::FORBIDDEN,
        );
        union_assert_status_code(StatusCode::OK, StatusCode::NOT_FOUND, StatusCode::NOT_FOUND);
        union_assert_status_code(
            StatusCode::OK,
            StatusCode::NOT_ACCEPTABLE,
            StatusCode::NOT_ACCEPTABLE,
        );

        let (status, _) = RouteNonMatch::new(StatusCode::OK)
            .union(RouteNonMatch::new(StatusCode::NO_CONTENT))
            .deconstruct();
        assert_eq!(status, StatusCode::OK);
        let (status, _) = RouteNonMatch::new(StatusCode::NO_CONTENT)
            .union(RouteNonMatch::new(StatusCode::OK))
            .deconstruct();
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    fn union_assert_allow_list(
        list1: Option<&[Method]>,
        list2: Option<&[Method]>,
        expected: &[Method],
    ) {
        let status = StatusCode::BAD_REQUEST;
        let (_, allow_list) = RouteNonMatch::new(status)
            .apply_allow_list(list1)
            .union(RouteNonMatch::new(status).apply_allow_list(list2))
            .deconstruct();
        assert_eq!(&allow_list, &expected);
        let (_, allow_list) = RouteNonMatch::new(status)
            .apply_allow_list(list2)
            .union(RouteNonMatch::new(status).apply_allow_list(list1))
            .deconstruct();
        assert_eq!(&allow_list, &expected);
    }

    #[test]
    fn union_test_allow_list() {
        union_assert_allow_list(None, None, &[]);
        union_assert_allow_list(Some(&ALL), None, &ALL);
        union_assert_allow_list(Some(&ALL), Some(&[Method::GET]), &ALL);
        union_assert_allow_list(None, Some(&[Method::GET]), &[Method::GET]);
        union_assert_allow_list(
            Some(&[Method::GET, Method::POST]),
            Some(&[Method::POST, Method::PUT]),
            &[Method::GET, Method::POST, Method::PUT],
        );
    }

    #[test]
    fn deconstruct_tests() {
        let (_, allow_list) = RouteNonMatch::new(StatusCode::NOT_FOUND)
            .with_allow_list(&[
                Method::DELETE,
                Method::GET,
                Method::HEAD,
                Method::OPTIONS,
                Method::PATCH,
                Method::POST,
                Method::PUT,
                Method::CONNECT,
                Method::TRACE,
                Method::from_bytes(b"PROPFIND").unwrap(),
                Method::from_bytes(b"PROPSET").unwrap(),
            ])
            .deconstruct();

        assert_eq!(
            &allow_list[..],
            &[
                Method::CONNECT,
                Method::DELETE,
                Method::GET,
                Method::HEAD,
                Method::OPTIONS,
                Method::PATCH,
                Method::POST,
                Method::from_bytes(b"PROPFIND").unwrap(),
                Method::from_bytes(b"PROPSET").unwrap(),
                Method::PUT,
                Method::TRACE,
            ]
        );
    }
}
