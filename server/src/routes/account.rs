use poem::{
    get, handler, post,
    web::{websocket::WebSocket, Data},
    EndpointExt, IntoResponse, Response, Result, Route,
};
use redis::{AsyncCommands, Client as RedisClient};
use serde_json::json;

use crate::{
    error::{Forbidden, InternalError},
    middleware::{csrf, AuthRequired, CurrentUser},
    util::{base64_urlsafe, generate_token, json_response, redis_join},
    websocket::{websocket_receiver, AccountRooms, AccountConnections},
    CONFIG,
};

#[handler]
async fn websocket(
    req: &poem::Request,
    websocket: WebSocket,
    connections: Data<&AccountConnections>,
    rooms: Data<&AccountRooms>,
    redis: Data<&RedisClient>,
) -> Result<impl IntoResponse> {
    if req.headers().get("Origin") != Some(&CONFIG.client.origin) {
        Err(Forbidden)?;
    }

    let connections = connections.clone();
    let rooms = rooms.clone();
    let redis = redis.clone();
    Ok(websocket.on_upgrade(|socket| async {
        tokio::spawn(websocket_receiver(socket, connections, rooms, redis));
    }))
}

#[handler]
async fn websocket_token(user: Data<&CurrentUser>, redis: Data<&RedisClient>) -> Result<Response> {
    let mut redis = redis.get_async_connection().await.map_err(InternalError::new)?;
    let token = generate_token(CONFIG.websocket.token_length);
    let redis_key = redis_join(&["websocket-token", "account", &token]);
    let redis_value = format!("{}:{}", user.id, base64_urlsafe(&user.session_id_hash));
    redis
        .set_ex(redis_key, redis_value, CONFIG.websocket.token_lifetime)
        .await
        .map_err(InternalError::new)?;
    json_response(json!({
        "token": token,
    }))
}

#[handler]
async fn websocket_clients(connections: Data<&AccountConnections>, rooms: Data<&AccountRooms>) -> Result<Response> {
    let connections = connections.lock().await;
    let connections: std::collections::HashMap<_, _> = connections
        .iter()
        .map(|(session_id, session_connections)| {
            let session_connections: std::collections::HashMap<_, _> = session_connections
                .iter()
                .map(|(connection_id, connection)| {
                    (connection_id, &connection.user_id)
                })
                .collect();
            (session_id, session_connections)
        })
        .collect();
    let rooms = rooms.lock().await;
    let rooms: std::collections::HashMap<_, _> = rooms
        .iter()
        .map(|(room, sender)| (room, sender.receiver_count()))
        .collect();

    json_response(json!({
        "connections": connections,
        "rooms": rooms,
    }))
}

pub fn routes() -> Route {
    Route::new()
        .at("/socket", get(websocket))
        .at("/socket/clients", get(websocket_clients))
        .at("/socket/token", post(websocket_token.with(AuthRequired::default()).around(csrf)))
}
