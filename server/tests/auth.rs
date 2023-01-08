use chrono::{DateTime, Utc};
use poem::{
    http::{header::{COOKIE, SET_COOKIE}, StatusCode},
    test::{TestClient, TestResponse},
    web::cookie::{Cookie, CookieJar},
};
use serde_json::{json, Value as JsonValue};
use serial_test::serial;

use dodatok::{create_app, db, util::{generate_token, generate_totp, utc_now}, CONFIG};

mod setup;
mod util;

use setup::TestDb;
use util::{
    assert_error,
    assert_error_with_details,
    check_csrf_cookie,
    check_response,
    check_session_cookie,
    check_session_cookie_removed,
};

#[tokio::test]
#[serial]
async fn csrf_no_cookie_no_session() {
    let _db = TestDb::new("csrf_no_cookie_no_session").await;
    let cli = TestClient::new(create_app());
    let res = cli.post("/auth/login").send().await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body contains a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token);
}

#[tokio::test]
#[serial]
async fn csrf_no_cookie_existing_session() {
    let _db = TestDb::new("csrf_no_cookie_existing_session").await;
    let user = setup::add_user('a', false).await;
    let (session_id, session_csrf_token) = setup::add_session(&user, false).await;
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);

    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .header(COOKIE, session_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body contains a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body and in the session
    assert_eq!(csrf_token, session_csrf_token);
    check_csrf_cookie(&cookie, csrf_token);
}

#[tokio::test]
#[serial]
async fn csrf_no_cookie_expired_session() {
    let _db = TestDb::new("csrf_no_cookie_expired_session").await;
    let user = setup::add_user('a', false).await;
    let (session_id, session_csrf_token) = setup::add_session(&user, true).await;
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);

    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .header(COOKIE, session_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie sets CSRF token and clears session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 2);
    let cookie_jar = CookieJar::default();
    for cookie in cookies {
        cookie_jar.add(Cookie::parse(cookie.to_str().unwrap()).unwrap());
    }
    check_session_cookie_removed(&cookie_jar.get(&CONFIG.session.cookie).unwrap());

    // Check that the response body contains a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);
    assert_ne!(csrf_token, session_csrf_token);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie_jar.get(&CONFIG.csrf.cookie).unwrap(), csrf_token);
}

#[tokio::test]
#[serial]
async fn csrf_no_cookie_invalid_session() {
    let _db = TestDb::new("csrf_no_cookie_invalid_session").await;
    let session_id = generate_token(CONFIG.session.id_length);
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);

    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .header(COOKIE, session_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie sets CSRF token and clears session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 2);
    let cookie_jar = CookieJar::default();
    for cookie in cookies {
        cookie_jar.add(Cookie::parse(cookie.to_str().unwrap()).unwrap());
    }
    check_session_cookie_removed(&cookie_jar.get(&CONFIG.session.cookie).unwrap());

    // Check that the response body contains a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie_jar.get(&CONFIG.csrf.cookie).unwrap(), csrf_token);
}

#[tokio::test]
#[serial]
async fn csrf_no_header() {
    let _db = TestDb::new("csrf_no_header").await;
    let cli = TestClient::new(create_app());
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, "a");
    let res = cli
        .post("/auth/login")
        .header(COOKIE, csrf_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body contains a CSRF token and a csrf:missing-header error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-header");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token);
}

#[tokio::test]
#[serial]
async fn csrf_mismatch() {
    let _db = TestDb::new("csrf_mismatch").await;
    let csrf_cookie = Cookie::new_with_str(
        &CONFIG.csrf.cookie, &generate_token(CONFIG.csrf.token_length)
    );
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &generate_token(CONFIG.csrf.token_length))
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body contains a CSRF token and a csrf:mismatch error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "mismatch");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token);
}

#[tokio::test]
#[serial]
async fn get_csrf_token_no_session() {
    let _db = TestDb::new("get_csrf_token_no_session").await;
    let cli = TestClient::new(create_app());
    let res = cli.get("/auth/csrf-token").send().await;
    check_response(&res, StatusCode::OK);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body only contains CSRF token
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token);
}

#[tokio::test]
#[serial]
async fn get_csrf_token_invalid_session() {
    let _db = TestDb::new("get_csrf_token_invalid_session").await;
    let session_id = generate_token(CONFIG.session.id_length);
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, session_id);

    let cli = TestClient::new(create_app());
    let res = cli
        .get("/auth/csrf-token")
        .header(COOKIE, session_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::OK);

    // Check that Set-Cookie sets CSRF token and clears session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 2);
    let cookie_jar = CookieJar::default();
    for cookie in cookies {
        cookie_jar.add(Cookie::parse(cookie.to_str().unwrap()).unwrap());
    }
    check_session_cookie_removed(&cookie_jar.get(&CONFIG.session.cookie).unwrap());

    // Check that the response body only contains CSRF token
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie_jar.get(&CONFIG.csrf.cookie).unwrap(), csrf_token);
}

