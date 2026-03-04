# selection_overlay.py — draw colored bounding boxes for other users' selections
import bpy
import gpu
import blf
from gpu_extras.batch import batch_for_shader
from mathutils import Vector
from .state import PluginState


def _get_bbox_corners(obj):
    """Get the 8 world-space corners of an object's bounding box."""
    bbox = obj.bound_box
    matrix = obj.matrix_world
    return [matrix @ Vector(corner) for corner in bbox]


def _bbox_edges(corners):
    """Return pairs of corner indices forming the 12 edges of a bounding box."""
    indices = [
        (0, 1), (1, 2), (2, 3), (3, 0),  # bottom face
        (4, 5), (5, 6), (6, 7), (7, 4),  # top face
        (0, 4), (1, 5), (2, 6), (3, 7),  # vertical edges
    ]
    lines = []
    for a, b in indices:
        lines.append(corners[a])
        lines.append(corners[b])
    return lines


def draw_selection_overlays():
    """Draw callback registered with SpaceView3D. Draws colored bounding boxes
    around objects selected by other users, with their name label."""
    state = PluginState()
    if not state.connected:
        return

    for user_id, info in state.users.items():
        # Skip our own selection
        if user_id == str(state.user_id):
            continue

        selected_id = info.get("selected_object")
        if not selected_id:
            continue

        obj = state.object_map.get(selected_id)
        if obj is None:
            continue

        try:
            if obj.name not in bpy.data.objects:
                continue
        except ReferenceError:
            continue

        # Get color as 0-1 floats from the 0-255 palette
        color_raw = info.get("color", [200, 200, 200])
        color = (color_raw[0] / 255.0, color_raw[1] / 255.0, color_raw[2] / 255.0, 1.0)

        # Draw bounding box wireframe
        corners = _get_bbox_corners(obj)
        lines = _bbox_edges(corners)

        shader = gpu.shader.from_builtin('UNIFORM_COLOR')
        batch = batch_for_shader(shader, 'LINES', {"pos": lines})

        gpu.state.line_width_set(2.0)
        gpu.state.blend_set('ALPHA')
        shader.bind()
        shader.uniform_float("color", color)
        batch.draw(shader)
        gpu.state.line_width_set(1.0)
        gpu.state.blend_set('NONE')

        # Draw user name label at the top of the bounding box
        # Find the highest point of the bbox
        top_center = Vector((0, 0, 0))
        max_z = -float('inf')
        for c in corners:
            top_center += c
            if c.z > max_z:
                max_z = c.z
        top_center /= 8
        top_center.z = max_z + 0.3  # offset above the object

        # Project 3D point to 2D screen space
        region = bpy.context.region
        rv3d = bpy.context.space_data.region_3d
        if region and rv3d:
            from bpy_extras.view3d_utils import location_3d_to_region_2d
            screen_pos = location_3d_to_region_2d(region, rv3d, top_center)
            if screen_pos:
                font_id = 0
                blf.size(font_id, 14)
                blf.color(font_id, color[0], color[1], color[2], 1.0)
                blf.position(font_id, screen_pos.x, screen_pos.y, 0)
                blf.draw(font_id, info.get("display_name", "?"))


def register_draw_handler():
    """Register the selection overlay draw handler."""
    state = PluginState()
    if state.draw_handler is None:
        state.draw_handler = bpy.types.SpaceView3D.draw_handler_add(
            draw_selection_overlays, (), 'WINDOW', 'POST_VIEW'
        )


def unregister_draw_handler():
    """Remove the selection overlay draw handler."""
    state = PluginState()
    if state.draw_handler is not None:
        bpy.types.SpaceView3D.draw_handler_remove(state.draw_handler, 'WINDOW')
        state.draw_handler = None
