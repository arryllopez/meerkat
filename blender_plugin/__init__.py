import bpy
from . import operators, panels, preferences

bl_info = {
    "name": "Meerkat",
    "description": "Real-time collaborative scene layout for Blender.",
    "author": "Lawrence, Lopez",
    "version": (0, 1, 0),
    "blender": (5, 0, 0),
    "location": "View3D > Sidebar > Meerkat",
    "tracker_url": "https://github.com/arryllopez/meerkat/issues",
    "category": "3D View",
}

classes = [
    preferences.MeerkatPreferences,
    operators.MEERKAT_OT_connect,
    operators.MEERKAT_OT_disconnect,
    panels.MEERKAT_PT_main_panel,
]

def register():
    for cls in classes:
        bpy.utils.register_class(cls)

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
