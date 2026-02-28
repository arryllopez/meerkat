import bpy
from .state import PluginState


class MEERKAT_PT_main_panel(bpy.types.Panel):
    bl_label = "Meerkat Collaboration"
    bl_idname = "MEERKAT_PT_main_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Meerkat"

    def draw(self, context):
        layout = self.layout
        state = PluginState()

        if not state.connected:
            layout.prop(context.scene, "meerkat_room_name")
            layout.prop(context.scene, "meerkat_display_name")
            layout.operator("meerkat.connect")
        else:
            layout.label(text=f"Connected: {state.session_id}")
            layout.operator("meerkat.disconnect")
            layout.separator()
            layout.label(text="Users")
            layout.label(text="Objects")
