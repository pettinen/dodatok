use std::{collections::HashMap, sync::Arc};

use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use poem::web::websocket::{Message, WebSocketStream};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value as JsonValue};
use tokio::{sync::{broadcast, Mutex as AsyncMutex}, task::JoinHandle};

use crate::{
    error::{AuthError, GeneralError, InternalError, WebSocketError},
    util::{generate_token, redis_join},
    CONFIG,
};

pub type AccountConnections = Arc<AsyncMutex<HashMap<String, HashMap<String, WebSocketConnection>>>>;
pub type AccountRooms = Arc<AsyncMutex<HashMap<String, broadcast::Sender<String>>>>;
type WebSocketSink = SplitSink<WebSocketStream, Message>;

enum AccountEvent {
    Authenticate(AuthenticateEvent),
}

impl<'de> Deserialize<'de> for AccountEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum AccountEventType {
            Authenticate,
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Event {
            event: AccountEventType,
            data: JsonValue,
        }

        let event = Event::deserialize(deserializer)?;
        match event.event {
            AccountEventType::Authenticate => AuthenticateEvent::deserialize(event.data)
                    .map(AccountEvent::Authenticate)
                    .map_err(de::Error::custom),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthenticateEvent {
    token: String,
}

pub struct WebSocketConnection {
    room_handles: HashMap<String, JoinHandle<()>>,
    sink: Arc<AsyncMutex<WebSocketSink>>,
    pub user_id: Option<String>,
}

impl WebSocketConnection {
    pub fn new(sink: WebSocketSink) -> Self {
        Self {
            room_handles: HashMap::new(),
            sink: Arc::new(AsyncMutex::new(sink)),
            user_id: None,
        }
    }

    pub async fn disconnect(&mut self, rooms: &AccountRooms) {
        let mut rooms = rooms.lock().await;

        for (room, handle) in self.room_handles.drain() {
            handle.abort();
            let _ = handle.await;

            if let Some(sender) = rooms.get(&room) {
                if sender.receiver_count() == 0 {
                    rooms.remove(&room);
                }
            }
        }
    }

    pub async fn enter_room(&mut self, room: String, rooms: &AccountRooms) -> Result<(), WebSocketError> {
        if self.room_handles.contains_key(&room) {
            return Err(WebSocketError::AlreadyInRoom);
        }
        let mut rooms = rooms.lock().await;
        let mut receiver = match rooms.get(&room) {
            Some(sender) => sender.subscribe(),
            None => {
                let (sender, receiver) = broadcast::channel::<String>(CONFIG.websocket.channel_capacity);
                rooms.insert(room.clone(), sender);
                receiver
            },
        };
        let sink = self.sink.clone();
        let handle = tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                let mut sink = sink.lock().await;
                if sink.send(Message::Text(message)).await.is_err() {
                    break;
                }
            }
        });

        self.room_handles.insert(room.to_owned(), handle);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn leave_room(&mut self, room: &str, rooms: &AccountRooms) -> Result<(), WebSocketError> {
        if let Some(handle) = self.room_handles.remove(room) {
            handle.abort();
            let _ = handle.await;
            let mut rooms = rooms.lock().await;
            if let Some(sender) = rooms.get(room) {
                if sender.receiver_count() == 0 {
                    rooms.remove(room);
                }
            }
            Ok(())
        } else {
            Err(WebSocketError::NotInRoom)
        }
    }

    pub async fn send(&self, message: JsonValue) -> std::io::Result<()> {
        self.sink.lock().await.send(Message::Text(message.to_string())).await
    }

    pub async fn send_error<E: Serialize>(&self, error: E) -> std::io::Result<()> {
        self.sink.lock().await
            .send(Message::Text(json!({"error": error}).to_string()))
            .await
    }
}

fn get_event(message: String) -> Result<AccountEvent, GeneralError> {
    serde_json::from_str::<AccountEvent>(&message)
        .map_err(|err| GeneralError::InvalidData(err.to_string()))
}

