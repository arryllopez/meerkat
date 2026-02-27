import bpy
from bpy.props import StringProperty


class MeerkatPreferences(bpy.types.AddonPreferences):
    bl_idname = __package__

    server_url: StringProperty(
        name="Server URL",
        description="WebSocket server address",
        default="ws://localhost:8000/ws",
    )

    asset_library_path: StringProperty(
        name="Asset Library",
        description="Path to shared .blend asset library file",
        subtype='FILE_PATH',
        default="",
    )

    def draw(self, context):
        layout = self.layout
        layout.prop(self, "server_url")
        layout.prop(self, "asset_library_path")
