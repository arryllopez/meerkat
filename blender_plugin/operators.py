import bpy
import os
import time
from uuid import uuid4
from .state import PluginState
from .websocket_client import WebSocketClient
from .utils import build_transform


# ── Helper: tag object and send CreateObject to server ────────────────────────

def _send_create_object(obj, object_type, properties=None, asset_id=None, asset_library=None):
    """Tag a Blender object with a meerkat_id and send CreateObject to the server."""
    state = PluginState()
    meerkat_id = str(uuid4())
    obj["meerkat_id"] = meerkat_id
    state.object_map[meerkat_id] = obj

    state.ws_client.send({
        "event_type": "CreateObject",
        "payload": {
            "object_id": meerkat_id,
            "name": obj.name,
            "object_type": object_type,
            "asset_id": asset_id,
            "asset_library": asset_library,
            "transform": build_transform(obj),
            "properties": properties,
        }
    })


# ── Connection operators ──────────────────────────────────────────────────────

class MEERKAT_OT_connect(bpy.types.Operator):
    bl_idname = "meerkat.connect"
    bl_label = "Connect"
    bl_description = "Connect to Meerkat server and join a session"

    def execute(self, context):
        state = PluginState()

        if state.connected:
            self.report({'WARNING'}, "Already connected")
            return {'CANCELLED'}

        # Read server URL from addon preferences
        prefs = context.preferences.addons[__package__].preferences
        url = prefs.server_url

        # Read room name and display name from scene properties
        scene = context.scene
        room_name = scene.meerkat_room_name
        display_name = scene.meerkat_display_name

        if not room_name or not display_name:
            self.report({'ERROR'}, "Room name and display name are required")
            return {'CANCELLED'}

        # Clear the scene — Meerkat owns the whole scene, start fresh
        for obj in list(bpy.data.objects):
            bpy.data.objects.remove(obj, do_unlink=True)

        # Create client, connect, and send JoinSession
        state.intentional_disconnect = False
        client = WebSocketClient(url)
        client.connect(session_id=room_name, display_name=display_name)

        state.ws_client = client
        state.session_id = room_name
        state.display_name = display_name
        state.connected = True

        client.send({
            "event_type": "JoinSession",
            "payload": {
                "session_id": room_name,
                "display_name": display_name,
            }
        })

        self.report({'INFO'}, f"Connected to {room_name}")
        return {'FINISHED'}


class MEERKAT_OT_disconnect(bpy.types.Operator):
    bl_idname = "meerkat.disconnect"
    bl_label = "Disconnect"
    bl_description = "Disconnect from the Meerkat session"

    def execute(self, context):
        state = PluginState()

        if not state.connected or not state.ws_client:
            self.report({'WARNING'}, "Not connected")
            return {'CANCELLED'}

        # Send LeaveSession before disconnecting
        state.ws_client.send({
            "event_type": "LeaveSession",
            "payload": None,
        })
        state.intentional_disconnect = True 
        state.ws_client.disconnect()
        state.ws_client = None
        state.connected = False
        state.session_id = ""
        state.display_name = ""
        state.object_map.clear()
        state.users.clear()

        self.report({'INFO'}, "Disconnected")
        return {'FINISHED'}


# ── Primitive creation operators ──────────────────────────────────────────────

class MEERKAT_OT_add_cube(bpy.types.Operator):
    bl_idname = "meerkat.add_cube"
    bl_label = "Add Cube"
    bl_description = "Add a cube and sync to session"

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        bpy.ops.mesh.primitive_cube_add()
        _send_create_object(context.active_object, "Cube")
        return {'FINISHED'}


class MEERKAT_OT_add_sphere(bpy.types.Operator):
    bl_idname = "meerkat.add_sphere"
    bl_label = "Add Sphere"
    bl_description = "Add a sphere and sync to session"

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        bpy.ops.mesh.primitive_uv_sphere_add()
        _send_create_object(context.active_object, "Sphere")
        return {'FINISHED'}


class MEERKAT_OT_add_cylinder(bpy.types.Operator):
    bl_idname = "meerkat.add_cylinder"
    bl_label = "Add Cylinder"
    bl_description = "Add a cylinder and sync to session"

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        bpy.ops.mesh.primitive_cylinder_add()
        _send_create_object(context.active_object, "Cylinder")
        return {'FINISHED'}


