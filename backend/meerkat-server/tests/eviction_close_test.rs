use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;

use meerkat_server::messages::{ClientEvent, JoinSessionPayload, ServerEvent};

mod common;

use common::{recv, send, start_test_server_with_state};

#[tokio::test]
async fn test_evicted_client_receives_close_code_4008() {
    let (url, state) = start_test_server_with_state().await;
    let session_id = "evict-close";

    let (mut ws, _) = connect_async(&url).await.expect("connect failed");
    send(&mut ws, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: session_id.to_string(),
        display_name: "Alice".to_string(),
    })).await;
    let sync = recv(&mut ws).await;
    assert!(matches!(sync, ServerEvent::FullStateSync(_)));

    let conn_id = timeout(Duration::from_secs(2), async {
        loop {
            for entry in state.connection_meta.iter() {
                let (sid, _) = entry.value();
                if sid == session_id {
                    return *entry.key();
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("timed out waiting for connection metadata");

    let removed = state.connections.remove(&conn_id);
    assert!(removed.is_some(), "expected active sender for joined connection");
    drop(removed);

    let close_frame = timeout(Duration::from_secs(5), async {
        loop {
            let msg = ws
                .next()
                .await
                .expect("stream closed before close frame")
                .expect("websocket error while waiting for close");
            if let Message::Close(frame) = msg {
                return frame;
            }
        }
    })
    .await
    .expect("timed out waiting for close frame");

    let frame = close_frame.expect("expected close frame details");
    assert_eq!(u16::from(frame.code), 4008, "expected eviction close code 4008");
}
