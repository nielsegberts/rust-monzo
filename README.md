# Monzo API for Rust

[![Build Status](https://travis-ci.org/nielsegberts/rust-monzo.svg?branch=master)](https://travis-ci.org/nielsegberts/rust-monzo)

This is a library that wraps over the Monzo API in a future aware manner.

## Example usage

```rust
extern crate monzo;
extern crate tokio_core;

let mut core = tokio_core::reactor::Core::new().unwrap();
let monzo = monzo::Client::new(&core.handle(), "<access_token>");
let work = monzo.balance("<account_id>".into());
let response = core.run(work).unwrap();
println!("Balance: {} {}", response.balance, response.currency);
println!("Spent today: {}", response.spend_today);
```

## Implemented endpoints

* accounts
* balance
* transactions (just listing)

Send me a pull request if you want to help out!

## Tests

Tests use [mockito](https://crates.io/crates/mockito) so need to be ran one at the time:

```
cargo test -- --test-threads=1
```

## Thanks to

Inspired by [citymapper-rs](https://crates.io/crates/citymapper) and [monzo-rust](https://github.com/llompartg/monzo-rust).
