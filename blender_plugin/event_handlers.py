# event_handlers.py — server event -> Blender action dispatch
import bpy
import time
import queue
import traceback
from uuid import uuid4
from .state import PluginState
from .utils import build_transform

# helper for redrawing panel 
def _redraw_panels():
    for window in bpy.context.window_manager.windows:
        for area in window.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()



def _transforms_changed(current, cached):
    EPSILON = 1e-5
    for key in ("position", "rotation", "scale"):
        for a, b in zip(current[key], cached[key]):
            if abs(a - b) > EPSILON:
                return True
    return False


def _verbose_logging_enabled():
    try:
        addon = bpy.context.preferences.addons.get(__package__)
        if addon is None:
            return False
        return bool(getattr(addon.preferences, "verbose_logging", False))
    except Exception:
        return False


def _log_incoming_event(event_type, payload, verbose_logging):
    if not verbose_logging:
        return
    print(f"[Meerkat] <<< {event_type}: {payload}")


# ── Property builders: read current values from Blender objects ──────────────

def _build_camera_props(obj):
    cam = obj.data
    return {"Camera": {
        "lens_type": "Orthographic" if cam.type == "ORTHO" else "Perspective",
        "focal_length": cam.lens,
        "orthographic_scale": cam.ortho_scale,
        "shift_x": cam.shift_x,
        "shift_y": cam.shift_y,
        "clip_start": cam.clip_start,
        "clip_end": cam.clip_end,
        "focal_distance": cam.dof.focus_distance,
        "aperture_fstop": cam.dof.aperture_fstop,
        "aperture_blades": cam.dof.aperture_blades,
        "aperture_rotation": cam.dof.aperture_rotation,
        "aperture_ratio": cam.dof.aperture_ratio,
        "sensor_fit": cam.sensor_fit,
        "sensor_width": cam.sensor_width,
        "sensor_height": cam.sensor_height,
    }}


# temperature/exposure/normalize landed in Blender 4.2; getattr keeps 4.0/4.1 hosts working.
def _build_point_light_props(obj):
    light = obj.data
    return {"PointLight": {
        "color": list(light.color),
        "use_temperature": getattr(light, "use_temperature", False),
        "temperature": getattr(light, "temperature", 6500.0),
        "exposure": getattr(light, "exposure", 0.0),
        "power": light.energy,
        "radius": light.shadow_soft_size,
        "soft_falloff": getattr(light, "use_soft_falloff", False),
        "normalize": getattr(light, "normalize", False),
        "cast_shadow": getattr(light, "use_shadow", True),
        "shadow_jitter": getattr(light, "use_shadow_jitter", False),
        "shadow_jitter_overblur": getattr(light, "shadow_jitter_overblur", 0.0),
        "shadow_filter_radius": getattr(light, "shadow_filter_radius", 1.0),
        "shadow_maximum_resolution": getattr(light, "shadow_maximum_resolution", 0.001),
        "diffuse_factor": getattr(light, "diffuse_factor", 1.0),
        "specular_factor": getattr(light, "specular_factor", 1.0),
        "transmission_factor": getattr(light, "transmission_factor", 1.0),
        "volume_factor": getattr(light, "volume_factor", 1.0),
        "use_custom_distance": getattr(light, "use_custom_distance", False),
        "cutoff_distance": getattr(light, "cutoff_distance", 40.0),
    }}


def _build_sun_light_props(obj):
    light = obj.data
    return {"SunLight": {
        "color": list(light.color),
        "use_temperature": getattr(light, "use_temperature", False),
        "temperature": getattr(light, "temperature", 6500.0),
        "exposure": getattr(light, "exposure", 0.0),
        "normalize": getattr(light, "normalize", False),
        "strength": light.energy,
        "angle": light.angle,
        "cast_shadow": getattr(light, "use_shadow", True),
        "shadow_jitter": getattr(light, "use_shadow_jitter", False),
        "shadow_jitter_overblur": getattr(light, "shadow_jitter_overblur", 0.0),
        "shadow_filter_radius": getattr(light, "shadow_filter_radius", 1.0),
        "shadow_maximum_resolution": getattr(light, "shadow_maximum_resolution", 0.001),
        "diffuse_factor": getattr(light, "diffuse_factor", 1.0),
        "specular_factor": getattr(light, "specular_factor", 1.0),
        "transmission_factor": getattr(light, "transmission_factor", 1.0),
        "volume_factor": getattr(light, "volume_factor", 1.0),
    }}


