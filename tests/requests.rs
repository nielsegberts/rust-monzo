extern crate monzo;
extern crate tokio_core;
extern crate mockito;
extern crate url;
extern crate spectral;
extern crate hyper;

use mockito::mock;
use monzo::{Client, Balance};
use spectral::prelude::*;
use tokio_core::reactor::Core;
use url::Url;

fn create_monzo(core: &Core) -> monzo::Client {
    Client::new_with_base_url(&core.handle(), Url::parse(mockito::SERVER_URL).unwrap())
}

#[test]
fn balance() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/balance\?.*$".to_string()),
    ).with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
            \"balance\": 1234,
            \"currency\": \"GBP\",
            \"spend_today\": 567
        }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.balance("some_id".into(), "some_token".into());
    let b: Balance = core.run(work).unwrap();
    assert_that(&b.balance).is_equal_to(1234);
    assert_that(&b.currency).is_equal_to(String::from("GBP"));
    assert_that(&b.spend_today).is_equal_to(567);
}

#[test]
fn unauthorized() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/balance\?.*$".to_string()),
    ).with_status(401)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
            \"code\": \"unauthorized.bad_access_token\",
            \"error\": \"invalid_token\",
            \"error_description\": \"expired1\",
            \"message\": \"expired2\"
        }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.balance("some_id".into(), "some_token".into());
    let response_error = core.run(work).unwrap_err();

    match response_error {
        monzo::errors::Error(monzo::errors::ErrorKind::BadResponse(statuscode, e), _) => {
            assert_that(&statuscode).is_equal_to(hyper::StatusCode::Unauthorized);
            assert_that(&e.code).is_some().is_equal_to(
                "unauthorized.bad_access_token"
                    .to_string(),
            );
            assert_that(&e.error).is_some().is_equal_to(
                String::from("invalid_token"),
            );
            assert_that(&e.error_description).is_some().is_equal_to(
                String::from(
                    "expired1",
                ),
            );
            assert_that(&e.message).is_some().is_equal_to(
                String::from("expired2"),
            );
        }
        _ => panic!("Incorrect error type"),
    }
}

#[test]
fn bad_json() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/balance\?.*$".to_string()),
    ).with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body("{ badjson ")
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.balance("some_id".into(), "some_token".into());
    let response_error = core.run(work).unwrap_err();

    match response_error {
        monzo::errors::Error(monzo::errors::ErrorKind::BadJsonResponse(_), _) => {}
        _ => panic!("Incorrect error type"),
    }
}
