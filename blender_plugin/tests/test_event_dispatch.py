"""Tests for event dispatch — dict lookups, handler wiring, unknown events."""
from blender_plugin.event_handlers import (
    EVENT_HANDLERS,
    OBJECT_CREATORS,
    PROPERTY_APPLIERS,
    PROPERTY_BUILDERS,
    _get_property_builder,
    handle_full_state_sync,
    handle_object_created,
    handle_object_deleted,
    handle_transform_updated,
    handle_properties_updated,
    handle_name_updated,
    handle_user_joined,
    handle_user_left,
)
from blender_plugin.panels import _connection_status_lines
from blender_plugin.tests.helpers import reset_state, clear_scene, TestResult


def run(result):
    print("\n--- Event Dispatch Tests ---")

    # ── EVENT_HANDLERS has all expected keys ──

    expected_events = [
        "FullStateSync", "ObjectCreated", "ObjectDeleted",
        "TransformUpdated", "PropertiesUpdated", "NameUpdated",
        "UserJoined", "UserLeft",
    ]
    for event in expected_events:
        if event in EVENT_HANDLERS:
            result.ok(f"EVENT_HANDLERS contains '{event}'")
        else:
            result.fail(f"EVENT_HANDLERS contains '{event}'")

    # ── EVENT_HANDLERS maps to correct functions ──

    mappings = {
        "FullStateSync": handle_full_state_sync,
        "ObjectCreated": handle_object_created,
        "ObjectDeleted": handle_object_deleted,
        "TransformUpdated": handle_transform_updated,
        "PropertiesUpdated": handle_properties_updated,
        "NameUpdated": handle_name_updated,
        "UserJoined": handle_user_joined,
        "UserLeft": handle_user_left,
    }
    for event, func in mappings.items():
        if EVENT_HANDLERS[event] is func:
            result.ok(f"EVENT_HANDLERS['{event}'] → correct handler")
        else:
            result.fail(f"EVENT_HANDLERS['{event}'] → correct handler")

    # ── Unknown event type returns None (no crash) ──

    handler = EVENT_HANDLERS.get("NonexistentEvent")
    if handler is None:
        result.ok("unknown event → returns None")
    else:
        result.fail("unknown event → returns None")

    # ── OBJECT_CREATORS has all expected types ──

    expected_types = ["Cube", "Sphere", "Cylinder", "Camera", "PointLight", "SunLight"]
    for obj_type in expected_types:
        if obj_type in OBJECT_CREATORS:
            result.ok(f"OBJECT_CREATORS contains '{obj_type}'")
        else:
            result.fail(f"OBJECT_CREATORS contains '{obj_type}'")

    # ── OBJECT_CREATORS does NOT contain AssetRef (special-cased) ──

    if "AssetRef" not in OBJECT_CREATORS:
        result.ok("OBJECT_CREATORS does not contain 'AssetRef' (special-cased)")
    else:
        result.fail("OBJECT_CREATORS does not contain 'AssetRef'")

    # ── PROPERTY_APPLIERS has all expected types ──

    expected_prop_types = ["Camera", "PointLight", "SunLight"]
    for prop_type in expected_prop_types:
        if prop_type in PROPERTY_APPLIERS:
            result.ok(f"PROPERTY_APPLIERS contains '{prop_type}'")
        else:
            result.fail(f"PROPERTY_APPLIERS contains '{prop_type}'")

    # ── PROPERTY_BUILDERS has all expected types ──

    expected_builder_types = ["CAMERA", "POINT", "SUN"]
    for builder_type in expected_builder_types:
        if builder_type in PROPERTY_BUILDERS:
            result.ok(f"PROPERTY_BUILDERS contains '{builder_type}'")
        else:
            result.fail(f"PROPERTY_BUILDERS contains '{builder_type}'")

    # ── UserJoined populates users dict ──

    clear_scene()
    state, mock_ws = reset_state()

    handle_user_joined({
        "user_id": "new-user-001",
        "display_name": "Alice",
        "color": [255, 128, 0],
    })

    if "new-user-001" in state.users:
        result.ok("UserJoined adds user to state.users")
    else:
        result.fail("UserJoined adds user to state.users")

    if state.users["new-user-001"]["display_name"] == "Alice":
        result.ok("UserJoined stores correct display_name")
    else:
        result.fail("UserJoined stores correct display_name")

    if state.users["new-user-001"]["color"] == [255, 128, 0]:
        result.ok("UserJoined stores correct color")
    else:
        result.fail("UserJoined stores correct color")

    # ── UserLeft removes user from users dict ──

    handle_user_left({"user_id": "new-user-001"})

    if "new-user-001" not in state.users:
        result.ok("UserLeft removes user from state.users")
    else:
        result.fail("UserLeft removes user from state.users")

    # ── UserLeft for nonexistent user is no-op ──

    try:
        handle_user_left({"user_id": "ghost-user"})
        result.ok("UserLeft for nonexistent user → no crash")
    except Exception as e:
        result.fail("UserLeft for nonexistent user → no crash", str(e))

    test_connection_status_lines(result)


def test_connection_status_lines(result):
    """Panel connection status text is stable for evicted and reconnecting states."""
    state, _ = reset_state()

    state.connected = False
    state.reconnecting = False
    state.evicted = False
    lines = _connection_status_lines(state)
    if lines == []:
        result.ok("panel status: disconnected non-evicted shows no warning")
    else:
        result.fail("panel status: disconnected non-evicted shows no warning", str(lines))

    state.connected = False
    state.reconnecting = False
    state.evicted = True
    lines = _connection_status_lines(state)
    expected = [("Connection closed: client fell behind", 'ERROR')]
    if lines == expected:
        result.ok("panel status: evicted disconnected warning text")
    else:
        result.fail("panel status: evicted disconnected warning text", str(lines))

    state.connected = False
    state.reconnecting = True
    state.reconnect_attempt = 2
    state.evicted = True
    lines = _connection_status_lines(state)
    expected = [
        ("Disconnected by server (lag)", 'ERROR'),
        ("Reconnecting (2/3)...", 'TIME'),
    ]
    if lines == expected:
        result.ok("panel status: evicted reconnecting text")
    else:
        result.fail("panel status: evicted reconnecting text", str(lines))
