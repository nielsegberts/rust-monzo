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
extern crate chrono;
extern crate futures;
extern crate hyper;
extern crate serde;
extern crate hyper_tls;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate url;

use chrono::DateTime;
use chrono::offset::Utc;
use futures::{Future, Stream};
use hyper::{Body, Method, Request, Uri, Chunk, StatusCode};
use hyper::header::{Authorization, Bearer};
use serde::de;
use serde::de::Deserialize;
use serde::de::Deserializer;
use serde::de::Visitor;
use std::collections::HashMap;
use std::str::FromStr;
use std::string::String;
use tokio_core::reactor::Handle;
use url::Url;

/// Identifier for an account.
pub type AccountId = String;
/// Identifier for a transaction.
pub type TransactionId = String;
/// Identifier of a merchant.
pub type MerchantId = String;
/// Holds an ISO 4217 currency code.
pub type Currency = String;

/// Accounts represent a store of funds, and have a list of transactions.
#[derive(Debug, Deserialize)]
pub struct Account {
    /// The account id.
    pub id: AccountId,
    /// Description of the account.
    pub description: String,
    /// The timestamp when the account was created.
    pub created: DateTime<Utc>,
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
    pub currency: Currency,
    /// The amount spent from this account today (considered from approx 4am onwards), in minor
    /// units of the currency.
    pub spend_today: i64,
}

/// Deserializes a string but returns None on empty string.
fn none_for_empty_string<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de> + FromStr,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl if the string is
    // non-empty The `PhantomData` is to keep the compiler from complaining about T being an unused
    // generic type parameter. We need T in order to know the Value type for the Visitor impl.
    struct NonEmptyString<T>(std::marker::PhantomData<Option<T>>);

    impl<'de, T> Visitor<'de> for NonEmptyString<T>
    where
        T: Deserialize<'de> + FromStr,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<T>, E>
        where
            E: de::Error,
        {
            if value.is_empty() {
                return Ok(None);
            } else {
                let res = FromStr::from_str(value);
                match res {
                    Ok(good) => Ok(Some(good)),
                    // TODO: Find a way to propagate the error.
                    Err(_) => Err(de::Error::custom("could not parse string")),
                }
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Option<T>, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(NonEmptyString(std::marker::PhantomData))
}

/// Describes a transaction.
#[derive(Debug, Deserialize)]
pub struct Transaction {
    /// Balance in the account after the transaction.
    pub account_balance: i64,
    /// The amount of the transaction in minor units of currency. For example pennies in the case
    /// of GBP. A negative amount indicates a debit (most card transactions will have a negative
    /// amount).
    pub amount: i64,
    /// The timestamp in when the transaction was created.
    pub created: DateTime<Utc>,
    /// The ISO 4217 currency code.
    pub currency: Currency,
    /// Description of the transaction.
    pub description: String,
    /// The transaction id.
    pub id: TransactionId,
    /// This contains the merchant_id of the merchant that this transaction was made at.
    pub merchant: Option<MerchantId>,
    /// Key-value annotations made for transaction. Metadata is private to your application.
    pub metadata: HashMap<String, String>,
    /// Notes attached to the transaction.
    pub notes: String,
    /// Top-ups to an account are represented as transactions with a positive amount and
    /// is_load = true. Other transactions such as refunds, reversals or chargebacks may have a
    /// positive amount but is_load = false
    pub is_load: bool,
    /// The timestamp in UTC (RFC 3339) at which the transaction settled. In most cases, this
    /// happens 24-48 hours after created. If this field is not present, the transaction is
    /// authorised but not yet “complete”.
    ///
    /// Bug: Even though the Monzo documentation says the field is not present when not authorised,
    /// in practice they send an empty string. See https://github.com/monzo/docs/pull/59.
    #[serde(deserialize_with = "none_for_empty_string")]
    pub settled: Option<DateTime<Utc>>,
    /// The category can be set for each transaction by the user. Over time we learn which merchant
    /// goes in which category and auto-assign the category of a transaction. If the user hasn’t
    /// set a category, we’ll return the default category of the merchant on this transactions.
    /// Top-ups have category mondo. Valid values are general, eating_out, expenses, transport,
    /// cash, bills, entertainment, shopping, holidays, groceries.
    pub category: String,
    /// This is only present on declined transactions! Valid values are INSUFFICIENT_FUNDS,
    /// CARD_INACTIVE, CARD_BLOCKED or OTHER.
    // TODO: Make this an enum?
    pub decline_reason: Option<String>,
}

/// Response to the transactions future if successful.
#[derive(Debug, Deserialize)]
pub struct Transactions {
    /// List of transactions.
    pub transactions: Vec<Transaction>,
}

/// Response to the transaction future if successful.
#[derive(Debug, Deserialize)]
pub struct TransactionResponse {
    /// A single transaction.
    pub transaction: Transaction,
}

/// Response to the futures in case of an error.
#[derive(Debug, Deserialize)]
pub struct Error {
    /// The HTTP response code.
    pub code: Option<String>,
    /// Short description of the error.
    // TODO; Maybe make this an enum since the documentation says it can only contain a certain set
    // of values.
    pub error: Option<String>,
    /// Longer description of the error.
    pub error_description: Option<String>,
    /// Additional information.
    pub message: Option<String>,
}

const ACCOUNT_ID: &'static str = "account_id";

/// Errors for this crate using `error_chain`.
pub mod errors {
    error_chain! {
        errors {
            #[doc = "When the Monzo API returns an error response code with more detailed \
            information."]
            BadResponse(statuscode: ::hyper::StatusCode, error: ::Error)
        }
        foreign_links {
            BadJsonResponse(::serde_json::Error)
            #[doc = "When the Monzo API returns invalid or unexpected json content."];
            NetworkError(::hyper::Error) #[doc = "Returned on network failure."];
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
                let status = res.status();
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
        url.query_pairs_mut().append_pair(ACCOUNT_ID, &account_id);
        let uri: Uri = url.into_string().parse().unwrap();

        self.make_request(uri, |body| {
            let b: Balance = serde_json::from_slice(&body)?;
            Ok(b)
        })
    }

    /// Returns a list of transactions on the user’s account.
    pub fn transactions(
        &self,
        account_id: AccountId,
    ) -> Box<Future<Item = Transactions, Error = errors::Error>> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().unwrap().push("transactions");
        url.query_pairs_mut().append_pair(ACCOUNT_ID, &account_id);
        let uri: Uri = url.into_string().parse().unwrap();

        self.make_request(uri, |body| {
            let t: Transactions = serde_json::from_slice(&body)?;
            Ok(t)
        })
    }

    /// Returns a list of transactions on the user’s account.
    pub fn transaction(
        &self,
        account_id: AccountId,
        transaction_id: TransactionId,
    ) -> Box<Future<Item = TransactionResponse, Error = errors::Error>> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().unwrap().push("transactions");
        url.path_segments_mut().unwrap().push(&transaction_id);
        url.query_pairs_mut().append_pair(ACCOUNT_ID, &account_id);
        let uri: Uri = url.into_string().parse().unwrap();

        self.make_request(uri, |body| {
            let t: TransactionResponse = serde_json::from_slice(&body)?;
            Ok(t)
        })
    }
}
