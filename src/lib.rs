//! Tide Fluent Routes is a fluent api to define routes for the Tide HTTP framework.
//! At the moment it supports setting up paths, you can integrate middleware at any place in the
//! route-tree and you can integrate endpoints.
//! Some things that are possible with Tide-native routes are not (yet) possible;
//! - Tide prefix routes are not implemented
//! - you can not nest Tide servers
//!
//! To use this you can import Tide Fluent Routes with `use tide_fluent_routes::prelude::*` it
//! introduces the `register` extension method on the `Tide::Server` to register routes from a
//! RouteBuilder.
//! A RouteBuilder can be initialized using the `route()` method.
//! You can register simple endpoints like this;
//! ```rust
//! # use tide::{Request, Result};
//! # pub async fn endpoint(_: Request<()>) -> Result {
//! #     todo!()
//! # }
//! use tide_fluent_routes::prelude::*;
//!
//! let mut server = tide::Server::new();
//!
//! server.register(
//!    root()
//!        .get(endpoint)
//!        .post(endpoint)
//!    ).expect("Error setting up routes");
//! ```
//! Fluent Routes follows conventions from Tide. All HTTP verbs are supported the same way. Paths
//! can be extended using `at` but this method takes a router closure that allows building the route
//! as a tree.
//! A complete route tree can be defined like this;
//! ```rust
//! # use tide::{Request, Result};
//! # use tide_fluent_routes::prelude::*;
//! # async fn endpoint(_: Request<()>) -> Result {
//! #     todo!()
//! # }
//! # let mut server = tide::Server::new();
//!
//! server.register(
//!     root()
//!         .get(endpoint)
//!         .post(endpoint)
//!         .at("api/v1", |route| route
//!             .get(endpoint)
//!             .post(endpoint)
//!         )
//!         .at("api/v2", |route| route
//!             .get(endpoint)
//!             .post(endpoint)
//!         )
//! ).expect("Error setting up routes");
//! ```
//! This eliminates the need to introduce variables for partial pieces of your route tree.
//!
//! Including routes defined in other functions also looks very natural, this makes it easy
//! to compose large route trees from smaller trees defined elsewhere;
//! ```rust
//! # use tide::{Request, Result};
//! # use tide_fluent_routes::prelude::*;
//! # async fn endpoint(_: Request<()>) -> Result {
//! #     todo!()
//! # }
//! # let mut server = tide::Server::new();
//!
//! fn v1_routes(routes: SubRoute<()>) -> SubRoute<()> {
//!     routes
//!         .at("articles", |route| route
//!             .get(endpoint)
//!             .post(endpoint)
//!             .at(":id", |route| route
//!                 .get(endpoint)
//!                 .put(endpoint)
//!                 .delete(endpoint)
//!             )
//!         )
//! }
//!
//! fn v2_routes(routes: SubRoute<()>) -> SubRoute<()> {
//!     routes
//!         .at("articles", |route| route
//!             .get(endpoint))
//! }
//!
//! server.register(
//!     root()
//!         .get(endpoint)
//!         .post(endpoint)
//!         .at("api/v1", v1_routes)
//!         .at("api/v2", v2_routes)
//! ).expect("Error setting up routes");
//! ```
//!
//! With vanilla Tide routes it can be hard to see what middleware is active for what
//! endpoints.
//! Adding middleware to a tree is easy, and its very clear where the middleware is applied;
//! ```rust
//! # use std::{future::Future, pin::Pin};
//! # use tide::{Next, Request, Result};
//! # use tide_fluent_routes::prelude::*;
//! # async fn endpoint(_: Request<()>) -> Result {
//! #     todo!()
//! # }
//! # fn dummy_middleware<'a>(
//! #     request: Request<()>,
//! #     next: Next<'a, ()>,
//! # ) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>> {
//! #     Box::pin(async { Ok(next.run(request).await) })
//! # }
//! # let mut server = tide::Server::new();
//! server.register(
//!     root()
//!         .get(endpoint)
//!         .post(endpoint)
//!         .at("api/v1", |route| route
//!             .with(dummy_middleware, |route| route
//!                 .get(endpoint)
//!             )
//!             .post(endpoint)
//!         )
//!         .at("api/v2", |route| route
//!             .get(endpoint)
//!             .get(endpoint)
//!         ),
//! );
//! ```
//!
//! Serving directories is possible using `serve_dir`, this works the same as with normal Tide routes,
//! fluent routes adds the `serve_file` convenience method for serving single files.
//! ```rust,no_run
//! # use tide::{Request, Result};
//! use tide_fluent_routes::prelude::*;
//! use tide_fluent_routes::fs::ServeFs;
//!
//! let mut server = tide::Server::new();
//!
//! server.register(
//!     root()
//!         .serve_file("files/index.html").unwrap()
//!         .at("img", |r| r
//!             .serve_dir("files/images").unwrap()
//!         )
//! );
//! ```

