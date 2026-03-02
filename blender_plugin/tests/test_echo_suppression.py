"""Tests for echo suppression — all receive handlers should ignore events from self."""
import bpy
from blender_plugin.state import PluginState
from blender_plugin.event_handlers import (
    handle_object_created,
    handle_object_deleted,
    handle_transform_updated,
    handle_properties_updated,
    handle_name_updated,
)
from blender_plugin.tests.helpers import (
    reset_state, clear_scene, create_tagged_cube, create_tagged_camera, TestResult,
)


def run(result):
    print("\n--- Echo Suppression Tests ---")

    # ── ObjectCreated from self is ignored ──

    clear_scene()
    state, mock_ws = reset_state()
    initial_count = len(bpy.data.objects)

    handle_object_created({
        "object": {
            "object_id": "echo-cube-001",
            "object_type": "Cube",
            "name": "EchoCube",
            "transform": {"position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
            "properties": None,
        },
        "created_by": "test-user-123",  # same as state.user_id
    })

    if len(bpy.data.objects) == initial_count:
        result.ok("ObjectCreated from self → ignored (no new object)")
    else:
        result.fail("ObjectCreated from self → ignored", f"objects went from {initial_count} to {len(bpy.data.objects)}")

    # ── ObjectCreated from another user works ──

    handle_object_created({
        "object": {
            "object_id": "other-cube-001",
            "object_type": "Cube",
            "name": "OtherCube",
            "transform": {"position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
            "properties": None,
        },
        "created_by": "other-user-456",
    })

    if len(bpy.data.objects) == initial_count + 1:
        result.ok("ObjectCreated from other user → object created")
    else:
        result.fail("ObjectCreated from other user → object created")

    # ── ObjectDeleted from self is ignored ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("echo-del-001")
    initial_count = len(bpy.data.objects)

    handle_object_deleted({
        "object_id": "echo-del-001",
        "deleted_by": "test-user-123",  # same as state.user_id
    })

    if len(bpy.data.objects) == initial_count:
        result.ok("ObjectDeleted from self → ignored (object still exists)")
    else:
        result.fail("ObjectDeleted from self → ignored")

    # ── TransformUpdated from self is ignored ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("echo-tf-001")
    obj.location = (1.0, 2.0, 3.0)
    original_x = obj.location.x

    handle_transform_updated({
        "object_id": "echo-tf-001",
        "updated_by": "test-user-123",  # same as state.user_id
        "transform": {"position": [99.0, 99.0, 99.0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
    })

    if abs(obj.location.x - original_x) < 0.001:
        result.ok("TransformUpdated from self → ignored (position unchanged)")
    else:
        result.fail("TransformUpdated from self → ignored", f"x moved to {obj.location.x}")

    # ── PropertiesUpdated from self is ignored ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_camera("echo-cam-001")
    original_lens = obj.data.lens

    handle_properties_updated({
        "object_id": "echo-cam-001",
        "updated_by": "test-user-123",  # same as state.user_id
        "properties": {"Camera": {"focal_length": 999.0}},
    })

    if abs(obj.data.lens - original_lens) < 0.001:
        result.ok("PropertiesUpdated from self → ignored (focal_length unchanged)")
    else:
        result.fail("PropertiesUpdated from self → ignored", f"lens changed to {obj.data.lens}")

    # ── NameUpdated from self is ignored ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("echo-name-001")
    obj.name = "OriginalName"

    handle_name_updated({
        "object_id": "echo-name-001",
        "updated_by": "test-user-123",  # same as state.user_id
        "name": "ShouldNotApply",
    })

    if obj.name == "OriginalName":
        result.ok("NameUpdated from self → ignored (name unchanged)")
    else:
        result.fail("NameUpdated from self → ignored", f"name changed to {obj.name}")
