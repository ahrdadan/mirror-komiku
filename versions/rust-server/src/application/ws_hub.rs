use tokio::sync::broadcast;

use crate::application::state::AppState;
use crate::domain::models::WsEvent;

const PREFETCH_EVENT_BUFFER: usize = 32;

pub async fn ws_subscribe(state: &AppState, chapter_hash: &str) -> broadcast::Receiver<WsEvent> {
    let tx = ws_sender(state, chapter_hash).await;
    tx.subscribe()
}

pub async fn ws_boot_events(state: &AppState, chapter_hash: &str) -> Vec<WsEvent> {
    let mut out = Vec::new();

    let init_event = {
        let map = state.ws_init_events.lock().await;
        map.get(chapter_hash).cloned()
    };
    if let Some(event) = init_event {
        out.push(event);
    }

    let prefetched_events = {
        let map = state.ws_prefetch_events.lock().await;
        map.get(chapter_hash).cloned().unwrap_or_default()
    };
    out.extend(prefetched_events);

    out
}

pub async fn ws_emit(state: &AppState, chapter_hash: &str, event: WsEvent) {
    if matches!(&event, WsEvent::ChapterInit { .. } | WsEvent::RawChapterInit { .. }) {
        let mut map = state.ws_init_events.lock().await;
        map.insert(chapter_hash.to_string(), event.clone());
        drop(map);

        let mut prefetch_map = state.ws_prefetch_events.lock().await;
        prefetch_map.remove(chapter_hash);
    }
    if matches!(&event, WsEvent::RawPrefetchedChapter { .. }) {
        let mut map = state.ws_prefetch_events.lock().await;
        let entry = map.entry(chapter_hash.to_string()).or_default();
        entry.push(event.clone());
        if entry.len() > PREFETCH_EVENT_BUFFER {
            let remove_count = entry.len() - PREFETCH_EVENT_BUFFER;
            entry.drain(0..remove_count);
        }
    }
    let tx = ws_sender(state, chapter_hash).await;
    let _ = tx.send(event);
}

pub async fn ws_drop(state: &AppState, chapter_hash: &str) {
    let mut map = state.ws_channels.lock().await;
    map.remove(chapter_hash);
    drop(map);

    let mut init_map = state.ws_init_events.lock().await;
    init_map.remove(chapter_hash);

    let mut prefetch_map = state.ws_prefetch_events.lock().await;
    prefetch_map.remove(chapter_hash);
}

async fn ws_sender(state: &AppState, chapter_hash: &str) -> broadcast::Sender<WsEvent> {
    let mut map = state.ws_channels.lock().await;
    if let Some(tx) = map.get(chapter_hash) {
        return tx.clone();
    }
    let (tx, _) = broadcast::channel(512);
    map.insert(chapter_hash.to_string(), tx.clone());
    tx
}
