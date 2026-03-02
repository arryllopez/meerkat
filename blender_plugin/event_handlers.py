# event_handlers.py — server event -> Blender action dispatch
import bpy
import queue
from .state import PluginState
from .utils import build_transform


def _transforms_changed(current, cached):
    EPSILON = 1e-5
    for key in ("position", "rotation", "scale"):
        for a, b in zip(current[key], cached[key]):
            if abs(a - b) > EPSILON:
                return True
    return False


def timer_function_transforms():
    timer: float = 0.033
    state = PluginState()

    if not state.connected or not state.ws_client:
        return timer
    if state.is_applying_remote_update:
        return timer

    for meerkat_id, obj in state.object_map.items():
        if obj is None or obj.name not in bpy.data.objects:
            continue

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

    return timer



def handle_full_state_sync(payload):
    state = PluginState()
    state.is_applying_remote_update = True

    try:
        # 1. Delete all existing Meerkat-managed objects from the scene
        objs_to_remove = [obj for obj in bpy.data.objects if "meerkat_id" in obj]
        for obj in objs_to_remove:
            bpy.data.objects.remove(obj, do_unlink=True)
        state.object_map.clear()

        # 2. Recreate objects from the session snapshot
        session = payload.get("session", {})
        objects = session.get("objects", {})
        for obj_id, obj_data in objects.items():
            _create_object_from_snapshot(obj_id, obj_data)

        # 3. Rebuild user list and find our own user_id
        state.users.clear()
        users = session.get("users", {})
        for user_id, user_data in users.items():
            state.users[user_id] = {
                "display_name": user_data.get("display_name", "Unknown"),
                "color": user_data.get("color", [200, 200, 200]),
                "selected_object": user_data.get("selected_object"),
            }
            # Match our display_name to learn our server-assigned user_id
            if user_data.get("display_name") == state.display_name:
                state.user_id = user_id

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


def _apply_point_light_props(obj, p):
    light = obj.data
    if "color" in p:
        light.color = p["color"]
    light.energy = p.get("power", light.energy)
    light.shadow_soft_size = p.get("radius", light.shadow_soft_size)


def _apply_sun_light_props(obj, p):
    light = obj.data
    if "color" in p:
        light.color = p["color"]
    light.energy = p.get("strength", light.energy)
    light.angle = p.get("angle", light.angle)


PROPERTY_APPLIERS = {
    "Camera": _apply_camera_props,
    "PointLight": _apply_point_light_props,
    "SunLight": _apply_sun_light_props,
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
    "Camera":     _create_primitive(bpy.ops.object.camera_add),
    "PointLight":  _create_primitive(lambda: bpy.ops.object.light_add(type='POINT')),
    "SunLight":    _create_primitive(lambda: bpy.ops.object.light_add(type='SUN')),
}


def _create_asset_ref(obj_id, obj_data, transform):
    """Handle AssetRef creation with library linking and placeholder fallback."""
    asset_id = obj_data.get("asset_id")
    prefs = bpy.context.preferences.addons["blender_plugin"].preferences
    library_path = prefs.asset_library_path

    if not library_path or not asset_id:
        _create_asset_placeholder(obj_id, asset_id or "unknown", transform)
        return None

    try:
        with bpy.data.libraries.load(library_path, link=True) as (data_from, data_to):
            data_to.objects = [asset_id]
        obj = bpy.data.objects.get(asset_id)
        if obj:
            bpy.context.collection.objects.link(obj)
            return obj
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


def detect_and_send_deletions():
    state = PluginState()
    if not state.connected or not state.ws_client:
        return

    deleted = []
    for meerkat_id, obj in state.object_map.items():
        if obj is None or obj.name not in bpy.data.objects:
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
    print(f"[Meerkat] User joined: {payload.get('display_name')}")


def handle_user_left(payload):
    state = PluginState()
    user_id = payload.get("user_id", "")
    removed = state.users.pop(user_id, None)
    if removed:
        print(f"[Meerkat] User left: {removed['display_name']}")
        


EVENT_HANDLERS = {
    "FullStateSync": handle_full_state_sync,
    "ObjectCreated": handle_object_created,
    "ObjectDeleted": handle_object_deleted,
    "TransformUpdated": handle_transform_updated,
    "UserJoined": handle_user_joined,
    "UserLeft": handle_user_left,
}


def timer_function():
    timer: float = 0.05
    state = PluginState()
    if not state.connected or not state.ws_client:
        return timer

    while True:
        try:
            msg = state.ws_client.incoming.get_nowait()
        except queue.Empty:
            break

        event_type = msg.get("event_type")
        payload = msg.get("payload")

        handler = EVENT_HANDLERS.get(event_type)
        if handler:
            handler(payload)

    # Detect local deletions and notify server
    if not state.is_applying_remote_update:
        detect_and_send_deletions()

    return timer