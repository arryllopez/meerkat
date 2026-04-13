import bpy
from bpy.props import BoolProperty, StringProperty


def _on_asset_library_changed(self, context):
    from .utils import load_asset_library
    load_asset_library(self.asset_library_path)


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
        update=_on_asset_library_changed,
    )

    verbose_logging: BoolProperty(
        name="Verbose Logging",
        description="Print per-event payload logs in the Blender console",
        default=False,
    )

    def draw(self, context):
        layout = self.layout
        layout.prop(self, "server_url")
        layout.prop(self, "asset_library_path")
        layout.prop(self, "verbose_logging")
