use poem::{http::StatusCode, test::TestResponse, web::cookie::Cookie};

use dodatok::config::Config;

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

fn check_cookie(cookie: &Cookie, config: &Config) {
    assert_eq!(cookie.http_only(), true);
    assert_eq!(cookie.path().unwrap(), config.cookie.path);
    assert_eq!(cookie.same_site().unwrap(), config.cookie.same_site);
    assert_eq!(cookie.secure(), config.cookie.secure);
}

pub fn check_csrf_cookie(cookie: &Cookie, value: &str, config: &Config) {
    check_cookie(cookie, config);
    assert_eq!(cookie.name(), &config.csrf.cookie);
    assert_eq!(cookie.value_str(), value);
    assert_eq!(
        cookie.max_age().unwrap(),
        config.csrf.cookie_lifetime.to_std().unwrap()
    );
}

pub fn check_response(response: &TestResponse, status: StatusCode) {
    response.assert_content_type("application/json");
    response.assert_status(status);
}

pub fn check_session_cookie(cookie: &Cookie, value: &str, config: &Config) {
    check_cookie(cookie, config);
    assert_eq!(cookie.name(), &config.session.cookie);
    assert_eq!(cookie.value_str(), value);
    assert!(cookie.max_age().is_none());
}

pub fn check_session_cookie_removed(cookie: &Cookie, config: &Config) {
    check_cookie(cookie, config);
    assert_eq!(cookie.name(), &config.session.cookie);
    assert_eq!(cookie.value_str(), "");
    assert!(cookie.max_age().unwrap().is_zero());
}