def _build_spot_light_props(obj):
    light = obj.data
    return {"SpotLight": {
        "color": list(light.color),
        "use_temperature": getattr(light, "use_temperature", False),
        "temperature": getattr(light, "temperature", 6500.0),
        "exposure": getattr(light, "exposure", 0.0),
        "normalize": getattr(light, "normalize", False),
        "power": light.energy,
        "radius": light.shadow_soft_size,
        "soft_falloff": getattr(light, "use_soft_falloff", False),
        "angle": light.spot_size,
        "blend": light.spot_blend,
        "show_cone": light.show_cone,
        "cast_shadow": getattr(light, "use_shadow", True),
        "shadow_jitter": getattr(light, "use_shadow_jitter", False),
        "shadow_jitter_overblur": getattr(light, "shadow_jitter_overblur", 0.0),
        "shadow_filter_radius": getattr(light, "shadow_filter_radius", 1.0),
        "shadow_maximum_resolution": getattr(light, "shadow_maximum_resolution", 0.001),
        "diffuse_factor": getattr(light, "diffuse_factor", 1.0),
        "specular_factor": getattr(light, "specular_factor", 1.0),
        "transmission_factor": getattr(light, "transmission_factor", 1.0),
        "volume_factor": getattr(light, "volume_factor", 1.0),
        "use_custom_distance": getattr(light, "use_custom_distance", False),
        "cutoff_distance": getattr(light, "cutoff_distance", 40.0),
    }}


def _build_area_light_props(obj):
    light = obj.data
    is_rect = light.shape in ("RECTANGLE", "ELLIPSE")
    return {"AreaLight": {
        "color": list(light.color),
        "use_temperature": getattr(light, "use_temperature", False),
        "temperature": getattr(light, "temperature", 6500.0),
        "exposure": getattr(light, "exposure", 0.0),
        "normalize": getattr(light, "normalize", False),
        "power": light.energy,
        "shape": light.shape,
        "size_x": light.size if is_rect else 0.0,
        "size_y": light.size_y if is_rect else 0.0,
        "size": light.size if not is_rect else 0.0,
        "cast_shadow": getattr(light, "use_shadow", True),
        "shadow_jitter": getattr(light, "use_shadow_jitter", False),
        "shadow_jitter_overblur": getattr(light, "shadow_jitter_overblur", 0.0),
        "shadow_filter_radius": getattr(light, "shadow_filter_radius", 1.0),
        "shadow_maximum_resolution": getattr(light, "shadow_maximum_resolution", 0.001),
        "diffuse_factor": getattr(light, "diffuse_factor", 1.0),
        "specular_factor": getattr(light, "specular_factor", 1.0),
        "transmission_factor": getattr(light, "transmission_factor", 1.0),
        "volume_factor": getattr(light, "volume_factor", 1.0),
        "use_custom_distance": getattr(light, "use_custom_distance", False),
        "cutoff_distance": getattr(light, "cutoff_distance", 40.0),
    }}


# Maps Blender obj.type + obj.data.type to property builder
PROPERTY_BUILDERS = {
    "CAMERA": _build_camera_props,
    "POINT":  _build_point_light_props,
    "SUN":    _build_sun_light_props,
    "SPOT":   _build_spot_light_props,
    "AREA":   _build_area_light_props,
}


def _get_property_builder(obj):
    """Return the property builder for this object, or None if it has no synced properties."""
    if obj.type == "CAMERA":
        return PROPERTY_BUILDERS["CAMERA"]
    if obj.type == "LIGHT":
        return PROPERTY_BUILDERS.get(obj.data.type)
    return None


