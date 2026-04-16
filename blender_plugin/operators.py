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

def _is_last_user_in_session(state) -> bool:
    """Best-effort check for whether this client appears to be the only connected user."""
    return len(state.users) <= 1

class MEERKAT_OT_create_session(bpy.types.Operator):
    bl_idname = "meerkat.create_session"
    bl_label = "Create Session"
    bl_description = "Create a new password-protected Meerkat session"

    def invoke(self, context, event):
        if bpy.data.objects:
            try:
                return context.window_manager.invoke_props_dialog(
                    self,
                    width=420,
                    confirm_text="Create Session",
                    cancel_default=True,
                )
            except TypeError:
                return context.window_manager.invoke_props_dialog(self, width=420)
        return self.execute(context)

    def draw(self, context):
        layout = self.layout
        warning = layout.box()
        warning.alert = True
        warning.label(text="WARNING: Connecting will clear all objects in the scene.", icon='ERROR')
        warning.label(text="Unsaved work will be lost.")
        tip = layout.box()
        tip.label(text="Tip: Save your .blend file before connecting.", icon='FILE_TICK')

    def cancel(self, context):
        self.report({'INFO'}, "Create session cancelled.")

    def execute(self, context):
        state = PluginState()

        if state.connected:
            self.report({'WARNING'}, "Already connected")
            return {'CANCELLED'}

        prefs = context.preferences.addons[__package__].preferences
        url = prefs.server_url

        scene = context.scene
        room_name = scene.meerkat_room_name
        display_name = scene.meerkat_display_name
        password = scene.meerkat_session_password

        if not room_name or not display_name or not password:
            self.report({'ERROR'}, "Room name, display name, and password are required")
            return {'CANCELLED'}

        for obj in list(bpy.data.objects):
            bpy.data.objects.remove(obj, do_unlink=True)

        state.intentional_disconnect = False
        client = WebSocketClient(url)
        client.connect(session_id=room_name, display_name=display_name)

        state.ws_client = client
        state.session_id = room_name
        state.display_name = display_name
        state.connected = True
        state.evicted = False

        client.send({
            "event_type": "CreateSession",
            "payload": {
                "session_id": room_name,
                "display_name": display_name,
                "password": password,
            }
        })

        bpy.ops.meerkat.cursor_tracker('INVOKE_DEFAULT')

        self.report({'INFO'}, f"Created session {room_name}")
        return {'FINISHED'}


class MEERKAT_OT_connect(bpy.types.Operator):
    bl_idname = "meerkat.connect"
    bl_label = "Join Session"
    bl_description = "Join an existing Meerkat session"

    def invoke(self, context, event):
        if bpy.data.objects:
            try:
                return context.window_manager.invoke_props_dialog(
                    self,
                    width=420,
                    confirm_text="Join Session",
                    cancel_default=True,
                )
            except TypeError:
                return context.window_manager.invoke_props_dialog(self, width=420)
        return self.execute(context)

    def draw(self, context):
        layout = self.layout
        warning = layout.box()
        warning.alert = True
        warning.label(text="WARNING: Connecting will clear all objects in the scene.", icon='ERROR')
        warning.label(text="Unsaved work will be lost.")
        tip = layout.box()
        tip.label(text="Tip: Save your .blend file before connecting.", icon='FILE_TICK')

    def cancel(self, context):
        self.report({'INFO'}, "Join session cancelled.")

    def execute(self, context):
        state = PluginState()

        if state.connected:
            self.report({'WARNING'}, "Already connected")
            return {'CANCELLED'}

        prefs = context.preferences.addons[__package__].preferences
        url = prefs.server_url

        scene = context.scene
        room_name = scene.meerkat_room_name
        display_name = scene.meerkat_display_name
        password = scene.meerkat_session_password

        if not room_name or not display_name or not password:
            self.report({'ERROR'}, "Room name, display name, and password are required")
            return {'CANCELLED'}

        for obj in list(bpy.data.objects):
            bpy.data.objects.remove(obj, do_unlink=True)

        state.intentional_disconnect = False
        client = WebSocketClient(url)
        client.connect(session_id=room_name, display_name=display_name)

        state.ws_client = client
        state.session_id = room_name
        state.display_name = display_name
        state.connected = True
        state.evicted = False

        client.send({
            "event_type": "JoinSession",
            "payload": {
                "session_id": room_name,
                "display_name": display_name,
                "password": password,
            }
        })

        bpy.ops.meerkat.cursor_tracker('INVOKE_DEFAULT')

        self.report({'INFO'}, f"Joined session {room_name}")
        return {'FINISHED'}


