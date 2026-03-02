"""Test helpers — scene setup, state reset, assertion utilities."""
import bpy
import sys
import os

# Add parent dir to path so we can import the plugin modules
plugin_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if plugin_dir not in sys.path:
    sys.path.insert(0, os.path.dirname(plugin_dir))

from blender_plugin.state import PluginState
from blender_plugin.tests.mock_ws import MockWebSocketClient


def reset_state():
    """Clear plugin state and all caches. Call before each test."""
    state = PluginState()
    state.connected = True
    state.user_id = "test-user-123"
    state.display_name = "TestUser"
    state.session_id = "test-session"
    state.is_applying_remote_update = False
    state.object_map.clear()
    state.users.clear()
    state.transform_cache.clear()
    state.property_cache.clear()
    state.name_cache.clear()
    state.asset_library_objects.clear()

    mock_ws = MockWebSocketClient()
    state.ws_client = mock_ws
    return state, mock_ws


def clear_scene():
    """Remove all objects from the Blender scene and clear stale references."""
    state = PluginState()
    state.object_map.clear()
    state.transform_cache.clear()
    state.property_cache.clear()
    state.name_cache.clear()
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def create_tagged_cube(meerkat_id="cube-001"):
    """Create a cube with a meerkat_id and register it in state."""
    bpy.ops.mesh.primitive_cube_add()
    obj = bpy.context.active_object
    obj["meerkat_id"] = meerkat_id
    state = PluginState()
    state.object_map[meerkat_id] = obj
    return obj


def create_tagged_camera(meerkat_id="cam-001"):
    """Create a camera with a meerkat_id and register it in state."""
    bpy.ops.object.camera_add()
    obj = bpy.context.active_object
    obj["meerkat_id"] = meerkat_id
    state = PluginState()
    state.object_map[meerkat_id] = obj
    return obj


def create_tagged_point_light(meerkat_id="plight-001"):
    """Create a point light with a meerkat_id and register it in state."""
    bpy.ops.object.light_add(type='POINT')
    obj = bpy.context.active_object
    obj["meerkat_id"] = meerkat_id
    state = PluginState()
    state.object_map[meerkat_id] = obj
    return obj


def create_tagged_sun_light(meerkat_id="slight-001"):
    """Create a sun light with a meerkat_id and register it in state."""
    bpy.ops.object.light_add(type='SUN')
    obj = bpy.context.active_object
    obj["meerkat_id"] = meerkat_id
    state = PluginState()
    state.object_map[meerkat_id] = obj
    return obj


class TestResult:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []

    def ok(self, name):
        self.passed += 1
        print(f"  PASS  {name}")

    def fail(self, name, reason=""):
        self.failed += 1
        self.errors.append((name, reason))
        print(f"  FAIL  {name} — {reason}")

    def summary(self):
        total = self.passed + self.failed
        print(f"\n{'='*60}")
        print(f"Results: {self.passed}/{total} passed, {self.failed} failed")
        if self.errors:
            print("\nFailures:")
            for name, reason in self.errors:
                print(f"  - {name}: {reason}")
        print(f"{'='*60}")
        return self.failed == 0
