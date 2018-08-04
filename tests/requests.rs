extern crate hyper;
extern crate mockito;
extern crate monzo;
extern crate spectral;
extern crate tokio_core;
extern crate url;

use mockito::mock;
use monzo::{Accounts, Balance, Client, PotsResponse, TransactionResponse, Transactions};
use spectral::prelude::*;
use tokio_core::reactor::Core;
use url::Url;

fn create_monzo() -> monzo::Client {
    Client::new_with_base_url("token", Url::parse(mockito::SERVER_URL).unwrap())
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
    let monzo = create_monzo();
    let work = monzo.accounts();
    let a: Accounts = core.run(work).unwrap();
    assert_that(&a.accounts.len()).is_equal_to(1);
    assert_that(&a.accounts[0].id.as_str()).is_equal_to("acc_00009237aqC8c5umZmrRdh");
    assert_that(&a.accounts[0].description.as_str()).is_equal_to("Peter Pan's Account");
    assert_that(&a.accounts[0].created.to_rfc3339())
        .is_equal_to("2015-11-13T12:17:42+00:00".to_string());
}

#[test]
fn balance() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/balance\?account_id=some_id$".to_string()),
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
    let monzo = create_monzo();
    let work = monzo.balance("some_id".into());
    let b: Balance = core.run(work).unwrap();
    assert_that(&b.balance).is_equal_to(5000);
    assert_that(&b.currency.as_str()).is_equal_to("GBP");
    assert_that(&b.spend_today).is_equal_to(100);
}

#[test]
fn transactions() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/transactions\?account_id=some_id$".to_string()),
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
    let monzo = create_monzo();
    let work = monzo.transactions("some_id".into());
    let ts: Transactions = core.run(work).unwrap();
    assert_that(&ts.transactions.len()).is_equal_to(1);
    let t = &ts.transactions[0];
    assert_that(&t.account_balance).is_equal_to(13013);
    assert_that(&t.amount).is_equal_to(-510);
    assert_that(&t.created.to_rfc3339().as_str()).is_equal_to("2015-08-22T12:20:18+00:00");
    assert_that(&t.currency.as_str()).is_equal_to("GBP");
    assert_that(&t.description.as_str()).is_equal_to("THE DE BEAUVOIR DELI C LONDON GBR");
    assert_that(&t.merchant)
        .is_some()
        .is_equal_to("merch_00008zIcpbAKe8shBxXUtl".to_string());
    assert_that(&t.id.as_str()).is_equal_to("tx_00008zIcpb1TB4yeIFXMzx");
    assert_that(&t.metadata.len()).is_equal_to(1);
    assert_that(&t.notes.as_str()).is_equal_to("Salmon sandwich 🍞");
    assert_that(&t.is_load).is_equal_to(false);
    assert_that(&t.settled.unwrap().to_rfc3339())
        .is_equal_to("2015-08-23T12:20:18+00:00".to_string());
    assert_that(&t.category.as_str()).is_equal_to("eating_out");
    assert_that(&t.decline_reason).is_none();
}

#[test]
fn transactions_declined_no_merchant_no_settled() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/transactions\?account_id=some_id$".to_string()),
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
                        \"settled\": \"\",
                        \"category\": \"eating_out\",
                        \"decline_reason\": \"CARD_INACTIVE\"
                    }
                ]
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo();
    let work = monzo.transactions("some_id".into());
    let t = &core.run(work).unwrap().transactions[0];
    assert_that(&t.decline_reason)
        .is_some()
        .is_equal_to("CARD_INACTIVE".to_string());
    assert_that(&t.merchant).is_none();
    assert_that(&t.settled).is_none();
}

#[test]
fn transaction() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/transactions/some_t_id\?account_id=some_id$".to_string()),
    ).with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
                \"transaction\": {
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
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo();
    let work = monzo.transaction("some_id".into(), "some_t_id".into());
    let ts: TransactionResponse = core.run(work).unwrap();
    let t = &ts.transaction;
    assert_that(&t.account_balance).is_equal_to(13013);
    // No point in testing Transaction deserialization further.
}

#[test]
fn pots() {
    let _m = mock("GET", mockito::Matcher::Regex(r"^/pots/listV1".to_string()))
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            "{
                \"pots\": [
                    {
                        \"id\": \"pot_0000778xxfgh4iu8z83nWb\",
                        \"name\": \"Savings\",
                        \"style\": \"beach_ball\",
                        \"balance\": 133700,
                        \"currency\": \"GBP\",
                        \"created\": \"2017-11-09T12:30:53.695Z\",
                        \"updated\": \"2017-11-09T13:30:53.695Z\",
                        \"deleted\": false
                    }
                ]
            }",
        )
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo();
    let work = monzo.pots();
    let pots: PotsResponse = core.run(work).unwrap();
    let pot = &pots.pots[0];
    assert_that(&pot.id.as_str()).is_equal_to("pot_0000778xxfgh4iu8z83nWb");
    assert_that(&pot.name.as_str()).is_equal_to("Savings");
    assert_that(&pot.style.as_str()).is_equal_to("beach_ball");
    assert_that(&pot.balance).is_equal_to(133700);
    assert_that(&pot.currency.as_str()).is_equal_to("GBP");
    assert_that(&pot.created.to_rfc3339()).is_equal_to("2017-11-09T12:30:53.695+00:00".to_string());
    assert_that(&pot.updated.to_rfc3339()).is_equal_to("2017-11-09T13:30:53.695+00:00".to_string());
    assert_that(&pot.deleted).is_equal_to(false);
}

#[test]
fn unauthorized() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/balance\?account_id=some_id$".to_string()),
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
    let monzo = create_monzo();
    let work = monzo.balance("some_id".into());
    let response_error = core.run(work).unwrap_err();

    match response_error {
        monzo::errors::Error(monzo::errors::ErrorKind::BadResponse(statuscode, e), _) => {
            assert_that(&statuscode).is_equal_to(hyper::StatusCode::UNAUTHORIZED);
            assert_that(&e.code)
                .is_some()
                .is_equal_to("unauthorized.bad_access_token".to_string());
            assert_that(&e.error)
                .is_some()
                .is_equal_to("invalid_token".to_string());
            assert_that(&e.error_description)
                .is_some()
                .is_equal_to("expired1".to_string());
            assert_that(&e.message)
                .is_some()
                .is_equal_to("expired2".to_string());
        }
        _ => panic!("Incorrect error type"),
    }
}

#[test]
fn bad_json() {
    let _m = mock(
        "GET",
        mockito::Matcher::Regex(r"^/balance\?account_id=some_id$".to_string()),
    ).with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body("{ badjson ")
        .create();
    let mut core = Core::new().unwrap();
    let monzo = create_monzo();
    let work = monzo.balance("some_id".into());
    let response_error = core.run(work).unwrap_err();

    match response_error {
        monzo::errors::Error(monzo::errors::ErrorKind::BadJsonResponse(_), _) => {}
        _ => panic!("Incorrect error type"),
    }
}