class MEERKAT_OT_disconnect(bpy.types.Operator):
    bl_idname = "meerkat.disconnect"
    bl_label = "Disconnect"
    bl_description = "Disconnect from the Meerkat session"

    def invoke(self, context, event):
        state = PluginState()

        if not state.connected or not state.ws_client:
            self.report({'WARNING'}, "Not connected")
            return {'CANCELLED'}

        if _is_last_user_in_session(state):
            try:
                return context.window_manager.invoke_props_dialog(
                    self,
                    width=420,
                    confirm_text="Leave Session",
                    cancel_default=True,
                )
            except TypeError:
                # Fallback for Blender versions without confirm_text/cancel_default args.
                return context.window_manager.invoke_props_dialog(self, width=420)

        return self.execute(context)

    def draw(self, context):
        state = PluginState()
        if not _is_last_user_in_session(state):
            return

        layout = self.layout
        warning = layout.box()
        warning.alert = True
        warning.label(text="WARNING: You are the last user in this session.", icon='ERROR')
        warning.label(text="Leaving now ends this in-memory session.")
        warning.label(text="Unsaved collaborative work may be lost.")
        warning.label(text="Press Cancel to stay connected.")

        tip = layout.box()
        tip.label(text="Tip: Save Scene before disconnecting.", icon='FILE_TICK')

    def cancel(self, context):
        self.report({'INFO'}, "Leave cancelled. Still connected.")

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
        state.evicted = False
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

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)

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

        # Build list of objects to link: root + all descendants
        objects_to_link = [asset_name]
        descendants = state.asset_hierarchy.get(asset_name, [])
        objects_to_link.extend(descendants)

        # Link all objects from the local .blend library
        try:
            with bpy.data.libraries.load(library_path, link=False) as (data_from, data_to):
                data_to.objects = objects_to_link
        except Exception as e:
            self.report({'ERROR'}, f"Failed to link asset: {e}")
            return {'CANCELLED'}

        # Add all linked objects to the scene collection
        root_obj = None
        for obj in data_to.objects:
            if obj is not None:
                context.collection.objects.link(obj)
                if obj.name == asset_name:
                    root_obj = obj

        if not root_obj:
            self.report({'ERROR'}, f"Asset '{asset_name}' not found after linking")
            return {'CANCELLED'}

        _send_create_object(
            root_obj, "AssetRef",
            asset_id=asset_name,
            asset_library=os.path.basename(library_path),
        )
        return {'FINISHED'}


# ── Cursor tracker modal operator ───────────────────────────────────────────

class MEERKAT_OT_cursor_tracker(bpy.types.Operator):
    bl_idname = "meerkat.cursor_tracker"
    bl_label = "Meerkat Cursor Tracker"
    bl_options = {'INTERNAL'}

    def modal(self, context, event):
        state = PluginState()
        if not state.connected:
            return {'CANCELLED'}

        if event.type == 'MOUSEMOVE':
            # Find the 3D viewport region under the mouse
            for area in context.screen.areas:
                if area.type != 'VIEW_3D':
                    continue
                for region in area.regions:
                    if region.type != 'WINDOW':
                        continue
                    # Check if mouse is inside this region
                    mx = event.mouse_x - region.x
                    my = event.mouse_y - region.y
                    if 0 <= mx <= region.width and 0 <= my <= region.height:
                        rv3d = area.spaces[0].region_3d
                        state._last_mouse = (region, rv3d, mx, my)
                        break

        return {'PASS_THROUGH'}

    def invoke(self, context, event):
        context.window_manager.modal_handler_add(self)
        return {'RUNNING_MODAL'}


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
