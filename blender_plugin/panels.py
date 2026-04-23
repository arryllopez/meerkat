import bpy
from .state import PluginState


def _connection_status_lines(state):
    lines = []
    # Reconnect UI removed: no retry logic in client as of 2026-04-13.
    if not state.connected and state.evicted:
        lines.append(("Connection closed: client fell behind", 'ERROR'))
    return lines


class MEERKAT_PT_main_panel(bpy.types.Panel):
    bl_label = "Meerkat Collaboration"
    bl_idname = "MEERKAT_PT_main_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Meerkat"

    def draw(self, context):
        layout = self.layout
        state = PluginState()

        for text, icon in _connection_status_lines(state):
            layout.label(text=text, icon=icon)

        # Reconnect UI removed: no retry logic in client as of 2026-04-13.
        if not state.connected:
            row = layout.row(align=True)
            row.prop(context.scene, "meerkat_panel_mode", expand=True)

            layout.prop(context.scene, "meerkat_room_name")
            layout.prop(context.scene, "meerkat_session_password")
            layout.prop(context.scene, "meerkat_display_name")

            if context.scene.meerkat_panel_mode == 'CREATE':
                layout.operator("meerkat.create_session")
            else:
                layout.operator("meerkat.connect")
        else:
            layout.separator()
            box = layout.box() 
            box.label(text="Users") 
            for user_id, info in state.users.items():
                row = box.row() 
                row.label(text=info["display_name"])
                if user_id == str(state.user_id): 
                    row.label(text="(you)")

            if len(state.users) <= 1:
                warning = layout.box()
                warning.alert = True
                warning.label(text="LAST USER IN SESSION", icon='ERROR')
                warning.label(text="Disconnecting can lose unsaved collaboration state.")

            if state.asset_library_objects:
                layout.separator()
                layout.label(text="Asset Library")
                layout.operator("meerkat.place_asset")

            layout.separator()
            layout.operator("meerkat.save_scene", icon='FILE_TICK')
            disconnect_row = layout.row()
            disconnect_row.alert = len(state.users) <= 1
            disconnect_row.operator("meerkat.disconnect", icon='CANCEL')
            
