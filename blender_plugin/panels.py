import bpy


class MEERKAT_PT_main_panel(bpy.types.Panel):
    bl_label = "Meerkat Collaboration"
    bl_idname = "MEERKAT_PT_main_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Meerkat"

    def draw(self, context):
        layout = self.layout
        layout.label(text="Not connected")
