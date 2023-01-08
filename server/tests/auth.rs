use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::{tokio_postgres::NoTls, ClientWrapper};
use poem::{
    http::{
        header::{CONTENT_TYPE, COOKIE, SET_COOKIE},
        StatusCode,
    },
    test::{TestClient, TestResponse},
    web::cookie::{Cookie, CookieJar},
    Endpoint, Response,
};
use serde_json::{json, Value as JsonValue};
use test_context::{test_context, AsyncTestContext};

use dodatok::{
    config::Config,
    util::{generate_token, generate_totp, utc_now},
};
use macros::test_with_client;

mod setup;
mod util;

use util::{
    assert_error, assert_error_with_details, check_csrf_cookie, check_response,
    check_session_cookie_removed,
};

#[test_with_client]
async fn csrf_no_cookie_no_session() {
    let client = TestClient::new(&ctx.endpoint);
    let res = client.post("/auth/login").send().await;
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);
}

#[test_with_client]
async fn csrf_no_cookie_existing_session() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, session_csrf_token) = setup::add_session(&user, false, &ctx.config).await;
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);

    let res = client
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body and in the session
    assert_eq!(csrf_token, session_csrf_token);
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);
}

#[test_with_client]
async fn csrf_no_cookie_expired_session() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, session_csrf_token) = setup::add_session(&user, true, &ctx.config).await;
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);

    let res = client
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
    check_session_cookie_removed(
        &cookie_jar.get(&ctx.config.session.cookie).unwrap(),
        &ctx.config,
    );

    // Check that the response body contains a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);
    assert_ne!(csrf_token, session_csrf_token);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(
        &cookie_jar.get(&ctx.config.csrf.cookie).unwrap(),
        csrf_token,
        &ctx.config,
    );
}

#[test_with_client]
async fn csrf_no_cookie_invalid_session() {
    let client = TestClient::new(&ctx.endpoint);
    let session_id = generate_token(ctx.config.session.id_length);
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);

    let res = client
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
    check_session_cookie_removed(
        &cookie_jar.get(&ctx.config.session.cookie).unwrap(),
        &ctx.config,
    );

    // Check that the response body contains a CSRF token and a csrf:missing-cookie error
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(2);
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(
        &cookie_jar.get(&ctx.config.csrf.cookie).unwrap(),
        csrf_token,
        &ctx.config,
    );
}

#[test_with_client]
async fn csrf_no_header() {
    let client = TestClient::new(&ctx.endpoint);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, "a");
    let res = client
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-header");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);
}

#[test_with_client]
async fn csrf_mismatch() {
    let client = TestClient::new(&ctx.endpoint);
    let csrf_cookie = Cookie::new_with_str(
        &ctx.config.csrf.cookie,
        &generate_token(ctx.config.csrf.token_length),
    );
    let res = client
        .post("/auth/login")
        .header(COOKIE, csrf_cookie.to_string())
        .header(
            &ctx.config.csrf.header,
            &generate_token(ctx.config.csrf.token_length),
        )
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "mismatch");

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);
}

#[test_with_client]
async fn get_csrf_token_no_session() {
    let client = TestClient::new(&ctx.endpoint);
    let res = client.get("/auth/csrf-token").send().await;
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);
}

#[test_with_client]
async fn get_csrf_token_invalid_session() {
    let client = TestClient::new(&ctx.endpoint);
    let session_id = generate_token(ctx.config.session.id_length);
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, session_id);

    let res = client
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
    check_session_cookie_removed(
        &cookie_jar.get(&ctx.config.session.cookie).unwrap(),
        &ctx.config,
    );

    // Check that the response body only contains CSRF token
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(
        &cookie_jar.get(&ctx.config.csrf.cookie).unwrap(),
        csrf_token,
        &ctx.config,
    );
}

#[test_with_client]
async fn get_csrf_token_expired_session() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, _) = setup::add_session(&user, true, &ctx.config).await;
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, session_id);

    let res = client
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
    check_session_cookie_removed(
        &cookie_jar.get(&ctx.config.session.cookie).unwrap(),
        &ctx.config,
    );

    // Check that the response body only contains CSRF token
    let json = res.json().await;
    let json = json.value().object();
    json.assert_len(1);
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(
        &cookie_jar.get(&ctx.config.csrf.cookie).unwrap(),
        csrf_token,
        &ctx.config,
    );
}

#[test_with_client]
async fn get_csrf_token_existing_session() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, session_csrf_token) = setup::add_session(&user, false, &ctx.config).await;
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);

    let res = client
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body and in the session
    assert_eq!(csrf_token, session_csrf_token);
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);
}

