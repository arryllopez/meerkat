import bpy
from .state import PluginState
from .websocket_client import WebSocketClient


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

        # Create client, connect, and send JoinSession
        client = WebSocketClient(url)
        client.connect()

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

        state.ws_client.disconnect()
        state.ws_client = None
        state.connected = False
        state.session_id = ""
        state.display_name = ""
        state.object_map.clear()
        state.users.clear()

        self.report({'INFO'}, "Disconnected")
        return {'FINISHED'}
