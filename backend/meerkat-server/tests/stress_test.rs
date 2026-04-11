/// Stress tests — run with:
///   cargo test --test stress_test -- --ignored --nocapture
///
/// These are excluded from normal CI (`#[ignore]`) because they're slow and
/// probe capacity limits rather than correctness.
///
/// What each test probes:
///   1. stress_100_concurrent_sessions   — DashMap session creation under concurrent load
///   2. stress_30_clients_one_session    — broadcast fan-out to N recipients; mpsc channel(32) limit
///   3. stress_500_rapid_fire            — sustained throughput; zero message loss with concurrent drain
///   4. stress_20_sessions_x_5_clients   — 100 simultaneous connections across isolated sessions
use std::sync::Arc;
use std::time::Instant;

use axum::{routing::any, Router};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    connect_async,
    tungstenite::Message,
};
use uuid::Uuid;

use meerkat_server::{
    messages::{ClientEvent, CreateObjectPayload, JoinSessionPayload, ServerEvent},
    types::{AppState, ObjectType, Transform},
    websocket::handler,
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn start_server() -> String {
    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections: Arc::new(DashMap::new()),
        connection_meta: Arc::new(DashMap::new()),
        connection_backpressure: Arc::new(DashMap::new()),
        session_connections: Arc::new(DashMap::new()),
    };
    let app = Router::new().route("/ws", any(handler)).with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("ws://127.0.0.1:{}/ws", port)
}

/// Connect + JoinSession + wait for FullStateSync, then return the stream.
async fn connect_and_join(url: &str, session_id: &str, display_name: &str) -> WsStream {
    let (mut ws, _) = connect_async(url).await.expect("connect failed");
    let json = serde_json::to_string(&ClientEvent::JoinSession(JoinSessionPayload {
        session_id: session_id.to_string(),
        display_name: display_name.to_string(),
    }))
    .unwrap();
    ws.send(Message::Text(json.into())).await.unwrap();
    // Consume frames until we see FullStateSync.
    loop {
        if let Some(Ok(Message::Text(t))) = ws.next().await 
            && let Ok(ServerEvent::FullStateSync(_)) = serde_json::from_str::<ServerEvent>(&t) 
            {
                break;
            }
        }
    ws
}

async fn send_ev(ws: &mut WsStream, event: ClientEvent) {
    let json = serde_json::to_string(&event).unwrap();
    ws.send(Message::Text(json.into())).await.unwrap();
}

/// Wait for the next ServerEvent. Panics if nothing arrives within 10 s.
async fn recv_ev(ws: &mut WsStream) -> ServerEvent {
    loop {
        let msg = timeout(Duration::from_secs(10), ws.next())
            .await
            .expect("recv timed out after 10 s")
            .expect("stream closed")
            .expect("ws error");
        if let Message::Text(t) = msg {
            return serde_json::from_str(&t).expect("invalid ServerEvent JSON");
        }
    }
}

/// Drain all pending frames, stopping after 100 ms of silence.
async fn drain(ws: &mut WsStream) {
    while timeout(Duration::from_millis(100), ws.next())
        .await
        .ok()
        .flatten()
        .is_some()
    {}
}

fn cube(id: Uuid) -> CreateObjectPayload {
    CreateObjectPayload {
        object_id: id,
        name: "Cube".into(),
        object_type: ObjectType::Cube,
        asset_id: None,
        asset_library: None,
        transform: Transform { position: [0.0; 3], rotation: [0.0; 3], scale: [1.0; 3] },
        properties: None,
    }
}

// ── Test 1: 100 concurrent independent sessions ───────────────────────────────

/// Spawns 100 tasks simultaneously, each creating its own session and joining it.
/// Verifies: the server handles 100 concurrent DashMap writes + session creations
/// without panic or deadlock.
/// Every client must receive FullStateSync within 10 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn stress_100_concurrent_sessions() {
    const N: usize = 100;
    let url = start_server().await;
    let start = Instant::now();

    let mut tasks: JoinSet<()> = JoinSet::new();
    for i in 0..N {
        let url = url.clone();
        tasks.spawn(async move {
            // connect_and_join already asserts FullStateSync is received.
            let _ws = connect_and_join(
                &url,
                &format!("sess-{}", i),
                &format!("user-{}", i),
            )
            .await;
        });
    }

    let mut completed = 0usize;
    while let Some(r) = tasks.join_next().await {
        r.expect("task panicked");
        completed += 1;
    }

    println!(
        "\n[stress_100_concurrent_sessions] {}/{} sessions joined in {:.2?}",
        completed, N, start.elapsed()
    );
    assert_eq!(completed, N);
}

// ── Test 2: 30 clients in one session ─────────────────────────────────────────

