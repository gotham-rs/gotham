# BorrowBag

A type-safe, heterogeneous collection with zero-cost add and borrow.

`BorrowBag` allows the storage of any value, and returns a `Handle` which can be
used to borrow the value back later. As the `BorrowBag` is add-only, `Handle`
values remain valid for the lifetime of the `BorrowBag`.

For usage details, please see the [documentation](https://docs.rs/borrow-bag/)

## Motivation

`BorrowBag` solves the problem of assembling Gotham's `Middleware` and `Pipeline` structures,
storing concrete types without losing their type information, and with an ability to borrow them
back later after moving the collection.

The Gotham project extracted the implementation into this crate for use in other contexts and
continues to maintain it.

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