def timer_function_transforms():
    timer: float = 0.033
    state = PluginState()

    if not state.connected or not state.ws_client:
        return timer
    if state.is_applying_remote_update:
        return timer

    for meerkat_id, obj in state.object_map.items():
        try:
            gone = obj is None or obj.name not in bpy.data.objects
        except ReferenceError:
            continue
        if gone:
            continue

        # --- Transform polling ---
        current = build_transform(obj)
        cached = state.transform_cache.get(meerkat_id)

        if cached is None or _transforms_changed(current, cached):
            state.transform_cache[meerkat_id] = current
            state.ws_client.send({
                "event_type": "UpdateTransform",
                "payload": {
                    "object_id": meerkat_id,
                    "transform": current,
                }
            })

        # --- Property polling ---
        builder = _get_property_builder(obj)
        if builder:
            current_props = builder(obj)
            cached_props = state.property_cache.get(meerkat_id)
            if current_props != cached_props:
                state.property_cache[meerkat_id] = current_props
                state.ws_client.send({
                    "event_type": "UpdateProperties",
                    "payload": {
                        "object_id": meerkat_id,
                        "properties": current_props,
                    }
                })

        # --- Name polling ---
        current_name = obj.name
        cached_name = state.name_cache.get(meerkat_id)
        if cached_name is None:
            state.name_cache[meerkat_id] = current_name
        elif current_name != cached_name:
            state.name_cache[meerkat_id] = current_name
            state.ws_client.send({
                "event_type": "UpdateName",
                "payload": {
                    "object_id": meerkat_id,
                    "name": current_name,
                }
            })

    return timer



def handle_full_state_sync(payload):
    state = PluginState()
    state.connecting = False
    state.connected = True
    state.is_applying_remote_update = True

    try:
        state.user_id = payload.get("your_user_id", "")
        # 1. Delete all existing Meerkat-managed objects from the scene
        objs_to_remove = list(bpy.data.objects)
        for obj in objs_to_remove:
            bpy.data.objects.remove(obj, do_unlink=True)
        state.object_map.clear()
        state.transform_cache.clear() 
        state.property_cache.clear()
        state.name_cache.clear() 
        state.last_selected = None 

        # 2. Recreate objects from the session snapshot
        session = payload.get("session", {})
        objects = session.get("objects", {})
        for obj_id, obj_data in objects.items():
            _create_object_from_snapshot(obj_id, obj_data)

        # 3. Rebuild user list using user ids from session snapshot
        state.users.clear()
        users = session.get("users", {})
        for user_id, user_data in users.items():
            state.users[user_id] = {
                "display_name": user_data.get("display_name", "Unknown"),
                "color": user_data.get("color", [200, 200, 200]),
                "selected_object": user_data.get("selected_object"),
            }
        _redraw_panels()
        print(f"[Meerkat] FullStateSync: {len(objects)} objects, {len(users)} users, my_id={state.user_id}")

    finally:
        state.is_applying_remote_update = False


def _apply_transform(obj, transform):
    """Set position, rotation, scale on a Blender object from a transform dict."""
    pos = transform.get("position", [0, 0, 0])
    rot = transform.get("rotation", [0, 0, 0])
    scl = transform.get("scale", [1, 1, 1])
    obj.location = pos
    obj.rotation_euler = rot
    obj.scale = scl


def _apply_camera_props(obj, p):
    cam = obj.data
    cam.lens = p.get("focal_length", cam.lens)
    cam.ortho_scale = p.get("orthographic_scale", cam.ortho_scale)
    cam.shift_x = p.get("shift_x", cam.shift_x)
    cam.shift_y = p.get("shift_y", cam.shift_y)
    cam.clip_start = p.get("clip_start", cam.clip_start)
    cam.clip_end = p.get("clip_end", cam.clip_end)
    cam.sensor_fit = p.get("sensor_fit", cam.sensor_fit)
    cam.sensor_width = p.get("sensor_width", cam.sensor_width)
    cam.sensor_height = p.get("sensor_height", cam.sensor_height)


# setattr-if-present keeps appliers safe on Blender < 4.2 where these attrs don't exist.
def _set_if_attr(obj, attr, value):
    if hasattr(obj, attr):
        setattr(obj, attr, value)


def _apply_attrs(target, p, mapping):
    """For each (json_key, blender_attr) in mapping, copy p[json_key] → target.attr if present."""
    for json_key, attr in mapping:
        if json_key in p:
            _set_if_attr(target, attr, p[json_key])


CONTRIBUTION_FACTORS = [
    ("diffuse_factor", "diffuse_factor"),
    ("specular_factor", "specular_factor"),
    ("transmission_factor", "transmission_factor"),
    ("volume_factor", "volume_factor"),
]

