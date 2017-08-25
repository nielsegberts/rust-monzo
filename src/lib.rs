//! A library for using the Monzo API
//!
//! This library wraps over the Monzo API in a future aware manner.
//!
//! Example usage:
//!
//! ```rust,no_run
//! extern crate monzo;
//! extern crate tokio_core;
//!
//! let mut core = tokio_core::reactor::Core::new().unwrap();
//! let monzo = monzo::Client::new(&core.handle(), "<access_token>");
//! let work = monzo.balance("<account_id>".into());
//! let response = core.run(work).unwrap();
//! println!("Balance: {} {}", response.balance, response.currency);
//! println!("Spent today: {}", response.spend_today);
//! ```

#![crate_name = "monzo"]
#![deny(missing_docs,
        missing_debug_implementations,
        missing_copy_implementations,
        trivial_casts,
        trivial_numeric_casts,
        unsafe_code,
        unused_extern_crates,
        unused_import_braces,
        unused_qualifications)]

#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate url;

use futures::{Future, Stream};
use hyper::{Body, Method, Request, Uri, Chunk, StatusCode};
use hyper::header::{Authorization, Bearer};
use std::string::String;
use tokio_core::reactor::Handle;
use url::Url;

/// Identifier for the account.
pub type AccountId = String;

/// Accounts represent a store of funds, and have a list of transactions.
#[derive(Debug, Deserialize)]
pub struct Account {
    /// The account id.
    pub id: AccountId,
    /// Description of the account.
    pub description: String,
    /// Date the account was created.
    // TODO: Change to date type?
    pub created: String,
}

/// Response to the list accounts future.
#[derive(Debug, Deserialize)]
pub struct Accounts {
    /// List of accounts owned by the currenty authorized user.
    pub accounts: Vec<Account>,
}

/// Response to the balance future if successful.
#[derive(Debug, Deserialize)]
pub struct Balance {
    /// The currently available balance of the account, as a 64bit integer in minor units of the
    /// currency, eg. pennies for GBP, or cents for EUR and USD.
    pub balance: i64,
    /// The ISO 4217 currency code.
    pub currency: String,
    /// The amount spent from this account today (considered from approx 4am onwards), in minor
    /// units of the currency.
    pub spend_today: i64,
}

/// Response to the futures in case of an error.
#[derive(Debug, Deserialize)]
pub struct Error {
    /// The HTTP response code.
    pub code: Option<String>,
    /// Short description of the error.
    // Maybe make this an enum since the documentation says it can only contain a certain set of
    // values
    pub error: Option<String>,
    /// Longer description of the error.
    pub error_description: Option<String>,
    /// Additional information.
    pub message: Option<String>,
}

/// Errors for this crate using error_chain.
pub mod errors {
    #![allow(missing_docs)]
    error_chain! {
        errors {
            /// When the Monzo API returns an error response code with more detailed information.
            BadResponse(statuscode: ::hyper::StatusCode, error: ::Error)
        }
        foreign_links {
            BadJsonResponse(::serde_json::Error);
            NetworkError(::hyper::Error);
        }
    }
}

/// The main interface for this crate.
#[derive(Debug)]
pub struct Client {
    client: hyper::client::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    access_token: String,
    base_url: Url,
}

/// The main interface for this crate.
impl Client {
    /// Creates a new Monzo client.
    pub fn new(handle: &Handle, access_token: &str) -> Client {
        Client::new_with_base_url(
            handle,
            access_token,
            "https://api.monzo.com".parse().unwrap(),
        )
    }

    /// Creates a new Monzo client with another base url. Useful for tests.
    pub fn new_with_base_url(handle: &Handle, access_token: &str, base_url: Url) -> Client {
        Client {
            client: ::hyper::Client::configure()
                .connector(::hyper_tls::HttpsConnector::new(4, handle).unwrap())
                .build(handle),
            access_token: access_token.into(),
            base_url: base_url,
        }
    }

    fn create_request(&self, uri: Uri) -> Request<Body> {
        let mut req: Request<Body> = Request::new(Method::Get, uri);
        req.headers_mut().set(Authorization(
            Bearer { token: self.access_token.clone() },
        ));
        req
    }

    fn make_request<T: 'static, F: 'static>(
        &self,
        uri: Uri,
        response_handler: F,
    ) -> Box<Future<Item = T, Error = errors::Error>>
    where
        F: Fn(Chunk) -> Result<T, errors::Error>,
    {
        let request = self.create_request(uri);
        let response: hyper::client::FutureResponse = self.client.request(request);
        let future = response
            .map_err(|err: hyper::Error| -> errors::Error { err.into() })
            .and_then(|res| {
                let status = res.status().clone();
                res.body()
                    .concat2()
                    .map_err(|err: hyper::Error| err.into())
                    .and_then(move |body: Chunk| {
                        match status {
                            StatusCode::Ok => {}
                            _ => {
                                let error: Error = serde_json::from_slice(&body)?;
                                return Err(errors::ErrorKind::BadResponse(status, error).into());
                            }
                        };
                        response_handler(body)
                    })
            });

        Box::new(future)
    }

    /// Returns a list of accounts owned by the currently authorised user.
    pub fn accounts(&self) -> Box<Future<Item = Accounts, Error = errors::Error>> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().unwrap().push("accounts");
        let uri: Uri = url.into_string().parse().unwrap();

        self.make_request(uri, |body| {
            let a: Accounts = serde_json::from_slice(&body)?;
            Ok(a)
        })
    }

    /// Retrieve information about an account’s balance.
    pub fn balance(
        &self,
        account_id: AccountId,
    ) -> Box<Future<Item = Balance, Error = errors::Error>> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().unwrap().push("balance");
        url.query_pairs_mut().append_pair("account_id", &account_id);
        let uri: Uri = url.into_string().parse().unwrap();

        self.make_request(uri, |body| {
            let b: Balance = serde_json::from_slice(&body)?;
            Ok(b)
        })
    }
}