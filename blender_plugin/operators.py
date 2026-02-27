import bpy


class MEERKAT_OT_connect(bpy.types.Operator):
    bl_idname = "meerkat.connect"
    bl_label = "Connect"
    bl_description = "Connect to Meerkat server and join a session"

    def execute(self, context):
        return {'FINISHED'}


class MEERKAT_OT_disconnect(bpy.types.Operator):
    bl_idname = "meerkat.disconnect"
    bl_label = "Disconnect"
    bl_description = "Disconnect from the Meerkat session"

    def execute(self, context):
        return {'FINISHED'}