# Shadow filter + reso are gated only by cast_shadow; jitter has its own sub-gate (overblur).
_SHADOW_FLAT_ATTRS = [
    ("shadow_filter_radius", "shadow_filter_radius"),
    ("shadow_maximum_resolution", "shadow_maximum_resolution"),
]


def _apply_shadow_block(light, p):
    """cast_shadow → (jitter → overblur), filter, resolution_limit. Mirrors Blender's grey-out hierarchy."""
    if "cast_shadow" in p:
        _set_if_attr(light, "use_shadow", p["cast_shadow"])
    if not p.get("cast_shadow", True):
        return
    _apply_attrs(light, p, _SHADOW_FLAT_ATTRS)
    if "shadow_jitter" in p:
        _set_if_attr(light, "use_shadow_jitter", p["shadow_jitter"])
    if p.get("shadow_jitter", False) and "shadow_jitter_overblur" in p:
        _set_if_attr(light, "shadow_jitter_overblur", p["shadow_jitter_overblur"])


def _apply_distance_block(light, p):
    """use_custom_distance → cutoff_distance."""
    if "use_custom_distance" in p:
        _set_if_attr(light, "use_custom_distance", p["use_custom_distance"])
    if p.get("use_custom_distance", False) and "cutoff_distance" in p:
        _set_if_attr(light, "cutoff_distance", p["cutoff_distance"])


def _apply_common_light_props(light, p):
    if "color" in p:
        light.color = p["color"]
    _apply_attrs(light, p, [
        ("exposure", "exposure"),
        ("normalize", "normalize"),
    ])
    # use_temperature gates the temperature value; toggle always set, value only when on.
    if "use_temperature" in p:
        _set_if_attr(light, "use_temperature", p["use_temperature"])
    if p.get("use_temperature", False) and "temperature" in p:
        _set_if_attr(light, "temperature", p["temperature"])


def _apply_point_light_props(obj, p):
    light = obj.data
    _apply_common_light_props(light, p)
    light.energy = p.get("power", light.energy)
    light.shadow_soft_size = p.get("radius", light.shadow_soft_size)
    if "soft_falloff" in p:
        _set_if_attr(light, "use_soft_falloff", p["soft_falloff"])
    _apply_attrs(light, p, CONTRIBUTION_FACTORS)
    _apply_shadow_block(light, p)
    _apply_distance_block(light, p)


def _apply_sun_light_props(obj, p):
    light = obj.data
    _apply_common_light_props(light, p)
    light.energy = p.get("strength", light.energy)
    light.angle = p.get("angle", light.angle)
    _apply_attrs(light, p, CONTRIBUTION_FACTORS)
    _apply_shadow_block(light, p)


def _apply_spot_light_props(obj, p):
    light = obj.data
    _apply_common_light_props(light, p)
    light.energy = p.get("power", light.energy)
    light.shadow_soft_size = p.get("radius", light.shadow_soft_size)
    if "soft_falloff" in p:
        _set_if_attr(light, "use_soft_falloff", p["soft_falloff"])
    light.spot_size = p.get("angle", light.spot_size)
    light.spot_blend = p.get("blend", light.spot_blend)
    light.show_cone = p.get("show_cone", light.show_cone)
    _apply_attrs(light, p, CONTRIBUTION_FACTORS)
    _apply_shadow_block(light, p)
    _apply_distance_block(light, p)


def _apply_area_light_props(obj, p):
    light = obj.data
    _apply_common_light_props(light, p)
    light.energy = p.get("power", light.energy)
    if "shape" in p:
        light.shape = p["shape"]
    # size_x/y used for Rectangle + Ellipse; size used for Square + Disk. Route by current shape.
    if light.shape in ("RECTANGLE", "ELLIPSE"):
        if "size_x" in p:
            light.size = p["size_x"]
        if "size_y" in p:
            light.size_y = p["size_y"]
    else:
        if "size" in p:
            light.size = p["size"]

    _apply_attrs(light, p, CONTRIBUTION_FACTORS)
    _apply_shadow_block(light, p)
    _apply_distance_block(light, p)


PROPERTY_APPLIERS = {
    "Camera": _apply_camera_props,
    "PointLight": _apply_point_light_props,
    "SunLight": _apply_sun_light_props,
    "SpotLight": _apply_spot_light_props,
    "AreaLight": _apply_area_light_props,
}


