//! The router trait and its implementation on tide::Server connect the RouteBuilder to tide and
//! allows you to call register on a tide::Server with a fluent route tree

use std::sync::Arc;

use crate::{EndpointDescriptor, RouteSegment};
use crate::routebuilder::RouteBuilder;
use tide::{Endpoint, Middleware, http::Method};

/// A router is any component where routes can be registered on like a tide::Server
pub trait Router<State: Clone + Send + Sync + 'static> {
    /// Register a single endpoint on the `Router`
    fn register_endpoint(
        &mut self,
        path: &str,
        method: Option<Method>,
        middleware: &[Arc<dyn Middleware<State>>],
        endpoint: impl Endpoint<State>,
    );

    /// Register all routes from a RouteBuilder on the `Router`
    fn register<T: RouteBuilder<State>>(&mut self, builder: RouteSegment<State>) {
        for EndpointDescriptor(path, method, middleware, endpoint) in builder.build() {
            self.register_endpoint(&path, method, &middleware, endpoint)
        }
    }
}

impl<State: Clone + Send + Sync + 'static> Router<State> for tide::Server<State> {
    fn register_endpoint(
        &mut self,
        path: &str,
        method: Option<Method>,
        _middleware:  &[Arc<dyn Middleware<State>>],
        endpoint: impl Endpoint<State>,
    ) {
        let route = self.at(path);
        // let endpoint = MiddlewareEndpoint::wrap_with_middleware(endpoint, &middleware);

        // if method is specified then register this method, otherwise register endpoint as a catch_all
        match method {
            Some(method) => self.at(path).method(method, endpoint),
            None => self.at(path).all(endpoint),
        };
    }
}