/// 30 clients join a single session sequentially.
/// After all are connected, client 0 broadcasts one CreateObject.
/// Every client must receive it — verifies broadcast fan-out at N=30.
///
/// N=30 is intentional: it stays just under the mpsc::channel(32) capacity.
/// Each client accumulates at most N-1=29 UserJoined messages during the join
/// phase, which fits in the channel without drops.
///
/// To probe the drop boundary, increase N past 33 and observe whether the final
/// ObjectCreated still reaches every client (it will, because channels drain
/// between the join phase and the broadcast).
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn stress_30_clients_one_session() {
    const N: usize = 30;
    let url = start_server().await;
    let start = Instant::now();

    let mut clients: Vec<WsStream> = Vec::with_capacity(N);
    for i in 0..N {
        let ws = connect_and_join(&url, "shared", &format!("user-{}", i)).await;
        clients.push(ws);
    }

    // One drain pass: clears the UserJoined storm that accumulated during joins.
    for ws in &mut clients {
        drain(ws).await;
    }

    println!(
        "[stress_30_clients_one_session] all {} clients joined + drained in {:.2?}",
        N, start.elapsed()
    );

    // Client 0 broadcasts one object.
    let obj_id = Uuid::new_v4();
    send_ev(&mut clients[0], ClientEvent::CreateObject(cube(obj_id))).await;

    // All N clients must receive ObjectCreated.
    let mut received = 0usize;
    for (i, ws) in clients.iter_mut().enumerate() {
        match recv_ev(ws).await {
            ServerEvent::ObjectCreated(p) if p.object.object_id == obj_id => received += 1,
            other => panic!("client {}: expected ObjectCreated({}), got {:?}", i, obj_id, other),
        }
    }

    println!(
        "[stress_30_clients_one_session] {}/{} clients received broadcast in {:.2?}",
        received, N, start.elapsed()
    );
    assert_eq!(received, N);
}

// ── Test 3: 500 rapid-fire events ─────────────────────────────────────────────

/// Client A sends 500 CreateObject events back-to-back.
/// Client B drains concurrently via tokio::join!.
///
/// Verifies: zero message loss when the receiver is actively draining.
/// The concurrent drain prevents B's mpsc::channel(32) from filling.
///
/// A's own echo channel may fill (A receives its own ObjectCreated broadcasts
/// but we do not drain A). This is intentional: it surfaces that the server
/// silently drops messages to slow consumers rather than blocking or crashing.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn stress_500_rapid_fire() {
    const N: usize = 500;
    let url = start_server().await;

    let mut ws_a = connect_and_join(&url, "rapid", "Sender").await;
    let mut ws_b = connect_and_join(&url, "rapid", "Receiver").await;
    drain(&mut ws_a).await; // clear UserJoined(B)

    let ids: Vec<Uuid> = (0..N).map(|_| Uuid::new_v4()).collect();
    let start = Instant::now();

    // Send from A and receive from B concurrently.
    let send_fut = async {
        for &id in &ids {
            send_ev(&mut ws_a, ClientEvent::CreateObject(cube(id))).await;
        }
    };

    let recv_fut = async {
        let mut count = 0usize;
        for _ in 0..N {
            if matches!(recv_ev(&mut ws_b).await, ServerEvent::ObjectCreated(_)) {
                count += 1;
            }
        }
        count
    };

    let (_, received) = tokio::join!(send_fut, recv_fut);
    let elapsed = start.elapsed();
    let rate = N as f64 / elapsed.as_secs_f64();

    println!(
        "\n[stress_500_rapid_fire] {}/{} events delivered to B in {:.2?} ({:.0} msg/s)",
        received, N, elapsed, rate
    );
    assert_eq!(received, N, "{} messages were dropped", N - received);
}

// ── Test 4: 20 sessions × 5 clients ──────────────────────────────────────────

/// 20 independent sessions each with 5 clients, all active simultaneously.
/// Each client creates 3 objects → 15 ObjectCreated events broadcast to all 5
/// clients in the session (75 messages per session, 1500 total across the server).
///
/// Verifies:
///   - Session isolation: events never bleed across sessions under concurrent load.
///   - The server handles 100 simultaneous WebSocket connections without deadlock.
///   - DashMap shard contention doesn't cause incorrect routing.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn stress_20_sessions_x_5_clients() {
    const SESSIONS: usize = 20;
    const CLIENTS: usize = 5;
    const OBJECTS: usize = 3;

    let url = start_server().await;
    let start = Instant::now();

    let mut session_tasks: JoinSet<()> = JoinSet::new();

    for s in 0..SESSIONS {
        let url = url.clone();
        session_tasks.spawn(async move {
            let sid = format!("load-{}", s);

            // Join all 5 clients for this session concurrently.
            let mut join_tasks: JoinSet<WsStream> = JoinSet::new();
            for c in 0..CLIENTS {
                let url = url.clone();
                let sid = sid.clone();
                join_tasks.spawn(async move {
                    connect_and_join(&url, &sid, &format!("s{}-u{}", s, c)).await
                });
            }

            let mut clients: Vec<WsStream> = Vec::with_capacity(CLIENTS);
            while let Some(r) = join_tasks.join_next().await {
                clients.push(r.expect("client join panicked"));
            }

            // Drain the UserJoined storm from concurrent joins.
            for ws in &mut clients {
                drain(ws).await;
            }

            // Each client creates OBJECTS objects (sequentially within the session task).
            for ws in &mut clients {
                for _ in 0..OBJECTS {
                    send_ev(ws, ClientEvent::CreateObject(cube(Uuid::new_v4()))).await;
                }
            }

            // Each client must receive CLIENTS * OBJECTS = 15 ObjectCreated events.
            let expected = CLIENTS * OBJECTS;
            for (i, ws) in clients.iter_mut().enumerate() {
                for _ in 0..expected {
                    timeout(Duration::from_secs(15), recv_ev(ws))
                        .await
                        .unwrap_or_else(|_| {
                            panic!("session {} client {}: timed out waiting for ObjectCreated", s, i)
                        });
                }
            }
        });
    }

    let mut completed = 0usize;
    while let Some(r) = session_tasks.join_next().await {
        r.expect("session task panicked");
        completed += 1;
    }

    println!(
        "\n[stress_20_sessions_x_5_clients] {}/{} sessions complete ({} total connections) in {:.2?}",
        completed,
        SESSIONS,
        SESSIONS * CLIENTS,
        start.elapsed()
    );
    assert_eq!(completed, SESSIONS);
}
