"""Tests for object deletion sync — send and receive."""
import bpy
from blender_plugin.state import PluginState
from blender_plugin.event_handlers import (
    detect_and_send_deletions,
    handle_object_deleted,
)
from blender_plugin.tests.helpers import (
    reset_state, clear_scene, create_tagged_cube, TestResult,
)


def run(result):
    print("\n--- Deletion Sync Tests ---")

    # ── Deleting an object in Blender sends DeleteObject ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-del-001")

    # Delete the object from Blender
    bpy.data.objects.remove(obj, do_unlink=True)

    detect_and_send_deletions()
    sent = mock_ws.get_sent("DeleteObject")
    if len(sent) == 1:
        result.ok("deleted object → sends DeleteObject")
    else:
        result.fail("deleted object → sends DeleteObject", f"sent {len(sent)}")

    if sent and sent[0]["payload"]["object_id"] == "cube-del-001":
        result.ok("DeleteObject payload has correct object_id")
    else:
        result.fail("DeleteObject payload has correct object_id")

    # ── Deleted object removed from object_map ──

    if "cube-del-001" not in state.object_map:
        result.ok("deleted object removed from object_map")
    else:
        result.fail("deleted object removed from object_map")

    # ── Second call doesn't re-send ──

    mock_ws.clear()
    detect_and_send_deletions()
    sent = mock_ws.get_sent("DeleteObject")
    if len(sent) == 0:
        result.ok("already deleted → no re-send")
    else:
        result.fail("already deleted → no re-send", f"sent {len(sent)}")

    # ── Existing objects are NOT reported as deleted ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-alive-001")

    detect_and_send_deletions()
    sent = mock_ws.get_sent("DeleteObject")
    if len(sent) == 0:
        result.ok("existing object → no DeleteObject")
    else:
        result.fail("existing object → no DeleteObject", f"sent {len(sent)}")

    # ── Multiple deletions at once ──

    clear_scene()
    state, mock_ws = reset_state()
    obj1 = create_tagged_cube("cube-multi-001")
    obj2 = create_tagged_cube("cube-multi-002")
    obj3 = create_tagged_cube("cube-multi-003")

    bpy.data.objects.remove(obj1, do_unlink=True)
    bpy.data.objects.remove(obj3, do_unlink=True)

    detect_and_send_deletions()
    sent = mock_ws.get_sent("DeleteObject")
    if len(sent) == 2:
        result.ok("two deletions → two DeleteObject messages")
    else:
        result.fail("two deletions → two DeleteObject messages", f"sent {len(sent)}")

    deleted_ids = {m["payload"]["object_id"] for m in sent}
    if deleted_ids == {"cube-multi-001", "cube-multi-003"}:
        result.ok("correct object_ids in DeleteObject messages")
    else:
        result.fail("correct object_ids in DeleteObject messages", f"got {deleted_ids}")

    # obj2 should still be tracked
    if "cube-multi-002" in state.object_map:
        result.ok("surviving object still in object_map")
    else:
        result.fail("surviving object still in object_map")

    # ── Receive handler deletes object from scene ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-recv-del-001")

    handle_object_deleted({
        "object_id": "cube-recv-del-001",
        "deleted_by": "other-user-456",
    })

    found = any(o.get("meerkat_id") == "cube-recv-del-001" for o in bpy.data.objects)
    if not found:
        result.ok("receive handler removes object from scene")
    else:
        result.fail("receive handler removes object from scene")

    if "cube-recv-del-001" not in state.object_map:
        result.ok("receive handler removes object from object_map")
    else:
        result.fail("receive handler removes object from object_map")

    # ── Receive delete for nonexistent object is no-op ──

    try:
        handle_object_deleted({
            "object_id": "ghost-object",
            "deleted_by": "other-user-456",
        })
        result.ok("delete nonexistent object → no crash")
    except Exception as e:
        result.fail("delete nonexistent object → no crash", str(e))

    # ── Deletion not detected during remote update ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-echo-del-001")
    bpy.data.objects.remove(obj, do_unlink=True)

    state.is_applying_remote_update = True
    mock_ws.clear()
    # detect_and_send_deletions is guarded by is_applying_remote_update in timer_function
    # but the function itself doesn't check — the guard is in timer_function
    # So we test that the timer_function guard works
    from blender_plugin.event_handlers import timer_function
    timer_function()
    sent = mock_ws.get_sent("DeleteObject")
    if len(sent) == 0:
        result.ok("is_applying_remote_update → no deletion detection")
    else:
        result.fail("is_applying_remote_update → no deletion detection", f"sent {len(sent)}")
    state.is_applying_remote_update = False
