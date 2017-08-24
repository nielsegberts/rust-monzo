# Monzo API for Rust

This is a library that wraps over the Monzo API in a future aware manner.

## Example usage

```rust
extern crate monzo;
extern crate tokio_core;

let mut core = tokio_core::reactor::Core::new().unwrap();
let monzo = monzo::Client::new(&core.handle());
let work = monzo.balance("<account_id>".to_string(), "<access_token>".to_string());
let response = core.run(work).unwrap();
println!("Balance: {} {}", response.balance, response.currency);
println!("Spent today: {}", response.spend_today);
```

## Tests

Tests use [mockito](https://crates.io/crates/mockito) so need to be ran one a the time:

```
cargo test -- --test-threads=1
```

## Thanks to

Inspired by [citymapper-rs](https://crates.io/crates/citymapper) and [monzo-rust](https://github.com/llompartg/monzo-rust).