def _apply_properties(obj, properties):
    """Apply type-specific properties (camera, lights) to a Blender object."""
    if not properties:
        return

    for key, applier in PROPERTY_APPLIERS.items():
        if key in properties:
            applier(obj, properties[key])
            return


def _create_asset_placeholder(obj_id, asset_id, transform):
    """Create a wireframe cube placeholder when the asset library file is missing."""
    bpy.ops.mesh.primitive_cube_add()
    obj = bpy.context.active_object
    obj.name = f"[MISSING] {asset_id}"
    obj.display_type = 'WIRE'
    obj["meerkat_id"] = obj_id
    _apply_transform(obj, transform)
    state = PluginState()
    state.object_map[obj_id] = obj
    print(f"[Meerkat] Missing asset '{asset_id}' — placed placeholder")


def _create_primitive(op):
    """Return a creator function that calls a Blender add operator."""
    def creator():
        op()
        return bpy.context.active_object
    return creator


OBJECT_CREATORS = {
    "Cube":       _create_primitive(bpy.ops.mesh.primitive_cube_add),
    "Sphere":     _create_primitive(bpy.ops.mesh.primitive_uv_sphere_add),
    "Cylinder":   _create_primitive(bpy.ops.mesh.primitive_cylinder_add),
    "Plane":      _create_primitive(bpy.ops.mesh.primitive_plane_add),
    "Circle":     _create_primitive(bpy.ops.mesh.primitive_circle_add),
    "Icosphere":  _create_primitive(bpy.ops.mesh.primitive_ico_sphere_add),
    "Cone":       _create_primitive(bpy.ops.mesh.primitive_cone_add),
    "Torus":      _create_primitive(bpy.ops.mesh.primitive_torus_add),
    "Grid":       _create_primitive(bpy.ops.mesh.primitive_grid_add),
    "Monkey":     _create_primitive(bpy.ops.mesh.primitive_monkey_add),
    "Camera":     _create_primitive(bpy.ops.object.camera_add),
    "PointLight": _create_primitive(lambda: bpy.ops.object.light_add(type='POINT')),
    "SunLight":   _create_primitive(lambda: bpy.ops.object.light_add(type='SUN')),
    "SpotLight":  _create_primitive(lambda: bpy.ops.object.light_add(type='SPOT')),
    "AreaLight":  _create_primitive(lambda: bpy.ops.object.light_add(type='AREA')),
}


def _create_asset_ref(obj_id, obj_data, transform):
    """Handle AssetRef creation with library linking and placeholder fallback.
    Links the root asset and all its descendants to preserve hierarchy."""
    asset_id = obj_data.get("asset_id")
    prefs = bpy.context.preferences.addons[__package__].preferences
    library_path = prefs.asset_library_path

    if not library_path or not asset_id:
        _create_asset_placeholder(obj_id, asset_id or "unknown", transform)
        return None

    state = PluginState()
    objects_to_link = [asset_id]
    descendants = state.asset_hierarchy.get(asset_id, [])
    objects_to_link.extend(descendants)
    print(f"[Meerkat] Linking asset '{asset_id}' with {len(descendants)} descendants from {library_path}")

    try:
        with bpy.data.libraries.load(library_path, link=False) as (data_from, data_to):
            data_to.objects = objects_to_link

        root_obj = None
        linked_count = 0
        for obj in data_to.objects:
            if obj is not None:
                bpy.context.collection.objects.link(obj)
                linked_count += 1
                if obj.name == asset_id:
                    root_obj = obj

        print(f"[Meerkat] Linked {linked_count}/{len(objects_to_link)} objects, root found: {root_obj is not None}")

        if root_obj:
            return root_obj
        else:
            _create_asset_placeholder(obj_id, asset_id, transform)
            return None
    except Exception as e:
        print(f"[Meerkat] Failed to link asset '{asset_id}': {e}")
        _create_asset_placeholder(obj_id, asset_id, transform)
        return None


