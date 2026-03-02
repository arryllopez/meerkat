"""Tests for name sync — polling, sending, receiving."""
import bpy
from blender_plugin.state import PluginState
from blender_plugin.event_handlers import (
    timer_function_transforms,
    handle_name_updated,
)
from blender_plugin.tests.helpers import (
    reset_state, clear_scene, create_tagged_cube, TestResult,
)


def run(result):
    print("\n--- Name Sync Tests ---")

    # ── First tick populates name cache without sending ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-name-001")
    obj.name = "MyCube"

    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateName")
    if len(sent) == 0:
        result.ok("first tick → cache populated, no UpdateName sent")
    else:
        result.fail("first tick → cache populated, no UpdateName sent", f"sent {len(sent)}")

    if state.name_cache.get("cube-name-001") == "MyCube":
        result.ok("name cache has correct initial value")
    else:
        result.fail("name cache has correct initial value",
                     f"got {state.name_cache.get('cube-name-001')}")

    # ── Rename sends UpdateName ──

    mock_ws.clear()
    obj.name = "RenamedCube"
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateName")
    if len(sent) == 1:
        result.ok("rename → sends UpdateName")
    else:
        result.fail("rename → sends UpdateName", f"sent {len(sent)}")

    if sent and sent[0]["payload"]["name"] == "RenamedCube":
        result.ok("UpdateName payload has correct name")
    else:
        result.fail("UpdateName payload has correct name")

    # ── No re-send when name unchanged ──

    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateName")
    if len(sent) == 0:
        result.ok("name unchanged → no re-send")
    else:
        result.fail("name unchanged → no re-send", f"sent {len(sent)}")

    # ── Receive handler applies name ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-name-002")

    handle_name_updated({
        "object_id": "cube-name-002",
        "updated_by": "other-user-456",
        "name": "RemotelyRenamed",
    })

    if obj.name == "RemotelyRenamed":
        result.ok("receive handler applies name")
    else:
        result.fail("receive handler applies name", f"got {obj.name}")

    # ── After receive, cache updated (no re-send) ──

    # Populate initial name cache first
    state.name_cache["cube-name-002"] = obj.name
    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateName")
    if len(sent) == 0:
        result.ok("after name receive → no re-send")
    else:
        result.fail("after name receive → no re-send", f"sent {len(sent)}")

    # ── Blender name collision — obj.name may differ from requested ──

    clear_scene()
    state, mock_ws = reset_state()
    obj1 = create_tagged_cube("cube-dup-001")
    obj1.name = "Cube"
    obj2 = create_tagged_cube("cube-dup-002")
    obj2.name = "Cube"  # Blender auto-renames to "Cube.001"

    handle_name_updated({
        "object_id": "cube-dup-002",
        "updated_by": "other-user-456",
        "name": "Cube",
    })

    # Cache should store what Blender actually set, not what was requested
    cached = state.name_cache.get("cube-dup-002")
    if cached == obj2.name:
        result.ok("name cache stores Blender's actual name (handles collisions)")
    else:
        result.fail("name cache stores Blender's actual name",
                     f"cached={cached}, obj.name={obj2.name}")

    # ── Receive handler for nonexistent object is no-op ──

    try:
        handle_name_updated({
            "object_id": "nonexistent-id",
            "updated_by": "other-user-456",
            "name": "Ghost",
        })
        result.ok("name update for nonexistent object → no crash")
    except Exception as e:
        result.fail("name update for nonexistent object → no crash", str(e))
