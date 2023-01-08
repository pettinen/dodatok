use poem::{http::StatusCode, test::TestResponse, web::cookie::Cookie};

use dodatok::CONFIG;

pub async fn assert_error(res: TestResponse, source: &str, id: &str) {
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), source);
    assert_eq!(errors[0].get("id").string(), id);
}

pub async fn assert_error_with_details(res: TestResponse, source: &str, id: &str) {
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(3);
    assert_eq!(errors[0].get("source").string(), source);
    assert_eq!(errors[0].get("id").string(), id);
    errors[0].get("details").string();
}

fn check_cookie(cookie: &Cookie) {
    assert_eq!(cookie.http_only(), true);
    assert_eq!(cookie.path().unwrap(), CONFIG.cookie.path);
    assert_eq!(cookie.same_site().unwrap(), CONFIG.cookie.same_site);
    assert_eq!(cookie.secure(), CONFIG.cookie.secure);
}

pub fn check_csrf_cookie(cookie: &Cookie, value: &str) {
    check_cookie(cookie);
    assert_eq!(cookie.name(), &CONFIG.csrf.cookie);
    assert_eq!(cookie.value_str(), value);
    assert_eq!(cookie.max_age().unwrap(), CONFIG.csrf.cookie_lifetime.to_std().unwrap());
}

pub fn check_response(response: &TestResponse, status: StatusCode) {
    response.assert_content_type("application/json");
    response.assert_status(status);
}

pub fn check_session_cookie(cookie: &Cookie, value: &str) {
    check_cookie(cookie);
    assert_eq!(cookie.name(), &CONFIG.session.cookie);
    assert_eq!(cookie.value_str(), value);
    assert!(cookie.max_age().is_none());
}

pub fn check_session_cookie_removed(cookie: &Cookie) {
    check_cookie(cookie);
    assert_eq!(cookie.name(), &CONFIG.session.cookie);
    assert_eq!(cookie.value_str(), "");
    assert!(cookie.max_age().unwrap().is_zero());
}