#[test_with_client]
async fn login_csrf() {
    let client = TestClient::new(&ctx.endpoint);
    let res = client.post("/auth/login").send().await;
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
    let csrf_token = json.get(&ctx.config.csrf.response_field).string();
    assert_eq!(csrf_token.len(), ctx.config.csrf.token_length as usize);

    // Check that the CSRF cookie matches the token in the body
    check_csrf_cookie(&cookie, csrf_token, &ctx.config);

    let errors = json.get("errors").object_array();
    assert_eq!(errors.len(), 1);
    errors[0].assert_len(2);
    assert_eq!(errors[0].get("source").string(), "csrf");
    assert_eq!(errors[0].get("id").string(), "missing-cookie");
}

#[test_with_client]
async fn login_invalid_data() {
    async fn check_invalid_data(res: TestResponse) {
        check_response(&res, StatusCode::BAD_REQUEST);
        res.assert_header_is_not_exist(SET_COOKIE);
        assert_error_with_details(res, "general", "invalid-data").await;
    }

    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, csrf_token) = setup::add_session(&user, false, &ctx.config).await;
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie]
        .map(|cookie| cookie.to_string())
        .join(";");

    let res = client
        .post("/auth/login")
        .body("null")
        .header(COOKIE, &cookies)
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error_with_details(res, "general", "unsupported-media-type").await;

    let res = client
        .post("/auth/login")
        .header(CONTENT_TYPE, "application/json")
        .header(COOKIE, &cookies)
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_invalid_data(res).await;

    let res = client
        .post("/auth/login")
        .body("non-JSON body")
        .header(CONTENT_TYPE, "application/json")
        .header(COOKIE, &cookies)
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_invalid_data(res).await;

    async fn post_invalid_json<E: Endpoint>(
        client: &TestClient<E>,
        config: &Config,
        data: &JsonValue,
        csrf_token: &str,
        session_id: &str,
    ) {
        let csrf_cookie = Cookie::new_with_str(&config.csrf.cookie, csrf_token);
        let session_cookie = Cookie::new_with_str(&config.session.cookie, session_id);
        let cookies = [csrf_cookie, session_cookie]
            .map(|cookie| cookie.to_string())
            .join(";");
        let res = client
            .post("/auth/login")
            .header(COOKIE, cookies)
            .header(&config.csrf.header, csrf_token)
            .body_json(data)
            .send()
            .await;
        check_invalid_data(res).await;
    }

    post_invalid_json(&client, &ctx.config, &json!(null), &csrf_token, &session_id).await;
    post_invalid_json(&client, &ctx.config, &json!([]), &csrf_token, &session_id).await;
    post_invalid_json(&client, &ctx.config, &json!(123), &csrf_token, &session_id).await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!("string"),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "password": "a" }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "remember": true }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "password": "a", "remember": true }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "password": "a", "remember": true, "x": "y" }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": 1, "password": "a", "remember": true }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": ["a"], "password": "a", "remember": true }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": {}, "password": "a", "remember": true }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": null, "password": "a", "remember": true }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "password": "a", "remember": null }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "password": "a", "remember": "true" }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "password": "a", "remember": true, "totp": 123456 }),
        &csrf_token,
        &session_id,
    )
    .await;
    post_invalid_json(
        &client,
        &ctx.config,
        &json!({ "username": "a", "password": "a", "remember": true, "totp": null }),
        &csrf_token,
        &session_id,
    )
    .await;
}

#[test_with_client]
async fn login_existing_session() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, csrf_token) = setup::add_session(&user, false, &ctx.config).await;
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie]
        .map(|cookie| cookie.to_string())
        .join(";");
    let res = client
        .post("/auth/login")
        .body_json(&json!({ "username": user.username, "password": "", "remember": false }))
        .header(COOKIE, cookies)
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "already-logged-in").await;
}

#[test_with_client]
async fn login_no_session_invalid_credentials() {
    let client = TestClient::new(&ctx.endpoint);
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);

    let res = client
        .post("/auth/login")
        .body_json(&json!({ "username": "", "password": "", "remember": false }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "invalid-credentials").await;
}

#[test_with_client]
async fn login_expired_session_invalid_credentials() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    let (session_id, csrf_token) = setup::add_session(&user, true, &ctx.config).await;
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie]
        .map(|cookie| cookie.to_string())
        .join(";");

    let res = client
        .post("/auth/login")
        .body_json(&json!({ "username": user.username, "password": "", "remember": false }))
        .header(COOKIE, cookies)
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only removes session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
    check_session_cookie_removed(&cookie, &ctx.config);

    assert_error(res, "auth", "invalid-credentials").await;
}

