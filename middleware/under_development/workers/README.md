# Workers Middleware

The workers middleware creates a single thread pool for a Gotham app to run
tasks in the "background", without blocking the main event loop. The thread pool
can process blocking I/O and long-running computation without degrading the
performance of other requests.

**This middleware is under active development**

## Performance caveats

Executing synchronous logic in a background thread is not intended to be a
perfect replacement for asynchronous logic.

Use of this middleware is a trade-off, the benefits of which depend on the task.

## License

Licensed under your option of:

* [MIT License](../LICENSE-MIT)
* [Apache License, Version 2.0](../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