# ── Camera creation operator ─────────────────────────────────────────────────

class MEERKAT_OT_add_camera(bpy.types.Operator):
    bl_idname = "meerkat.add_camera"
    bl_label = "Add Camera"
    bl_description = "Add a camera and sync to session"

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        bpy.ops.object.camera_add()
        obj = context.active_object
        cam = obj.data
        properties = {
            "Camera": {
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
            }
        }
        _send_create_object(obj, "Camera", properties=properties)
        return {'FINISHED'}


# ── Light creation operators ─────────────────────────────────────────────────

class MEERKAT_OT_add_point_light(bpy.types.Operator):
    bl_idname = "meerkat.add_point_light"
    bl_label = "Add Point Light"
    bl_description = "Add a point light and sync to session"

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        bpy.ops.object.light_add(type='POINT')
        obj = context.active_object
        light = obj.data
        properties = {
            "PointLight": {
                "color": list(light.color),
                "temperature": 6500.0,
                "exposure": 0.0,
                "power": light.energy,
                "radius": light.shadow_soft_size,
                "soft_falloff": False,
                "normalize": False,
            }
        }
        _send_create_object(obj, "PointLight", properties=properties)
        return {'FINISHED'}


class MEERKAT_OT_add_sun_light(bpy.types.Operator):
    bl_idname = "meerkat.add_sun_light"
    bl_label = "Add Sun Light"
    bl_description = "Add a sun light and sync to session"

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        bpy.ops.object.light_add(type='SUN')
        obj = context.active_object
        light = obj.data
        properties = {
            "SunLight": {
                "color": list(light.color),
                "temperature": 6500.0,
                "exposure": 0.0,
                "normalize": False,
                "strength": light.energy,
                "angle": light.angle,
            }
        }
        _send_create_object(obj, "SunLight", properties=properties)
        return {'FINISHED'}


# ── Asset placement operator ─────────────────────────────────────────────────

def _get_asset_items(self, context):
    """Build the enum items list from loaded asset library names."""
    state = PluginState()
    if not state.asset_library_objects:
        return [("NONE", "No assets loaded", "")]
    return [(name, name, "") for name in state.asset_library_objects]


class MEERKAT_OT_place_asset(bpy.types.Operator):
    bl_idname = "meerkat.place_asset"
    bl_label = "Place Asset"
    bl_description = "Link an asset from the shared library and sync to session"

    asset_name: bpy.props.EnumProperty(name="Asset", items=_get_asset_items)

    def execute(self, context):
        state = PluginState()
        if not state.connected:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}
        if self.asset_name == "NONE":
            self.report({'WARNING'}, "No assets loaded — set asset library path in preferences")
            return {'CANCELLED'}

        prefs = context.preferences.addons[__package__].preferences
        library_path = prefs.asset_library_path
        asset_name = self.asset_name

        # Link the object from the local .blend library
        try:
            with bpy.data.libraries.load(library_path, link=True) as (data_from, data_to):
                data_to.objects = [asset_name]
        except Exception as e:
            self.report({'ERROR'}, f"Failed to link asset: {e}")
            return {'CANCELLED'}

        # Find the newly linked object and add it to the scene
        linked_obj = bpy.data.objects.get(asset_name)
        if not linked_obj:
            self.report({'ERROR'}, f"Asset '{asset_name}' not found after linking")
            return {'CANCELLED'}

        context.collection.objects.link(linked_obj)
        _send_create_object(
            linked_obj, "AssetRef",
            asset_id=asset_name,
            asset_library=os.path.basename(library_path),
        )
        return {'FINISHED'}


# ── Save Scene operator ─────────────────────────────────────────────────────

class MEERKAT_OT_save_scene(bpy.types.Operator):
    bl_idname = "meerkat.save_scene"
    bl_label = "Save Scene"
    bl_description = "Request latest state from server and save as a .blend file"

    def execute(self, context):
        state = PluginState()
        if not state.connected or not state.ws_client:
            self.report({'ERROR'}, "Not connected to a session")
            return {'CANCELLED'}

        # Request fresh state from the server
        state.ws_client.send({
            "event_type": "RequestStateSync",
            "payload": None,
        })

        # Open file browser to save
        bpy.ops.wm.save_as_mainfile('INVOKE_DEFAULT')
        self.report({'INFO'}, "Scene saved")
        return {'FINISHED'}
