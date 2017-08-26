extern crate monzo;
extern crate tokio_core;
extern crate mockito;
extern crate url;
extern crate spectral;
extern crate hyper;

use mockito::mock;
use monzo::{Accounts, Client, Balance, Transactions};
use spectral::prelude::*;
use tokio_core::reactor::Core;
use url::Url;

fn create_monzo(core: &Core) -> monzo::Client {
    Client::new_with_base_url(
        &core.handle(),
        "token",
        Url::parse(mockito::SERVER_URL).unwrap(),
    )
}

#[test]
fn accounts() {
    let _m = mock("GET", mockito::Matcher::Regex(r"^/accounts$".to_string()))
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
                \"accounts\": [
                    {
                        \"id\": \"acc_00009237aqC8c5umZmrRdh\",
                        \"description\": \"Peter Pan's Account\",
                        \"created\": \"2015-11-13T12:17:42Z\"
                    }
                ]
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.accounts();
    let a: Accounts = core.run(work).unwrap();
    assert_that(&a.accounts.len()).is_equal_to(1);
    assert_that(&a.accounts[0].id).is_equal_to(String::from("acc_00009237aqC8c5umZmrRdh"));
    assert_that(&a.accounts[0].description).is_equal_to(String::from("Peter Pan's Account"));
    assert_that(&a.accounts[0].created).is_equal_to(String::from("2015-11-13T12:17:42Z"));
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
                \"balance\": 5000,
                \"currency\": \"GBP\",
                \"spend_today\": 100
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.balance("some_id".into());
    let b: Balance = core.run(work).unwrap();
    assert_that(&b.balance).is_equal_to(5000);
    assert_that(&b.currency).is_equal_to(String::from("GBP"));
    assert_that(&b.spend_today).is_equal_to(100);
}

#[test]
fn transactions() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/transactions\?.*$".to_string()),
    ).with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
                \"transactions\": [
                    {
                        \"account_balance\": 13013,
                        \"amount\": -510,
                        \"created\": \"2015-08-22T12:20:18Z\",
                        \"currency\": \"GBP\",
                        \"description\": \"THE DE BEAUVOIR DELI C LONDON GBR\",
                        \"merchant\": \"merch_00008zIcpbAKe8shBxXUtl\",
                        \"id\": \"tx_00008zIcpb1TB4yeIFXMzx\",
                        \"metadata\": {
                            \"seen\": \"2015-09-15T10:19:17Z\"
                        },
                        \"notes\": \"Salmon sandwich 🍞\",
                        \"is_load\": false,
                        \"settled\": \"2015-08-23T12:20:18Z\",
                        \"category\": \"eating_out\"
                    }
                ]
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.transactions("some_id".into());
    let ts: Transactions = core.run(work).unwrap();
    assert_that(&ts.transactions.len()).is_equal_to(1);
    let t = &ts.transactions[0];
    assert_that(&t.account_balance).is_equal_to(13013);
    assert_that(&t.amount).is_equal_to(-510);
    assert_that(&t.created).is_equal_to(String::from("2015-08-22T12:20:18Z"));
    assert_that(&t.currency).is_equal_to(String::from("GBP"));
    assert_that(&t.description).is_equal_to(String::from("THE DE BEAUVOIR DELI C LONDON GBR"));
    assert_that(&t.merchant).is_some().is_equal_to(
        String::from(
            "merch_00008zIcpbAKe8shBxXUtl",
        ),
    );
    assert_that(&t.id).is_equal_to(String::from("tx_00008zIcpb1TB4yeIFXMzx"));
    assert_that(&t.metadata.len()).is_equal_to(1);
    assert_that(&t.notes).is_equal_to(String::from("Salmon sandwich 🍞"));
    assert_that(&t.is_load).is_equal_to(false);
    assert_that(&t.settled).is_equal_to(String::from("2015-08-23T12:20:18Z"));
    assert_that(&t.category).is_equal_to(String::from("eating_out"));
    assert_that(&t.decline_reason).is_none();
}

#[test]
fn transactions_declined() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/transactions\?.*$".to_string()),
    ).with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
                \"transactions\": [
                    {
                        \"account_balance\": 13013,
                        \"amount\": -510,
                        \"created\": \"2015-08-22T12:20:18Z\",
                        \"currency\": \"GBP\",
                        \"description\": \"THE DE BEAUVOIR DELI C LONDON GBR\",
                        \"merchant\": null,
                        \"id\": \"tx_00008zIcpb1TB4yeIFXMzx\",
                        \"metadata\": {},
                        \"notes\": \"Salmon sandwich 🍞\",
                        \"is_load\": false,
                        \"settled\": \"2015-08-23T12:20:18Z\",
                        \"category\": \"eating_out\",
                        \"decline_reason\": \"CARD_INACTIVE\"
                    }
                ]
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo(&core);
    let work = monzo.transactions("some_id".into());
    let t = &core.run(work).unwrap().transactions[0];
    assert_that(&t.decline_reason).is_some().is_equal_to(
        String::from(
            "CARD_INACTIVE",
        ),
    );
    assert_that(&t.merchant).is_none();
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
    let work = monzo.balance("some_id".into());
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
    let work = monzo.balance("some_id".into());
    let response_error = core.run(work).unwrap_err();

    match response_error {
        monzo::errors::Error(monzo::errors::ErrorKind::BadJsonResponse(_), _) => {}
        _ => panic!("Incorrect error type"),
    }
}
