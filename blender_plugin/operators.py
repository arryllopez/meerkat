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

        if state.connected or state.connecting:
            self.report({'WARNING'}, "Already connected or connecting")
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

        state.intentional_disconnect = False
        client = WebSocketClient(url)
        client.connect(session_id=room_name, display_name=display_name)

        state.ws_client = client
        state.session_id = room_name
        state.display_name = display_name
        state.connecting = True
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

        self.report({'INFO'}, f"Creating session {room_name}…")
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

        if state.connected or state.connecting:
            self.report({'WARNING'}, "Already connected or connecting")
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

        state.intentional_disconnect = False
        client = WebSocketClient(url)
        client.connect(session_id=room_name, display_name=display_name)

        state.ws_client = client
        state.session_id = room_name
        state.display_name = display_name
        state.connecting = True
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

        self.report({'INFO'}, f"Connecting to {room_name}…")
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

        # Add all linked objects to the scene collection.
        # data_to.objects preserves the order of objects_to_link, so index 0 is the
        # root even when Blender renames on collision (e.g. "Chair" -> "Chair.001"
        # when re-importing an asset that's already in the scene).
        for obj in data_to.objects:
            if obj is not None:
                context.collection.objects.link(obj)

        root_obj = data_to.objects[0] if data_to.objects else None
        if root_obj is None:
            for obj in data_to.objects:
                if obj is not None:
                    bpy.data.objects.remove(obj, do_unlink=True)
            self.report({'ERROR'}, f"Asset '{asset_name}' not found in library")
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