pub async fn websocket_receiver(
    socket: WebSocketStream,
    connections: AccountConnections,
    rooms: AccountRooms,
    redis: RedisClient,
) {
    let mut redis = match redis.get_async_connection().await {
        Ok(redis) => redis,
        Err(_) => return,
    };

    let (sink, mut stream) = socket.split();
    let mut connection = WebSocketConnection::new(sink);

    let connection_ids = loop {
        let message = match stream.next().await {
            Some(Ok(message)) => message,
            _ => break None,
        };

        let message = match message {
            Message::Text(text) => text,
            Message::Close(_) => break None,
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Binary(_) => {
                if connection.send_error(WebSocketError::InvalidMessageType).await.is_err() {
                    break None;
                }
                continue;
            },
        };

        let event = match get_event(message) {
            Ok(event) => event,
            Err(err) => {
                if connection.send_error(err).await.is_err() {
                    break None;
                }
                continue;
            },
        };
        let token = match event {
            AccountEvent::Authenticate(AuthenticateEvent { token }) => token,
        };
        let redis_key = redis_join(&["websocket-token", "account", &token]);
        let (user_id, session_id) = match redis.get::<_, String>(&redis_key).await {
            Ok(data) => match data.split_once(':') {
                Some((user_id, session_id)) => (user_id.to_owned(), session_id.to_owned()),
                None => {
                    if connection.send_error(
                        InternalError::new("invalid data in redis for websocket token"),
                    ).await.is_err() {
                        break None;
                    }
                    continue;
                },
            },
            Err(_) => {
                if connection.send_error(AuthError::InvalidCredentials).await.is_err() {
                    break None;
                }
                continue;
            },
        };

        let mut connections = connections.lock().await;
        if !connections.contains_key(&session_id) {
            connections.insert(session_id.to_owned(), HashMap::new());
        }
        let session_connections = match connections.get_mut(&session_id) {
            Some(connections) => connections,
            None => continue,
        };
        if redis.del::<_, usize>(redis_key).await.is_err() {
            InternalError::new("could not delete websocket token from redis");
        }
        if let Err(err) = connection.enter_room(format!("user:{}", &user_id), &rooms).await {
            if connection.send_error(InternalError::new(err)).await.is_err() {
                break None;
            }
            continue;
        }
        connection.user_id = Some(user_id.clone());
        if connection.send(json!({"event": "authenticated"})).await.is_err() {
            break None;
        }

        let connection_id = generate_token(CONFIG.websocket.connection_id_length);
        session_connections.insert(connection_id.clone(), connection);
        break Some((session_id, connection_id));
    };

    if let Some((ref session_id, ref connection_id)) = connection_ids {
        while let Some(Ok(message)) = stream.next().await {
            let mut connections = connections.lock().await;
            let session_connections = match connections.get_mut(session_id) {
                Some(session_connections) => session_connections,
                None => break,
            };
            let connection = match session_connections.get_mut(connection_id) {
                Some(connection) => connection,
                None => break,
            };
            if connection.user_id.is_none() {
                break;
            }
            let message = match message {
                Message::Text(text) => text,
                Message::Close(_) => break,
                Message::Ping(_) | Message::Pong(_) => continue,
                Message::Binary(_) => {
                    if connection.send_error(WebSocketError::InvalidMessageType).await.is_err() {
                        break;
                    }
                    continue;
                },
            };

            let event = match get_event(message) {
                Ok(event) => event,
                Err(err) => {
                    if connection.send_error(err).await.is_err() {
                        break;
                    }
                    continue;
                },
            };

            match event {
                AccountEvent::Authenticate(_) => {
                    if connection.send_error(AuthError::AlreadyLoggedIn).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    if let Some((ref session_id, ref connection_id)) = connection_ids {
        let mut connections = connections.lock().await;
        if let Some(session_connections) = connections.get_mut(session_id) {
            if let Some(connection) = session_connections.get_mut(connection_id) {
                connection.disconnect(&rooms).await;
            }
            session_connections.remove(connection_id);
            if session_connections.is_empty() {
                connections.remove(session_id);
            }
        }
    }
}