#[tokio::test]
#[serial]
async fn get_csrf_token_expired_session() {
    let _db = TestDb::new("get_csrf_token_expired_session").await;
    let user = setup::add_user('a', false).await;
    let (session_id, _) = setup::add_session(&user, true).await;
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, session_id);

    let cli = TestClient::new(create_app());
    let res = cli
        .get("/auth/csrf-token")
        .header(COOKIE, session_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::OK);

    // Check that Set-Cookie sets CSRF token and clears session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 2);
    let cookie_jar = CookieJar::default();
    for cookie in cookies {
        cookie_jar.add(Cookie::parse(cookie.to_str().unwrap()).unwrap());
    }
    check_session_cookie_removed(&cookie_jar.get(&CONFIG.session.cookie).unwrap());

    // Check that the response body only contains CSRF token
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie_jar.get(&CONFIG.csrf.cookie).unwrap(), csrf_token);
}

#[tokio::test]
#[serial]
async fn get_csrf_token_existing_session() {
    let _db = TestDb::new("get_csrf_token_existing_session").await;
    let user = setup::add_user('a', false).await;
    let (session_id, session_csrf_token) = setup::add_session(&user, false).await;
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);

    let cli = TestClient::new(create_app());
    let res = cli
        .get("/auth/csrf-token")
        .header(COOKIE, session_cookie.to_string())
        .send()
        .await;
    check_response(&res, StatusCode::OK);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body only has a CSRF token
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body and in the session
    assert_eq!(csrf_token, session_csrf_token);
    check_csrf_cookie(&cookie, csrf_token);
}

#[tokio::test]
#[serial]
async fn login_csrf() {
    let _db = TestDb::new("login_csrf").await;
    let cli = TestClient::new(create_app());
    let res = cli.post("/auth/login").send().await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only sets CSRF token
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();

    // Check that the response body has a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&CONFIG.csrf.response_field).string();
    assert_eq!(csrf_token.len(), CONFIG.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");
}

#[tokio::test]
#[serial]
async fn login_invalid_data() {
    async fn check_invalid_data(res: TestResponse) {
        check_response(&res, StatusCode::BAD_REQUEST);
        res.assert_header_is_not_exist(SET_COOKIE);
        assert_error_with_details(res, "general", "invalid-data").await;
    }

    async fn post_invalid_json(data: &JsonValue, csrf_token: &str, session_id: &str) {
        let cli = TestClient::new(create_app());
        let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, csrf_token);
        let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, session_id);
        let cookies = [csrf_cookie, session_cookie].map(|cookie| cookie.to_string()).join(";");
        let res = cli
            .post("/auth/login")
            .header(COOKIE, cookies)
            .header(&CONFIG.csrf.header, csrf_token)
            .body_json(data)
            .send()
            .await;
        check_invalid_data(res).await;
    }

    let _db = TestDb::new("login_invalid_data").await;
    let user = setup::add_user('a', false).await;
    let (session_id, csrf_token) = setup::add_session(&user, false).await;
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie].map(|cookie| cookie.to_string()).join(";");
    let cli = TestClient::new(create_app());

    let res = cli
        .post("/auth/login")
        .header(COOKIE, &cookies)
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_invalid_data(res).await;

    let res = cli
        .post("/auth/login")
        .body("non-JSON body")
        .header(COOKIE, &cookies)
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_invalid_data(res).await;

    post_invalid_json(&json!(null), &csrf_token, &session_id).await;
    post_invalid_json(&json!([]), &csrf_token, &session_id).await;
    post_invalid_json(&json!(123), &csrf_token, &session_id).await;
    post_invalid_json(&json!("string"), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "password": "a"}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "remember": true}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"password": "a", "remember": true}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "password": "a", "remember": true, "x": "y"}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": 1, "password": "a", "remember": true}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": ["a"], "password": "a", "remember": true}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": {}, "password": "a", "remember": true}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": null, "password": "a", "remember": true}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "password": "a", "remember": null}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "password": "a", "remember": "true"}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "password": "a", "remember": true, "totp": 123456}), &csrf_token, &session_id).await;
    post_invalid_json(&json!({"username": "a", "password": "a", "remember": true, "totp": null}), &csrf_token, &session_id).await;
}

#[tokio::test]
#[serial]
async fn login_existing_session() {
    let _db = TestDb::new("login_existing_session").await;
    let user = setup::add_user('a', false).await;
    let (session_id, csrf_token) = setup::add_session(&user, false).await;
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie].map(|cookie| cookie.to_string()).join(";");
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({ "username": user.username, "password": "", "remember": false }))
        .header(COOKIE, cookies)
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "already-logged-in").await;
}

#[tokio::test]
#[serial]
async fn login_no_session_invalid_credentials() {
    let _db = TestDb::new("login_no_session_invalid_credentials").await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({ "username": "", "password": "", "remember": false }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "invalid-credentials").await;
}