// Turn on warnings for some lints
#![warn(
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_import_braces,
    unused_qualifications
)]

pub mod fs;
mod path;
pub mod reverse_router;
pub mod routebuilder;
pub mod router;
mod routesegment;
mod util;

use std::collections::HashMap;
pub use tide::Error;

/// The result type for fluent routing
pub type Result<T> = std::result::Result<T, Error>;

/// Import types to use tide_fluent_routes
pub mod prelude {
    pub use super::reverse_router::ReverseRouter;
    pub use super::routebuilder::{RouteBuilder, RouteBuilderExt};
    pub use super::router::Router;
    pub use super::routesegment::{root, RouteSegment, SubRoute};
    pub use tide::http::Method;
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use crate::util::ArcMiddleware;
    use std::future::Future;
    use std::pin::Pin;
    use tide::{Next, Request, Result};

    #[test]
    fn should_build_single_endpoint() {
        let routes: Vec<_> = root::<()>().get(|_| async { Ok("") }).unwrap().build();

        assert_eq!(routes.len(), 1);
    }

    #[test]
    fn should_build_multiple_endpoints() {
        let routes: Vec<_> = root::<()>()
            .get(|_| async { Ok("") })
            .post(|_| async { Ok("") })
            .unwrap()
            .build();

        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn should_build_sub_endpoints() {
        let routes: Vec<_> = root::<()>()
            .at("sub_path", |r| {
                r.get(|_| async { Ok("") }).post(|_| async { Ok("") })
            })
            .unwrap()
            .build();

        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn should_build_endpoint_path() {
        let routes: Vec<_> = root::<()>()
            .at("path", |r| r.at("subpath", |r| r.get(|_| async { Ok("") })))
            .unwrap()
            .build();

        assert_eq!(routes.len(), 1);
        // TODO: Fix this, possibly with a named endpoint
        // assert_eq!(routes.get(0).unwrap().route, Some(Method::Get));
        assert_eq!(
            routes.get(0).unwrap().path.to_string(),
            "/path/subpath".to_string()
        );
    }

    #[test]
    fn should_start_path_with_slash() {
        let routes: Vec<_> = root::<()>().get(|_| async { Ok("") }).unwrap().build();
        assert_eq!(routes.get(0).unwrap().path.to_string(), "/".to_string());
    }

    fn middleware<'a>(
        request: Request<()>,
        next: Next<'a, ()>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>> {
        Box::pin(async { Ok(next.run(request).await) })
    }

    #[test]
    fn should_collect_middleware() {
        let middleware1 = ArcMiddleware::new(middleware);
        let middleware2 = ArcMiddleware::new(middleware);

        let routes: Vec<_> = root::<()>()
            .at("path", |r| {
                r.with(middleware1.clone(), |r| {
                    r.at("subpath", |r| {
                        r.with(middleware2.clone(), |r| r.get(|_| async { Ok("") }))
                    })
                    .get(|_| async { Ok("") })
                })
            })
            .unwrap()
            .build();

        assert_eq!(routes.get(0).unwrap().middleware.len(), 1);
        assert_eq!(routes.get(1).unwrap().middleware.len(), 2);
    }
}
