"""Tests for FullStateSync — scene reconstruction, user_id extraction, clearing old objects."""
import bpy
from blender_plugin.state import PluginState
from blender_plugin.event_handlers import handle_full_state_sync
from blender_plugin.tests.helpers import (
    reset_state, clear_scene, create_tagged_cube, TestResult,
)


def run(result):
    print("\n--- Full State Sync Tests ---")

    # ── FullStateSync creates objects from snapshot ──

    clear_scene()
    state, mock_ws = reset_state()

    handle_full_state_sync({
        "session": {
            "objects": {
                "obj-001": {
                    "object_type": "Cube",
                    "name": "SyncCube",
                    "transform": {"position": [1, 2, 3], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
                    "properties": None,
                },
                "obj-002": {
                    "object_type": "Camera",
                    "name": "SyncCam",
                    "transform": {"position": [5, 5, 5], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
                    "properties": {
                        "Camera": {
                            "focal_length": 50.0,
                            "clip_start": 0.1,
                            "clip_end": 1000.0,
                        }
                    },
                },
            },
            "users": {
                "user-aaa": {
                    "display_name": "TestUser",
                    "color": [255, 0, 0],
                    "selected_object": None,
                },
                "user-bbb": {
                    "display_name": "OtherUser",
                    "color": [0, 255, 0],
                    "selected_object": None,
                },
            },
        }
    })

    if "obj-001" in state.object_map and "obj-002" in state.object_map:
        result.ok("FullStateSync creates all objects")
    else:
        result.fail("FullStateSync creates all objects", f"map keys: {list(state.object_map.keys())}")

    if len(bpy.data.objects) == 2:
        result.ok("scene has exactly 2 objects")
    else:
        result.fail("scene has exactly 2 objects", f"has {len(bpy.data.objects)}")

    # ── Objects have correct transforms ──

    cube = state.object_map.get("obj-001")
    if cube and abs(cube.location.x - 1.0) < 0.001:
        result.ok("cube has correct position from sync")
    else:
        result.fail("cube has correct position from sync")

    # ── Camera has correct properties ──

    cam = state.object_map.get("obj-002")
    if cam and abs(cam.data.lens - 50.0) < 0.001:
        result.ok("camera has correct focal_length from sync")
    else:
        result.fail("camera has correct focal_length from sync")

    # ── User_id extracted by matching display_name ──

    if state.user_id == "user-aaa":
        result.ok("user_id extracted correctly from FullStateSync")
    else:
        result.fail("user_id extracted correctly", f"got {state.user_id}")

    # ── Users populated ──

    if len(state.users) == 2:
        result.ok("users dict populated with 2 users")
    else:
        result.fail("users dict populated", f"has {len(state.users)} users")

    # ── FullStateSync clears old meerkat objects ──

    # Add a pre-existing meerkat object
    old_obj = create_tagged_cube("old-obj-999")
    old_count = len(bpy.data.objects)

    handle_full_state_sync({
        "session": {
            "objects": {
                "fresh-001": {
                    "object_type": "Sphere",
                    "name": "FreshSphere",
                    "transform": {"position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
                    "properties": None,
                },
            },
            "users": {
                "user-aaa": {
                    "display_name": "TestUser",
                    "color": [255, 0, 0],
                    "selected_object": None,
                },
            },
        }
    })

    # Old objects should be gone, only the fresh sphere remains
    if len(bpy.data.objects) == 1:
        result.ok("FullStateSync clears old meerkat objects")
    else:
        result.fail("FullStateSync clears old meerkat objects", f"scene has {len(bpy.data.objects)} objects")

    if "old-obj-999" not in state.object_map:
        result.ok("old object removed from object_map")
    else:
        result.fail("old object removed from object_map")

    if "fresh-001" in state.object_map:
        result.ok("fresh object in object_map")
    else:
        result.fail("fresh object in object_map")

    # ── FullStateSync with empty session ──

    handle_full_state_sync({
        "session": {
            "objects": {},
            "users": {
                "user-aaa": {
                    "display_name": "TestUser",
                    "color": [255, 0, 0],
                    "selected_object": None,
                },
            },
        }
    })

    if len(bpy.data.objects) == 0:
        result.ok("empty FullStateSync → empty scene")
    else:
        result.fail("empty FullStateSync → empty scene", f"has {len(bpy.data.objects)} objects")

    if len(state.object_map) == 0:
        result.ok("empty FullStateSync → empty object_map")
    else:
        result.fail("empty FullStateSync → empty object_map")

    # ── FullStateSync creates all supported object types ──

    clear_scene()
    state, mock_ws = reset_state()

    handle_full_state_sync({
        "session": {
            "objects": {
                "t-cube": {"object_type": "Cube", "name": "C", "transform": {"position": [0,0,0], "rotation": [0,0,0], "scale": [1,1,1]}, "properties": None},
                "t-sphere": {"object_type": "Sphere", "name": "S", "transform": {"position": [1,0,0], "rotation": [0,0,0], "scale": [1,1,1]}, "properties": None},
                "t-cyl": {"object_type": "Cylinder", "name": "Cy", "transform": {"position": [2,0,0], "rotation": [0,0,0], "scale": [1,1,1]}, "properties": None},
                "t-cam": {"object_type": "Camera", "name": "Ca", "transform": {"position": [3,0,0], "rotation": [0,0,0], "scale": [1,1,1]}, "properties": None},
                "t-pl": {"object_type": "PointLight", "name": "PL", "transform": {"position": [4,0,0], "rotation": [0,0,0], "scale": [1,1,1]}, "properties": None},
                "t-sl": {"object_type": "SunLight", "name": "SL", "transform": {"position": [5,0,0], "rotation": [0,0,0], "scale": [1,1,1]}, "properties": None},
            },
            "users": {
                "user-aaa": {"display_name": "TestUser", "color": [255,0,0], "selected_object": None},
            },
        }
    })

    if len(state.object_map) == 6:
        result.ok("FullStateSync creates all 6 object types")
    else:
        result.fail("FullStateSync creates all 6 object types", f"created {len(state.object_map)}")