#[tokio::test]
#[serial]
async fn login_expired_session_invalid_credentials() {
    let _db = TestDb::new("login_expired_session_invalid_credentials").await;
    let user = setup::add_user('a', false).await;
    let (session_id, csrf_token) = setup::add_session(&user, true).await;
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie].map(|cookie| cookie.to_string()).join(";");
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({ "username": user.username, "password": "", "remember": false }))
        .header(COOKIE, cookies)
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only removes session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
    check_session_cookie_removed(&cookie);

    assert_error(res, "auth", "invalid-credentials").await;
}

#[tokio::test]
#[serial]
async fn login_invalid_session_invalid_credentials() {
    let _db = TestDb::new("login_invalid_session_invalid_credentials").await;
    let session_id = generate_token(CONFIG.session.id_length);
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&CONFIG.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie].map(|cookie| cookie.to_string()).join(";");
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({ "username": "", "password": "", "remember": false }))
        .header(COOKIE, cookies)
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only removes session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
    check_session_cookie_removed(&cookie);

    assert_error(res, "auth", "invalid-credentials").await;
}

#[tokio::test]
#[serial]
async fn login_missing_totp() {
    let _db = TestDb::new("login_missing_totp").await;
    let user = setup::add_user('a', true).await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "missing-totp").await;
}

#[tokio::test]
#[serial]
async fn login_invalid_totp() {
    let _db = TestDb::new("login_invalid_totp").await;
    let user = setup::add_user('a', true).await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": "should be only digits",
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "invalid-totp").await;
}

#[tokio::test]
#[serial]
async fn login_totp_reuse() {
    let _db = TestDb::new("login_totp_reuse").await;
    let user = setup::add_user('a', true).await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let now = utc_now().timestamp() as u64;
    let totp = generate_totp(&user.totp_key.unwrap(), now);

    let cli = TestClient::new(create_app());
    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": totp,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::OK);

    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": totp,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);
    assert_error(res, "auth", "totp-reuse").await;
}

#[tokio::test]
#[serial]
async fn login_disabled() {
    let db = TestDb::new("login_disabled").await.get().await;
    let user = setup::add_user('a', false).await;
    db.execute(r#"UPDATE "users" SET "active" = false WHERE "id" = $1"#, &[&user.id]).await.unwrap();
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);

    let cli = TestClient::new(create_app());

    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);
    assert_error(res, "auth", "account-disabled").await;
}

#[tokio::test]
#[serial]
async fn login_success() {
    let db = TestDb::new("login_success").await.get().await;
    let user = setup::add_user('a', false).await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);

    let cli = TestClient::new(create_app());

    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    let row = db.query_one(r#"SELECT "csrf_token", "sudo_until" FROM "sessions""#, &[]).await.unwrap();

    check_response(&res, StatusCode::OK);
    res.assert_json(json!({
        "csrfToken": row.get::<_, &str>("csrf_token"),
        "user": {
            "id": user.id,
            "username": user.username,
            "totpEnabled": false,
            "passwordChangeReason": null,
            "icon": null,
            "locale": user.locale,
            "sudoUntil": row.get::<_, DateTime<Utc>>("sudo_until").to_rfc3339(),
        },
    })).await;
}

#[tokio::test]
#[serial]
async fn login_success_unused_totp() {
    let db = TestDb::new("login_success_unused_totp").await.get().await;
    let user = setup::add_user('a', false).await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);

    let cli = TestClient::new(create_app());

    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": "123456",
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    let row = db.query_one(r#"SELECT "csrf_token", "sudo_until" FROM "sessions""#, &[]).await.unwrap();

    check_response(&res, StatusCode::OK);
    res.assert_json(json!({
        "csrfToken": row.get::<_, &str>("csrf_token"),
        "user": {
            "id": user.id,
            "username": user.username,
            "totpEnabled": false,
            "passwordChangeReason": null,
            "icon": null,
            "locale": user.locale,
            "sudoUntil": row.get::<_, DateTime<Utc>>("sudo_until").to_rfc3339(),
        },
        "warnings": [{"source": "auth", "id": "unused-totp"}],
    })).await;
}

#[tokio::test]
#[serial]
async fn login_success_with_totp() {
    let db = TestDb::new("login_success_with_totp").await.get().await;
    let user = setup::add_user('a', true).await;
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&CONFIG.csrf.cookie, &csrf_token);
    let now = utc_now().timestamp() as u64;
    let totp = generate_totp(&user.totp_key.unwrap(), now);

    let cli = TestClient::new(create_app());

    let res = cli
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": totp,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&CONFIG.csrf.header, &csrf_token)
        .send()
        .await;
    let row = db.query_one(r#"SELECT "csrf_token", "sudo_until" FROM "sessions""#, &[]).await.unwrap();

    check_response(&res, StatusCode::OK);
    res.assert_json(json!({
        "csrfToken": row.get::<_, &str>("csrf_token"),
        "user": {
            "id": user.id,
            "username": user.username,
            "totpEnabled": true,
            "passwordChangeReason": null,
            "icon": null,
            "locale": user.locale,
            "sudoUntil": row.get::<_, DateTime<Utc>>("sudo_until").to_rfc3339(),
        },
    })).await;
}