def _create_object_from_snapshot(obj_id, obj_data):
    """Create a Blender object from server data and register it in state."""
    state = PluginState()
    obj_type = obj_data.get("object_type")
    name = obj_data.get("name", obj_type)
    transform = obj_data.get("transform", {})
    properties = obj_data.get("properties")

    # AssetRef has special handling (library linking + placeholder fallback)
    if obj_type == "AssetRef":
        obj = _create_asset_ref(obj_id, obj_data, transform)
        if obj is None:
            return
    else:
        creator = OBJECT_CREATORS.get(obj_type)
        if not creator:
            print(f"[Meerkat] Unknown object type: {obj_type}")
            return
        obj = creator()

    if obj is None:
        print(f"[Meerkat] Failed to create {obj_type}")
        return

    obj.name = name
    obj["meerkat_id"] = obj_id
    _apply_transform(obj, transform)
    _apply_properties(obj, properties)
    state.object_map[obj_id] = obj
    print(f"[Meerkat] Created {obj_type} '{name}' id={obj_id}")


def handle_object_created(payload):
    state = PluginState()
    obj_data = payload.get("object", {})
    created_by = payload.get("created_by", "")

    # Echo suppression — don't recreate our own objects
    if created_by == str(state.user_id):
        return

    state.is_applying_remote_update = True
    try:
        obj_id = obj_data.get("object_id", "")
        _create_object_from_snapshot(obj_id, obj_data)
    finally:
        state.is_applying_remote_update = False


# Ordered: check Icosphere before Sphere, Suzanne before Monkey-fallback.
# Substring match tolerates Blender's ".001"-suffixed dedup names.
MESH_NAME_PATTERNS = [
    ("icosphere", "Icosphere"),
    ("sphere",    "Sphere"),
    ("cube",      "Cube"),
    ("cylinder",  "Cylinder"),
    ("plane",     "Plane"),
    ("circle",    "Circle"),
    ("cone",      "Cone"),
    ("torus",     "Torus"),
    ("grid",      "Grid"),
    ("suzanne",   "Monkey"),
    ("monkey",    "Monkey"),
]


def _infer_mesh_type(name):
    lower = name.lower()
    for pattern, object_type in MESH_NAME_PATTERNS:
        if pattern in lower:
            return object_type
    return None


def _classify_new_object(obj):
    """Return (object_type, properties) for a native-added object, or (None, None) to skip."""
    if obj.type == "CAMERA":
        return "Camera", _build_camera_props(obj)
    if obj.type == "LIGHT":
        light_type = obj.data.type
        if light_type == "POINT":
            return "PointLight", _build_point_light_props(obj)
        if light_type == "SUN":
            return "SunLight", _build_sun_light_props(obj)
        if light_type == "SPOT":
            return "SpotLight", _build_spot_light_props(obj)
        if light_type == "AREA":
            return "AreaLight", _build_area_light_props(obj)
        return None, None
    if obj.type == "MESH":
        inferred = _infer_mesh_type(obj.name)
        if inferred is None:
            return None, None
        return inferred, None
    return None, None


def detect_and_send_creations():
    """Tag + sync any scene object that lacks a meerkat_id (e.g. added via Shift+A)."""
    state = PluginState()
    if not state.connected or not state.ws_client:
        return

    for obj in bpy.data.objects:
        if "meerkat_id" in obj or "meerkat_skip" in obj:
            continue
        # Library-linked objects come in via place_asset flow, not native add.
        if obj.library is not None:
            continue

        object_type, properties = _classify_new_object(obj)
        if object_type is None:
            # Mark so we don't re-log/re-check every tick.
            obj["meerkat_skip"] = True
            print(f"[Meerkat] Skipping unsupported object '{obj.name}' (blender type={obj.type})")
            continue

        meerkat_id = str(uuid4())
        obj["meerkat_id"] = meerkat_id
        state.object_map[meerkat_id] = obj

        state.ws_client.send({
            "event_type": "CreateObject",
            "payload": {
                "object_id": meerkat_id,
                "name": obj.name,
                "object_type": object_type,
                "asset_id": None,
                "asset_library": None,
                "transform": build_transform(obj),
                "properties": properties,
            }
        })
        print(f"[Meerkat] Detected native add: {object_type} '{obj.name}' id={meerkat_id}")


def detect_and_send_deletions():
    state = PluginState()
    if not state.connected or not state.ws_client:
        return

    deleted = []
    for meerkat_id, obj in state.object_map.items():
        try:
            gone = obj is None or obj.name not in bpy.data.objects
        except ReferenceError:
            gone = True
        if gone:
            deleted.append(meerkat_id)

    for meerkat_id in deleted:
        state.object_map.pop(meerkat_id, None)
        state.ws_client.send({
            "event_type": "DeleteObject",
            "payload": {
                "object_id": meerkat_id,
            }
        })
        print(f"[Meerkat] Sent DeleteObject: {meerkat_id}")


