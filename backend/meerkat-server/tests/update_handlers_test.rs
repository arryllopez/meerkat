use tokio_tungstenite::connect_async;
use uuid::Uuid;

use meerkat_server::{
    messages::{
        ClientEvent, CreateSessionPayload, JoinSessionPayload, SelectObjectPayload, ServerEvent, UpdateNamePayload,
        UpdatePropertiesPayload, UpdateTransformPayload,
    },
    types::{ObjectProperties, PointLightProperties, Transform},
};

mod common;

use common::{cube_payload, recv, send, start_test_server};

/// Exercises UpdateTransform, UpdateName, UpdateProperties, and SelectObject
/// in sequence, asserting that both clients receive every broadcast.
#[tokio::test]
async fn test_update_handlers() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "update-test".to_string(),
        display_name: "Alice".to_string(),
        password: "somepassword".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "update-test".to_string(),
        display_name: "Bob".to_string(),
        password: "somepassword".to_string(),
    })).await;
    recv(&mut ws_b).await; // FullStateSync
    recv(&mut ws_a).await; // UserJoined(Bob)

    // Create an object so the update handlers have something to mutate.
    let object_id = Uuid::new_v4();
    // let asset_id: Option<String> = Some("dragon".to_string());
    // let asset_library: Option<String> = Some("workLibrary".to_string());
    send(&mut ws_a, ClientEvent::CreateObject(cube_payload(object_id))).await;
    // send(&mut ws_a, ClientEvent::CreateObject(asset_payload(object_id, "Dragon", asset_id, asset_library,))).await;
    // send(&mut ws_a, ClientEvent::CreateObject(asset_payload(object_id, "Tree", asset_id, asset_library,))).await;
    recv(&mut ws_a).await; // ObjectCreated (echo to sender)
    recv(&mut ws_b).await; // ObjectCreated

    // ── UpdateTransform ───────────────────────────────────────────────────────
    let new_transform = Transform { position: [5.0, 10.0, 15.0], rotation: [0.1, 0.2, 0.3], scale: [2.0; 3] };
    send(&mut ws_a, ClientEvent::UpdateTransform(UpdateTransformPayload {
        object_id,
        transform: new_transform.clone(),
    })).await;

    let tf_a = recv(&mut ws_a).await;
    let tf_b = recv(&mut ws_b).await;

    match &tf_a {
        ServerEvent::TransformUpdated(p) => {
            assert_eq!(p.object_id, object_id);
            assert_eq!(p.transform.position, new_transform.position);
        }
        _ => panic!("A: expected TransformUpdated, got {:?}", tf_a),
    }
    assert!(matches!(tf_b, ServerEvent::TransformUpdated(_)), "B: expected TransformUpdated");

    // ── UpdateName ────────────────────────────────────────────────────────────
    send(&mut ws_a, ClientEvent::UpdateName(UpdateNamePayload {
        object_id,
        name: "renamed_cube".to_string(),
    })).await;

    let name_a = recv(&mut ws_a).await;
    let name_b = recv(&mut ws_b).await;

    match &name_a {
        ServerEvent::NameUpdated(p) => {
            assert_eq!(p.object_id, object_id);
            assert_eq!(p.name, "renamed_cube");
        }
        _ => panic!("A: expected NameUpdated, got {:?}", name_a),
    }
    assert!(matches!(name_b, ServerEvent::NameUpdated(_)), "B: expected NameUpdated");

    // ── UpdateProperties ──────────────────────────────────────────────────────
    let props = ObjectProperties::PointLight(PointLightProperties {
        color: [1.0, 0.5, 0.0],
        use_temperature: false,
        temperature: 6500.0,
        exposure: 0.0,
        power: 100.0,
        radius: 0.1,
        soft_falloff: true,
        normalize: false,
        cast_shadow: true,
        shadow_jitter: false,
        shadow_jitter_overblur: 0.0,
        shadow_filter_radius: 1.0,
        shadow_maximum_resolution: 0.001,
        diffuse_factor: 1.0,
        specular_factor: 1.0,
        transmission_factor: 1.0,
        volume_factor: 1.0,
        use_custom_distance: false,
        cutoff_distance: 40.0,
    });
    send(&mut ws_a, ClientEvent::UpdateProperties(UpdatePropertiesPayload {
        object_id,
        properties: props,
    })).await;

    let props_a = recv(&mut ws_a).await;
    let props_b = recv(&mut ws_b).await;

    match &props_a {
        ServerEvent::PropertiesUpdated(p) => assert_eq!(p.object_id, object_id),
        _ => panic!("A: expected PropertiesUpdated, got {:?}", props_a),
    }
    assert!(matches!(props_b, ServerEvent::PropertiesUpdated(_)), "B: expected PropertiesUpdated");

    // ── SelectObject ──────────────────────────────────────────────────────────
    send(&mut ws_a, ClientEvent::SelectObject(SelectObjectPayload {
        object_id: Some(object_id),
    })).await;

    let sel_a = recv(&mut ws_a).await;
    let sel_b = recv(&mut ws_b).await;

    match &sel_a {
        ServerEvent::UserSelected(p) => assert_eq!(p.object_id, Some(object_id)),
        _ => panic!("A: expected UserSelected, got {:?}", sel_a),
    }
    assert!(matches!(sel_b, ServerEvent::UserSelected(_)), "B: expected UserSelected");

    // ── Deselect ──────────────────────────────────────────────────────────────
    send(&mut ws_a, ClientEvent::SelectObject(SelectObjectPayload { object_id: None })).await;
    let desel_a = recv(&mut ws_a).await;
    match &desel_a {
        ServerEvent::UserSelected(p) => assert!(p.object_id.is_none(), "expected deselect (None)"),
        _ => panic!("A: expected UserSelected(None), got {:?}", desel_a),
    }
    let desel_b = recv(&mut ws_b).await; 
    match &desel_b {
        ServerEvent::UserSelected(p) => assert!(p.object_id.is_none(), "expected deselect (None)"),
        _ => panic!("B: expected UserSelected(None), got {:?}", desel_b),
    }
}
