import bpy
from . import operators, panels, preferences
from .event_handlers import timer_function, timer_function_transforms, timer_function_cursor
from .selection_overlay import register_draw_handler, unregister_draw_handler


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
    operators.MEERKAT_OT_create_session,
    operators.MEERKAT_OT_connect,
    operators.MEERKAT_OT_disconnect,
    operators.MEERKAT_OT_add_cube,
    operators.MEERKAT_OT_add_sphere,
    operators.MEERKAT_OT_add_cylinder,
    operators.MEERKAT_OT_add_camera,
    operators.MEERKAT_OT_add_point_light,
    operators.MEERKAT_OT_add_sun_light,
    operators.MEERKAT_OT_place_asset,
    operators.MEERKAT_OT_cursor_tracker,
    operators.MEERKAT_OT_save_scene,
    panels.MEERKAT_PT_main_panel,
]

def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    
    bpy.app.timers.register(timer_function)
    bpy.app.timers.register(timer_function_transforms)
    bpy.app.timers.register(timer_function_cursor)
    register_draw_handler()
    bpy.types.Scene.meerkat_room_name = bpy.props.StringProperty(name="Room Name", default="")
    bpy.types.Scene.meerkat_display_name = bpy.props.StringProperty(name="Display Name", default="")
    bpy.types.Scene.meerkat_session_password = bpy.props.StringProperty(
        name="Password", subtype='PASSWORD', default=""
    )
    bpy.types.Scene.meerkat_panel_mode = bpy.props.EnumProperty(
        items=[('JOIN', 'Join', 'Join an existing session'),
               ('CREATE', 'Create', 'Create a new session')],
        default='JOIN',
    )
    # add keybind here for adding stuff in the viewport
    # bpy.context.window_manager.keyconfigs.addon.keymaps.new()


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)

    bpy.app.timers.unregister(timer_function)
    bpy.app.timers.unregister(timer_function_transforms)
    bpy.app.timers.unregister(timer_function_cursor)
    unregister_draw_handler()
    del bpy.types.Scene.meerkat_room_name
    del bpy.types.Scene.meerkat_display_name
    del bpy.types.Scene.meerkat_session_password
    del bpy.types.Scene.meerkat_panel_mode

