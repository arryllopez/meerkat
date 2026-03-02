"""Tests for transform sync — polling, sending, receiving, caching."""
import bpy
from blender_plugin.state import PluginState
from blender_plugin.event_handlers import (
    _transforms_changed,
    timer_function_transforms,
    handle_transform_updated,
)
from blender_plugin.utils import build_transform
from blender_plugin.tests.helpers import (
    reset_state, clear_scene, create_tagged_cube, TestResult,
)


def run(result):
    print("\n--- Transform Sync Tests ---")

    # ── _transforms_changed logic ──

    # Identical transforms should return False
    clear_scene()
    state, mock_ws = reset_state()
    t = {"position": [1.0, 2.0, 3.0], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}
    if not _transforms_changed(t, t):
        result.ok("identical transforms → no change")
    else:
        result.fail("identical transforms → no change", "returned True")

    # Different position should return True
    t2 = {"position": [1.0, 2.5, 3.0], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}
    if _transforms_changed(t, t2):
        result.ok("different position → change detected")
    else:
        result.fail("different position → change detected", "returned False")

    # Difference within epsilon should return False
    t3 = {"position": [1.0, 2.0, 3.000001], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}
    if not _transforms_changed(t, t3):
        result.ok("within epsilon → no change")
    else:
        result.fail("within epsilon → no change", "returned True")

    # Different rotation should return True
    t4 = {"position": [1.0, 2.0, 3.0], "rotation": [0.5, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}
    if _transforms_changed(t, t4):
        result.ok("different rotation → change detected")
    else:
        result.fail("different rotation → change detected", "returned False")

    # Different scale should return True
    t5 = {"position": [1.0, 2.0, 3.0], "rotation": [0.0, 0.0, 0.0], "scale": [2.0, 1.0, 1.0]}
    if _transforms_changed(t, t5):
        result.ok("different scale → change detected")
    else:
        result.fail("different scale → change detected", "returned False")

    # ── Transform polling sends UpdateTransform ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-tf-001")
    obj.location = (3.0, 4.0, 5.0)

    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateTransform")
    if len(sent) == 1:
        result.ok("moved cube → sends UpdateTransform")
    else:
        result.fail("moved cube → sends UpdateTransform", f"sent {len(sent)} messages")

    payload = sent[0]["payload"]
    if abs(payload["transform"]["position"][0] - 3.0) < 0.001:
        result.ok("UpdateTransform payload has correct position")
    else:
        result.fail("UpdateTransform payload has correct position", f"got {payload['transform']['position']}")

    # ── No re-send when transform unchanged ──

    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateTransform")
    if len(sent) == 0:
        result.ok("unchanged transform → no re-send")
    else:
        result.fail("unchanged transform → no re-send", f"sent {len(sent)} messages")

    # ── Re-send when transform changes again ──

    obj.location = (10.0, 0.0, 0.0)
    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateTransform")
    if len(sent) == 1:
        result.ok("moved again → sends new UpdateTransform")
    else:
        result.fail("moved again → sends new UpdateTransform", f"sent {len(sent)} messages")

    # ── Receive handler applies transform ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-tf-002")

    handle_transform_updated({
        "object_id": "cube-tf-002",
        "updated_by": "other-user-456",
        "transform": {
            "position": [7.0, 8.0, 9.0],
            "rotation": [0.1, 0.2, 0.3],
            "scale": [2.0, 2.0, 2.0],
        }
    })

    if abs(obj.location.x - 7.0) < 0.001 and abs(obj.location.y - 8.0) < 0.001:
        result.ok("receive handler applies position")
    else:
        result.fail("receive handler applies position", f"got {list(obj.location)}")

    if abs(obj.scale.x - 2.0) < 0.001:
        result.ok("receive handler applies scale")
    else:
        result.fail("receive handler applies scale", f"got {list(obj.scale)}")

    # ── Receive handler updates cache (prevents re-send) ──

    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateTransform")
    if len(sent) == 0:
        result.ok("after receive → cache updated, no re-send")
    else:
        result.fail("after receive → cache updated, no re-send", f"sent {len(sent)}")

    # ── Receive handler for nonexistent object is no-op ──

    try:
        handle_transform_updated({
            "object_id": "nonexistent-id",
            "updated_by": "other-user-456",
            "transform": {"position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        })
        result.ok("transform for nonexistent object → no crash")
    except Exception as e:
        result.fail("transform for nonexistent object → no crash", str(e))