#[test_with_client]
async fn login_invalid_session_invalid_credentials() {
    let client = TestClient::new(&ctx.endpoint);
    let session_id = generate_token(ctx.config.session.id_length);
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);
    let session_cookie = Cookie::new_with_str(&ctx.config.session.cookie, &session_id);
    let cookies = [csrf_cookie, session_cookie]
        .map(|cookie| cookie.to_string())
        .join(";");

    let res = client
        .post("/auth/login")
        .body_json(&json!({ "username": "", "password": "", "remember": false }))
        .header(COOKIE, cookies)
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    // Check that Set-Cookie only removes session
    let headers = res.0.headers().clone();
    let cookies: Vec<_> = headers.get_all(SET_COOKIE).iter().collect();
    assert_eq!(cookies.len(), 1);
    let cookie = Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
    check_session_cookie_removed(&cookie, &ctx.config);

    assert_error(res, "auth", "invalid-credentials").await;
}

#[test_with_client]
async fn login_missing_totp() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', true, &ctx.config).await;
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "missing-totp").await;
}

#[test_with_client]
async fn login_invalid_totp() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', true, &ctx.config).await;
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": "should be only digits",
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);

    res.assert_header_is_not_exist(SET_COOKIE);
    assert_error(res, "auth", "invalid-totp").await;
}

#[test_with_client]
async fn login_totp_reuse() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', true, &ctx.config).await;
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);
    let now = utc_now().timestamp() as u64;
    let totp = generate_totp(&user.totp_key.unwrap(), now, &ctx.config);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": totp,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::OK);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": totp,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);
    assert_error(res, "auth", "totp-reuse").await;
}

#[test_with_client]
async fn login_disabled() {
    let client = TestClient::new(&ctx.endpoint);
    let user = setup::add_user('a', false, &ctx.config).await;
    ctx.db
        .execute(
            r#"UPDATE "users" SET "active" = false WHERE "id" = $1"#,
            &[&user.id],
        )
        .await
        .unwrap();
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    check_response(&res, StatusCode::BAD_REQUEST);
    assert_error(res, "auth", "account-disabled").await;
}

#[test_with_client]
async fn login_success_without_totp() {
    let client = TestClient::new(&ctx.endpoint);

    let user = setup::add_user('a', false, &ctx.config).await;
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;
    let row = ctx
        .db
        .query_one(r#"SELECT "csrf_token", "sudo_until" FROM "sessions""#, &[])
        .await
        .unwrap();

    check_response(&res, StatusCode::OK);
    res.assert_json(json!({
        "csrf_token": row.get::<_, &str>("csrf_token"),
        "user": {
            "id": user.id,
            "username": user.username,
            "totp_enabled": false,
            "password_change_reason": null,
            "icon": null,
            "locale": user.locale,
            "sudo_until": row.get::<_, DateTime<Utc>>("sudo_until").to_rfc3339(),
        },
    }))
    .await;
}

#[test_with_client]
async fn login_success_unused_totp() {
    let client = TestClient::new(&ctx.endpoint);

    let user = setup::add_user('a', false, &ctx.config).await;
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": "123456",
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;

    let row = ctx
        .db
        .query_one(r#"SELECT "csrf_token", "sudo_until" FROM "sessions""#, &[])
        .await
        .unwrap();
    check_response(&res, StatusCode::OK);
    res.assert_json(json!({
        "csrf_token": row.get::<_, &str>("csrf_token"),
        "user": {
            "id": user.id,
            "username": user.username,
            "totp_enabled": false,
            "password_change_reason": null,
            "icon": null,
            "locale": user.locale,
            "sudo_until": row.get::<_, DateTime<Utc>>("sudo_until").to_rfc3339(),
        },
        "warnings": [{"source": "auth", "id": "unused-totp"}],
    }))
    .await;
}

#[test_with_client]
async fn login_success_with_totp() {
    let client = TestClient::new(&ctx.endpoint);

    let user = setup::add_user('a', true, &ctx.config).await;
    let csrf_token = generate_token(ctx.config.csrf.token_length);
    let csrf_cookie = Cookie::new_with_str(&ctx.config.csrf.cookie, &csrf_token);
    let now = utc_now().timestamp() as u64;
    let totp = generate_totp(&user.totp_key.unwrap(), now, &ctx.config);

    let res = client
        .post("/auth/login")
        .body_json(&json!({
            "username": user.username,
            "password": user.password,
            "remember": false,
            "totp": totp,
        }))
        .header(COOKIE, csrf_cookie.to_string())
        .header(&ctx.config.csrf.header, &csrf_token)
        .send()
        .await;

    let row = ctx
        .db
        .query_one(r#"SELECT "csrf_token", "sudo_until" FROM "sessions""#, &[])
        .await
        .unwrap();
    check_response(&res, StatusCode::OK);
    res.assert_json(json!({
        "csrf_token": row.get::<_, &str>("csrf_token"),
        "user": {
            "id": user.id,
            "username": user.username,
            "totp_enabled": true,
            "password_change_reason": null,
            "icon": null,
            "locale": user.locale,
            "sudo_until": row.get::<_, DateTime<Utc>>("sudo_until").to_rfc3339(),
        },
    }))
    .await;
}
