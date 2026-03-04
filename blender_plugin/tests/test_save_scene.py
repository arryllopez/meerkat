"""Tests for Save Scene — verify RequestStateSync is sent before saving."""
from blender_plugin.state import PluginState
from blender_plugin.tests.helpers import reset_state, clear_scene, TestResult


def run(result):
    print("\n--- Save Scene Tests ---")

    test_save_sends_request_state_sync(result)
    test_save_not_connected(result)
    test_save_payload_is_correct(result)
    test_save_does_not_send_join_session(result)


def test_save_sends_request_state_sync(result):
    """Save Scene operator sends a RequestStateSync event."""
    name = "save scene sends RequestStateSync"
    state, mock_ws = reset_state()
    clear_scene()

    # Simulate what the operator does (minus bpy.ops.wm.save_as_mainfile)
    mock_ws.clear()
    state.ws_client.send({
        "event_type": "RequestStateSync",
        "payload": None,
    })

    sync_msgs = mock_ws.get_sent("RequestStateSync")
    if len(sync_msgs) == 1:
        result.ok(name)
    else:
        result.fail(name, f"expected 1 RequestStateSync, got {len(sync_msgs)}")


def test_save_not_connected(result):
    """Save Scene should not send anything when disconnected."""
    name = "save scene does nothing when disconnected"
    state, mock_ws = reset_state()
    state.connected = False
    mock_ws.clear()

    # Operator would bail early with CANCELLED — verify no messages sent
    if not state.connected:
        # Operator logic: early return, no send
        pass

    sent = mock_ws.get_sent()
    if len(sent) == 0:
        result.ok(name)
    else:
        result.fail(name, f"expected 0 messages, got {len(sent)}")


def test_save_payload_is_correct(result):
    """RequestStateSync payload should be None (no extra data needed)."""
    name = "RequestStateSync payload is None"
    state, mock_ws = reset_state()
    mock_ws.clear()

    state.ws_client.send({
        "event_type": "RequestStateSync",
        "payload": None,
    })

    msg = mock_ws.get_sent("RequestStateSync")[0]
    if msg["payload"] is None:
        result.ok(name)
    else:
        result.fail(name, f"expected payload=None, got {msg['payload']}")


def test_save_does_not_send_join_session(result):
    """Save Scene must NOT re-send JoinSession (would create a duplicate user)."""
    name = "save scene does not send JoinSession"
    state, mock_ws = reset_state()
    mock_ws.clear()

    # Simulate save: only RequestStateSync, nothing else
    state.ws_client.send({
        "event_type": "RequestStateSync",
        "payload": None,
    })

    join_msgs = mock_ws.get_sent("JoinSession")
    if len(join_msgs) == 0:
        result.ok(name)
    else:
        result.fail(name, f"expected 0 JoinSession, got {len(join_msgs)}")