def handle_object_deleted(payload):
    state = PluginState()
    object_id = payload.get("object_id", "")
    deleted_by = payload.get("deleted_by", "")

    if deleted_by == str(state.user_id):
        return

    state.is_applying_remote_update = True
    try:
        obj = state.object_map.pop(object_id, None)
        if obj is None:
            obj = None
            for o in bpy.data.objects:
                if o.get("meerkat_id") == object_id:
                    obj = o
                    break

        if obj and obj.name in bpy.data.objects:
            bpy.data.objects.remove(obj, do_unlink=True)
            print(f"[Meerkat] Deleted object: {object_id}")
        else:
            print(f"[Meerkat] Object already gone: {object_id}")
    finally:
        state.is_applying_remote_update = False


def handle_transform_updated(payload):
    state = PluginState()
    object_id = payload.get("object_id", "")
    updated_by = payload.get("updated_by", "")

    if updated_by == str(state.user_id):
        return

    state.is_applying_remote_update = True
    try:
        obj = state.object_map.get(object_id)
        if obj is None or obj.name not in bpy.data.objects:
            return

        transform = payload.get("transform", {})
        _apply_transform(obj, transform)
        # Cache what Blender actually stored, not the raw payload,
        # to avoid floating-point drift triggering a re-send
        state.transform_cache[object_id] = build_transform(obj)
    finally:
        state.is_applying_remote_update = False


def handle_user_joined(payload):
    state = PluginState()
    user_id = payload.get("user_id", "")
    state.users[user_id] = {
        "display_name": payload.get("display_name", "Unknown"),
        "color": payload.get("color", [200, 200, 200]),
        "selected_object": None,
    }
    _redraw_panels()
    print(f"[Meerkat] User joined: {payload.get('display_name')}")


def handle_user_left(payload):
    state = PluginState()
    user_id = payload.get("user_id", "")
    removed = state.users.pop(user_id, None)
    state.cursor_positions.pop(user_id, None)
    _redraw_panels()
    if removed:
        print(f"[Meerkat] User left: {removed['display_name']}")


def handle_properties_updated(payload):
    state = PluginState()
    object_id = payload.get("object_id", "")
    updated_by = payload.get("updated_by", "")

    if updated_by == str(state.user_id):
        return

    state.is_applying_remote_update = True
    try:
        obj = state.object_map.get(object_id)
        if obj is None or obj.name not in bpy.data.objects:
            return

        properties = payload.get("properties", {})
        _apply_properties(obj, properties)
        # Cache what Blender actually stored to avoid drift re-sends
        builder = _get_property_builder(obj)
        if builder:
            state.property_cache[object_id] = builder(obj)
    finally:
        state.is_applying_remote_update = False


def handle_name_updated(payload):
    state = PluginState()
    object_id = payload.get("object_id", "")
    updated_by = payload.get("updated_by", "")

    if updated_by == str(state.user_id):
        return

    state.is_applying_remote_update = True
    try:
        obj = state.object_map.get(object_id)
        if obj is None or obj.name not in bpy.data.objects:
            return

        name = payload.get("name", "")
        obj.name = name
        state.name_cache[object_id] = obj.name
    finally:
        state.is_applying_remote_update = False


def handle_user_selected(payload):
    state = PluginState()
    user_id = payload.get("user_id", "")
    object_id = payload.get("object_id")  # can be None (deselect)

    if user_id == str(state.user_id):
        return

    if user_id in state.users:
        state.users[user_id]["selected_object"] = object_id
    _redraw_panels()


def handle_cursor_updated(payload):
    state = PluginState()
    user_id = payload.get("user_id", "")

    if user_id == str(state.user_id):
        return

    position = payload.get("position", [0, 0, 0])
    state.cursor_positions[user_id] = position
    _redraw_panels()


