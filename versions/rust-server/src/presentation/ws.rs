use actix_web::{web, Error as ActixError, HttpRequest, HttpResponse};
use actix_ws::{Message, MessageStream, Session};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::broadcast;
use url::Url;

use crate::application::chapter_service::prefetch_raw_next_chapters;
use crate::application::state::AppState;
use crate::application::ws_hub::{ws_boot_events, ws_subscribe};
use crate::domain::models::WsEvent;
use crate::infrastructure::target::decode_and_validate_target;

async fn push_event(session: &mut Session, event: WsEvent) -> Result<(), ()> {
    let payload = match serde_json::to_string(&event) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    session.text(payload).await.map_err(|_| ())
}

async fn resolve_raw_prefetch_seed(state: &AppState, chapter_hash: &str) -> Option<Url> {
    let init_event = {
        let map = state.ws_init_events.lock().await;
        map.get(chapter_hash).cloned()
    };
    let next_raw_path = match init_event {
        Some(WsEvent::RawChapterInit {
            next_raw_path: Some(path),
            ..
        }) => path,
        _ => return None,
    };
    let target = next_raw_path.strip_prefix("/raw/")?;
    decode_and_validate_target(target, &state.allowed_domains)
        .await
        .ok()
}

async fn handle_client_text(state: &AppState, chapter_hash: &str, text: &str) {
    let parsed: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };
    let msg_type = parsed
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if msg_type != "raw_prefetch_request" {
        return;
    }

    let depth = parsed
        .get("depth")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 5) as usize;

    let already_sent = {
        let map = state.ws_prefetch_events.lock().await;
        map.get(chapter_hash).map(|v| v.len()).unwrap_or(0)
    };
    let remaining_depth = depth.saturating_sub(already_sent);
    if remaining_depth == 0 {
        return;
    }

    let seed_from_client = parsed
        .get("seed_next_raw_path")
        .and_then(Value::as_str)
        .and_then(|p| p.strip_prefix("/raw/"))
        .map(str::to_string);

    let seed_next_url = if let Some(raw_target) = seed_from_client {
        decode_and_validate_target(&raw_target, &state.allowed_domains)
            .await
            .ok()
    } else {
        resolve_raw_prefetch_seed(state, chapter_hash).await
    };

    let Some(seed_next_url) = seed_next_url else {
        return;
    };

    let _ =
        prefetch_raw_next_chapters(Some(seed_next_url), chapter_hash, state, remaining_depth).await;
}

async fn ws_loop(
    mut session: Session,
    mut msg_stream: MessageStream,
    mut rx: broadcast::Receiver<WsEvent>,
    boot_events: Vec<WsEvent>,
    state: AppState,
    chapter_hash: String,
) {
    let mut close_reason = None;

    for event in boot_events {
        if push_event(&mut session, event).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = msg_stream.next() => {
                match incoming {
                    Some(Ok(Message::Ping(bytes))) => {
                        if session.pong(&bytes).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        handle_client_text(&state, &chapter_hash, text.as_ref()).await;
                    }
                    Some(Ok(Message::Close(reason))) => {
                        close_reason = reason;
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            incoming = rx.recv() => {
                match incoming {
                    Ok(event) => {
                        if push_event(&mut session, event).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }
    }

    let _ = session.close(close_reason).await;
}

pub async fn handle_ws(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ActixError> {
    let chapter_hash = path.into_inner();
    let rx = ws_subscribe(state.get_ref(), &chapter_hash).await;
    let boot_events = ws_boot_events(state.get_ref(), &chapter_hash).await;

    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    actix_web::rt::spawn(ws_loop(
        session,
        msg_stream,
        rx,
        boot_events,
        state.get_ref().clone(),
        chapter_hash,
    ));
    Ok(res)
}
