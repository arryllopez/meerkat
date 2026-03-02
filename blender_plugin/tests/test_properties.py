"""Tests for property sync — camera, point light, sun light polling and receiving."""
import bpy
from blender_plugin.state import PluginState
from blender_plugin.event_handlers import (
    _build_camera_props,
    _build_point_light_props,
    _build_sun_light_props,
    _get_property_builder,
    timer_function_transforms,
    handle_properties_updated,
)
from blender_plugin.tests.helpers import (
    reset_state, clear_scene,
    create_tagged_cube, create_tagged_camera,
    create_tagged_point_light, create_tagged_sun_light,
    TestResult,
)


def run(result):
    print("\n--- Property Sync Tests ---")

    # ── Builder returns correct format for camera ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_camera("cam-prop-001")
    props = _build_camera_props(obj)
    if "Camera" in props and "focal_length" in props["Camera"]:
        result.ok("camera builder returns correct format")
    else:
        result.fail("camera builder returns correct format", f"got {props}")

    if props["Camera"]["focal_length"] == obj.data.lens:
        result.ok("camera builder reads actual focal_length")
    else:
        result.fail("camera builder reads actual focal_length",
                     f"expected {obj.data.lens}, got {props['Camera']['focal_length']}")

    # ── Builder returns correct format for point light ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_point_light("pl-prop-001")
    props = _build_point_light_props(obj)
    if "PointLight" in props and "power" in props["PointLight"]:
        result.ok("point light builder returns correct format")
    else:
        result.fail("point light builder returns correct format", f"got {props}")

    if props["PointLight"]["color"] == list(obj.data.color):
        result.ok("point light builder reads actual color")
    else:
        result.fail("point light builder reads actual color")

    # ── Builder returns correct format for sun light ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_sun_light("sl-prop-001")
    props = _build_sun_light_props(obj)
    if "SunLight" in props and "strength" in props["SunLight"]:
        result.ok("sun light builder returns correct format")
    else:
        result.fail("sun light builder returns correct format", f"got {props}")

    # ── _get_property_builder returns correct builder per type ──

    clear_scene()
    state, mock_ws = reset_state()

    cam = create_tagged_camera("cam-gp-001")
    if _get_property_builder(cam) == _build_camera_props:
        result.ok("_get_property_builder → camera")
    else:
        result.fail("_get_property_builder → camera")

    pl = create_tagged_point_light("pl-gp-001")
    if _get_property_builder(pl) == _build_point_light_props:
        result.ok("_get_property_builder → point light")
    else:
        result.fail("_get_property_builder → point light")

    sl = create_tagged_sun_light("sl-gp-001")
    if _get_property_builder(sl) == _build_sun_light_props:
        result.ok("_get_property_builder → sun light")
    else:
        result.fail("_get_property_builder → sun light")

    cube = create_tagged_cube("cube-gp-001")
    if _get_property_builder(cube) is None:
        result.ok("_get_property_builder → None for cube")
    else:
        result.fail("_get_property_builder → None for cube")

    # ── Camera property change sends UpdateProperties ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_camera("cam-poll-001")

    # First tick populates cache
    timer_function_transforms()
    mock_ws.clear()

    # Change focal length
    obj.data.lens = 85.0
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateProperties")
    if len(sent) == 1:
        result.ok("camera focal_length change → sends UpdateProperties")
    else:
        result.fail("camera focal_length change → sends UpdateProperties", f"sent {len(sent)}")

    if sent and sent[0]["payload"]["properties"]["Camera"]["focal_length"] == 85.0:
        result.ok("UpdateProperties payload has correct focal_length")
    else:
        result.fail("UpdateProperties payload has correct focal_length")

    # ── No re-send when camera props unchanged ──

    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateProperties")
    if len(sent) == 0:
        result.ok("camera props unchanged → no re-send")
    else:
        result.fail("camera props unchanged → no re-send", f"sent {len(sent)}")

    # ── Point light color change sends UpdateProperties ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_point_light("pl-poll-001")

    timer_function_transforms()
    mock_ws.clear()

    obj.data.color = (1.0, 0.0, 0.0)
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateProperties")
    if len(sent) == 1:
        result.ok("light color change → sends UpdateProperties")
    else:
        result.fail("light color change → sends UpdateProperties", f"sent {len(sent)}")

    # ── Point light power change sends UpdateProperties ──

    mock_ws.clear()
    obj.data.energy = 500.0
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateProperties")
    if len(sent) == 1:
        result.ok("light power change → sends UpdateProperties")
    else:
        result.fail("light power change → sends UpdateProperties", f"sent {len(sent)}")

    # ── Cube has no properties to sync ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_cube("cube-noprop-001")

    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateProperties")
    if len(sent) == 0:
        result.ok("cube → no UpdateProperties sent")
    else:
        result.fail("cube → no UpdateProperties sent", f"sent {len(sent)}")

    # ── Receive handler applies camera properties ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_camera("cam-recv-001")

    handle_properties_updated({
        "object_id": "cam-recv-001",
        "updated_by": "other-user-456",
        "properties": {
            "Camera": {
                "focal_length": 135.0,
                "clip_start": 0.5,
                "clip_end": 500.0,
            }
        }
    })

    if abs(obj.data.lens - 135.0) < 0.001:
        result.ok("receive handler applies camera focal_length")
    else:
        result.fail("receive handler applies camera focal_length", f"got {obj.data.lens}")

    if abs(obj.data.clip_start - 0.5) < 0.001:
        result.ok("receive handler applies camera clip_start")
    else:
        result.fail("receive handler applies camera clip_start", f"got {obj.data.clip_start}")

    # ── Receive handler applies point light properties ──

    clear_scene()
    state, mock_ws = reset_state()
    obj = create_tagged_point_light("pl-recv-001")

    handle_properties_updated({
        "object_id": "pl-recv-001",
        "updated_by": "other-user-456",
        "properties": {
            "PointLight": {
                "color": [0.0, 1.0, 0.0],
                "power": 250.0,
                "radius": 3.0,
            }
        }
    })

    if abs(obj.data.color[1] - 1.0) < 0.001:
        result.ok("receive handler applies point light color")
    else:
        result.fail("receive handler applies point light color", f"got {list(obj.data.color)}")

    if abs(obj.data.energy - 250.0) < 0.001:
        result.ok("receive handler applies point light power")
    else:
        result.fail("receive handler applies point light power", f"got {obj.data.energy}")

    # ── After receive, cache is updated (no re-send) ──

    mock_ws.clear()
    timer_function_transforms()
    sent = mock_ws.get_sent("UpdateProperties")
    if len(sent) == 0:
        result.ok("after property receive → cache updated, no re-send")
    else:
        result.fail("after property receive → cache updated, no re-send", f"sent {len(sent)}")