def handle_error(payload):
    state = PluginState()
    code = payload.get("code", "UNKNOWN")
    message = payload.get("message", "Unknown error")
    print(f"[Meerkat] Server error: {code} — {message}")

    if code in ("WRONG_PASSWORD", "SESSION_NOT_FOUND", "SESSION_ALREADY_EXISTS"):
        if state.ws_client:
            state.ws_client.disconnect()
            state.ws_client = None
        state.connecting = False
        state.connected = False
        state.session_id = ""
        state.display_name = ""
        _redraw_panels()
        _popup_error(code, message)


def _popup_error(code, message):
    def draw(self, _context):
        self.layout.label(text=message)
    title = {
        "SESSION_NOT_FOUND": "Session not found",
        "WRONG_PASSWORD": "Wrong password",
        "SESSION_ALREADY_EXISTS": "Session already exists",
    }.get(code, "Server error")
    bpy.context.window_manager.popup_menu(draw, title=title, icon='ERROR')


EVENT_HANDLERS = {
    "FullStateSync": handle_full_state_sync,
    "ObjectCreated": handle_object_created,
    "ObjectDeleted": handle_object_deleted,
    "TransformUpdated": handle_transform_updated,
    "PropertiesUpdated": handle_properties_updated,
    "NameUpdated": handle_name_updated,
    "UserJoined": handle_user_joined,
    "UserLeft": handle_user_left,
    "UserSelected": handle_user_selected,
    "CursorUpdated": handle_cursor_updated,
    "Error": handle_error,
}



def timer_function():
    timer: float = 0.05
    state = PluginState()
    if state.ws_client and state.ws_client.is_evicted() and not state.evicted:
        state.connected = False
        state.evicted = True
        _redraw_panels()
    if not state.ws_client:
        return timer

    verbose_logging = _verbose_logging_enabled()

    while True:
        if not state.ws_client:
            break
        try:
            msg = state.ws_client.incoming.get_nowait()
        except queue.Empty:
            break

        event_type = msg.get("event_type")
        payload = msg.get("payload")
        _log_incoming_event(event_type, payload, verbose_logging)

        handler = EVENT_HANDLERS.get(event_type)
        if handler:
            try:
                handler(payload)
            except Exception as e:
                print(f"[Meerkat] ERROR handling {event_type}: {e}")
                traceback.print_exc()

    if not state.connected:
        return timer

    if not state.is_applying_remote_update:
        detect_and_send_creations()

    # Detect local deletions and notify server
    if not state.is_applying_remote_update:
        detect_and_send_deletions()

    # --- Selection polling ---
    if not state.is_applying_remote_update:
        active = bpy.context.view_layer.objects.active if bpy.context.view_layer else None
        current_selected = None
        obj = active
        while obj is not None:
            if "meerkat_id" in obj:
                current_selected = obj["meerkat_id"]
                break
            obj = obj.parent

        if current_selected != state.last_selected:
            state.last_selected = current_selected
            state.ws_client.send({
                "event_type": "SelectObject",
                "payload": {
                    "object_id": current_selected,
                }
            })

    return timer


def timer_function_cursor():
    """Send stored mouse position to server at ~15Hz."""
    timer = 0.066
    state = PluginState()

    if not state.connected or not state.ws_client:
        return timer
    if state.is_applying_remote_update:
        return timer

    # _last_mouse is set by MEERKAT_OT_cursor_tracker modal operator
    mouse_data = getattr(state, '_last_mouse', None)
    if mouse_data is None:
        return timer

    now = time.monotonic()
    if now - state.last_cursor_send < 0.066:
        return timer

    region, rv3d, mx, my = mouse_data

    try:
        from bpy_extras.view3d_utils import region_2d_to_origin_3d, region_2d_to_vector_3d
        origin = region_2d_to_origin_3d(region, rv3d, (mx, my))
        direction = region_2d_to_vector_3d(region, rv3d, (mx, my))

        depsgraph = bpy.context.evaluated_depsgraph_get()
        hit, location, *_ = bpy.context.scene.ray_cast(depsgraph, origin, direction)

        if hit:
            pos = [location.x, location.y, location.z]
        else:
            fallback = origin + direction * 10.0
            pos = [fallback.x, fallback.y, fallback.z]

        state.last_cursor_send = now
        state.ws_client.send({
            "event_type": "UpdateCursor",
            "payload": {"position": pos}
        })
    except Exception as e:
        print(f"[Meerkat] Cursor send error: {e}")

    return timer
