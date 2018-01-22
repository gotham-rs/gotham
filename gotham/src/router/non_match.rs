use std::collections::HashSet;

use hyper::{Method, StatusCode};

pub struct RouteNonMatch {
    status: StatusCode,
    allow: HashSet<Method>,
}

impl RouteNonMatch {
    pub(super) fn new(status: StatusCode) -> RouteNonMatch {
        use hyper::Method::*;

        RouteNonMatch {
            status,
            allow: [Options, Get, Post, Put, Delete, Head, Patch]
                .into_iter()
                .cloned()
                .collect(),
        }
    }

    pub(super) fn with_allow_list(self, allow: &[Method]) -> RouteNonMatch {
        RouteNonMatch {
            allow: allow.into_iter().cloned().collect(),
            ..self
        }
    }

    pub(super) fn intersection(self, other: RouteNonMatch) -> RouteNonMatch {
        let status = higher_precedence_status(self.status, other.status);
        let allow = self.allow.intersection(&other.allow).cloned().collect();
        RouteNonMatch { status, allow }
    }

    pub(super) fn union(self, other: RouteNonMatch) -> RouteNonMatch {
        let status = higher_precedence_status(self.status, other.status);
        let allow = self.allow.union(&other.allow).cloned().collect();
        RouteNonMatch { status, allow }
    }

    pub(super) fn deconstruct(self) -> (StatusCode, Vec<Method>) {
        let RouteNonMatch { status, allow } = self;
        let mut allow: Vec<Method> = allow.into_iter().collect();
        allow.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
        (status, allow)
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
