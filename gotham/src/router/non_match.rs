//! Defines the types used to indicate a non-matching route, and associated metadata.

use std::collections::HashSet;

use hyper::{Method, StatusCode};

/// The error type used for a non-matching route, as returned by `RouteMatcher::is_match`. Multiple
/// values of this type can be aggregated with the `intersection` / `union` methods to be returned.
/// The data within is used by the `Router` to create a `Response` when no routes were successfully
/// matched.
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
/// struct MyRouteMatcher;
///
/// impl RouteMatcher for MyRouteMatcher {
///     fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
///         match state.borrow::<Method>() {
///             &Method::Get => Ok(()),
///             _ => Err(RouteNonMatch::new(StatusCode::MethodNotAllowed)
///                 .with_allow_list(&[Method::Get])),
///         }
///     }
/// }
/// #
/// # fn main() {
/// #   let mut state = State::new();
/// #   state.put(Method::Post);
/// #   assert!(MyRouteMatcher.is_match(&state).is_err());
/// # }
/// ```
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
    /// This is typically for Gotham internal use, but may be useful for implementors of advanced
    /// `RouteMatcher` logic which wraps other `RouteMatcher` instances. See the
    /// `gotham::router::route::matcher::AndRouteMatcher` implementation for an example.
    pub fn intersection(self, other: RouteNonMatch) -> RouteNonMatch {
        let status = higher_precedence_status(self.status, other.status);
        let allow = self.allow.intersection(other.allow);
        RouteNonMatch { status, allow }
    }

    /// Takes the union of two `RouteNonMatch` values, producing a single result. This is intended
    /// for use in cases where two `RouteMatcher` instances with a logical **OR** connection have
    /// both indicated a non-match, and their results need to be aggregated.
    ///
    /// This is typically for Gotham internal use, but may be useful for implementors of advanced
    /// `RouteMatcher` logic which wraps other `RouteMatcher` instances. See the
    /// `gotham::router::tree::Node::select_route` implementation for an example.
    pub fn union(self, other: RouteNonMatch) -> RouteNonMatch {
        let status = higher_precedence_status(self.status, other.status);
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

fn higher_precedence_status(lhs: StatusCode, rhs: StatusCode) -> StatusCode {
    use hyper::StatusCode::*;

    match (lhs, rhs) {
        // For 404, prefer routes that indicated *some* kind of match.
        (NotFound, _) => rhs,
        (_, NotFound) => lhs,
        // For 405, prefer routes that matched the HTTP method.
        (MethodNotAllowed, _) => rhs,
        (_, MethodNotAllowed) => lhs,
        // For 406, allow "harder" errors to overrule.
        (NotAcceptable, _) => rhs,
        (_, NotAcceptable) => lhs,
        // This is a silly safeguard that prefers errors over non-errors. This should never be
        // needed, but guards against strange custom `RouteMatcher` impls in applications.
        (_, _) if lhs.is_client_error() => lhs,
        (_, _) if rhs.is_client_error() => rhs,
        (_, _) => lhs,
    }
}

// This customised set prevents memory allocations while computing `Allow` lists, except in the
// case where extension methods are provided using the `hyper::Method::Extension` variant.
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
    fn intersection(self, other: MethodSet) -> MethodSet {
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

impl Default for MethodSet {
    fn default() -> MethodSet {
        MethodSet {
            connect: false,
            delete: true,
            get: true,
            head: true,
            options: true,
            patch: true,
            post: true,
            put: true,
            trace: false,
            other: HashSet::default(),
        }
    }
}

impl<'a> From<&'a [Method]> for MethodSet {
    fn from(methods: &[Method]) -> MethodSet {
        use hyper::Method::*;

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
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        );

        let mut other = HashSet::new();

        for method in methods {
            match *method {
                Connect => {
                    connect = true;
                }
                Delete => {
                    delete = true;
                }
                Get => {
                    get = true;
                }
                Head => {
                    head = true;
                }
                Options => {
                    options = true;
                }
                Patch => {
                    patch = true;
                }
                Post => {
                    post = true;
                }
                Put => {
                    put = true;
                }
                Trace => {
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
        use hyper::Method::*;

        let MethodSet {
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
        } = method_set;

        let mut result = None.into_iter()
            .chain(Some(Connect).into_iter().filter(|_| connect))
            .chain(Some(Delete).into_iter().filter(|_| delete))
            .chain(Some(Get).into_iter().filter(|_| get))
            .chain(Some(Head).into_iter().filter(|_| head))
            .chain(Some(Options).into_iter().filter(|_| options))
            .chain(Some(Patch).into_iter().filter(|_| patch))
            .chain(Some(Post).into_iter().filter(|_| post))
            .chain(Some(Put).into_iter().filter(|_| put))
            .chain(Some(Trace).into_iter().filter(|_| trace))
            .chain(other.into_iter())
            .collect::<Vec<Method>>();

        result.sort_unstable_by(|a, b| a.as_ref().cmp(b.as_ref()));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::Method::*;
    use hyper::StatusCode::*;

    #[test]
    fn intersection_tests() {
        let all = [Delete, Get, Head, Options, Patch, Post, Put];

        let (status, allow_list) = RouteNonMatch::new(NotFound)
            .intersection(RouteNonMatch::new(NotFound))
            .deconstruct();
        assert_eq!(status, NotFound);
        assert_eq!(&allow_list[..], &all);

        let (status, allow_list) = RouteNonMatch::new(NotFound)
            .intersection(RouteNonMatch::new(MethodNotAllowed).with_allow_list(&[Get]))
            .deconstruct();
        assert_eq!(status, MethodNotAllowed);
        assert_eq!(&allow_list[..], &[Get]);

        let (status, allow_list) = RouteNonMatch::new(NotAcceptable)
            .with_allow_list(&[Get, Patch, Post])
            .intersection(
                RouteNonMatch::new(MethodNotAllowed).with_allow_list(&[Get, Post, Options]),
            )
            .deconstruct();
        assert_eq!(status, NotAcceptable);
        assert_eq!(&allow_list[..], &[Get, Post]);
    }

    #[test]
    fn union_tests() {
        let all = [Delete, Get, Head, Options, Patch, Post, Put];

        let (status, allow_list) = RouteNonMatch::new(NotFound)
            .union(RouteNonMatch::new(NotFound))
            .deconstruct();
        assert_eq!(status, NotFound);
        assert_eq!(&allow_list[..], &all);

        let (status, allow_list) = RouteNonMatch::new(NotFound)
            .union(RouteNonMatch::new(MethodNotAllowed).with_allow_list(&[Get]))
            .deconstruct();
        assert_eq!(status, MethodNotAllowed);
        assert_eq!(&allow_list[..], &all);

        let (status, allow_list) = RouteNonMatch::new(NotAcceptable)
            .with_allow_list(&[Get, Patch, Post])
            .union(RouteNonMatch::new(MethodNotAllowed).with_allow_list(&[Get, Post, Options]))
            .deconstruct();
        assert_eq!(status, NotAcceptable);
        assert_eq!(&allow_list[..], &[Get, Options, Patch, Post]);
    }

    #[test]
    fn deconstruct_tests() {
        let (_, allow_list) = RouteNonMatch::new(NotFound)
            .with_allow_list(&[
                Delete,
                Get,
                Head,
                Options,
                Patch,
                Post,
                Put,
                Connect,
                Trace,
                Extension("PROPFIND".to_owned()),
                Extension("PROPSET".to_owned()),
            ])
            .deconstruct();

        assert_eq!(
            &allow_list[..],
            &[
                Connect,
                Delete,
                Get,
                Head,
                Options,
                Patch,
                Post,
                Extension("PROPFIND".to_owned()),
                Extension("PROPSET".to_owned()),
                Put,
                Trace,
            ]
        );
    }
}
