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

def draw_cursor_overlays():
    """POST_PIXEL draw callback — draws 2D cursor arrows for other users."""
    state = PluginState()
    if not state.connected:
        return

    region = bpy.context.region
    space = bpy.context.space_data
    if not region or not space:
        return
    rv3d = getattr(space, 'region_3d', None)
    if not rv3d:
        return

    from bpy_extras.view3d_utils import location_3d_to_region_2d

    for user_id, pos in list(state.cursor_positions.items()):
        if user_id == str(state.user_id):
            continue

        user_info = state.users.get(user_id)
        if not user_info:
            continue

        world_pos = Vector(pos)
        screen_pos = location_3d_to_region_2d(region, rv3d, world_pos)
        if not screen_pos:
            continue

        color_raw = user_info.get("color", [200, 200, 200])
        r, g, b = color_raw[0] / 255.0, color_raw[1] / 255.0, color_raw[2] / 255.0

        # Draw a cursor arrow — 2D coords for POST_PIXEL
        x, y = screen_pos.x, screen_pos.y
        arrow_verts = [
            (x, y),              # tip
            (x + 4, y - 18),     # right edge
            (x + 10, y - 13),    # elbow outer
            (x + 17, y - 22),    # pointer tip
            (x + 21, y - 19),    # pointer outer
            (x + 13, y - 10),    # elbow inner
            (x + 18, y - 5),     # right wing
        ]
        # Triangle fan from tip
        tris = []
        for i in range(1, len(arrow_verts) - 1):
            tris.append(arrow_verts[0])
            tris.append(arrow_verts[i])
            tris.append(arrow_verts[i + 1])

        shader = gpu.shader.from_builtin('UNIFORM_COLOR')
        batch = batch_for_shader(shader, 'TRIS', {"pos": tris})

        gpu.state.blend_set('ALPHA')
        shader.bind()
        shader.uniform_float("color", (r, g, b, 0.9))
        batch.draw(shader)

        # Draw outline
        outline_verts = arrow_verts + [arrow_verts[0]]
        outline_pairs = []
        for i in range(len(outline_verts) - 1):
            outline_pairs.append(outline_verts[i])
            outline_pairs.append(outline_verts[i + 1])

        batch_outline = batch_for_shader(shader, 'LINES', {"pos": outline_pairs})
        gpu.state.line_width_set(1.5)
        shader.uniform_float("color", (r * 0.6, g * 0.6, b * 0.6, 1.0))
        batch_outline.draw(shader)
        gpu.state.line_width_set(1.0)
        gpu.state.blend_set('NONE')

        # Draw display name label next to cursor
        display_name = user_info.get("display_name", "?")
        font_id = 0
        blf.size(font_id, 13)
        blf.color(font_id, r, g, b, 1.0)
        blf.position(font_id, x + 22, y - 24, 0)
        blf.draw(font_id, display_name)


def register_draw_handler():
    """Register both draw handlers: POST_VIEW for selection boxes, POST_PIXEL for cursors."""
    state = PluginState()
    if state.draw_handler is None:
        state.draw_handler = bpy.types.SpaceView3D.draw_handler_add(
            draw_selection_overlays, (), 'WINDOW', 'POST_VIEW'
        )
    if state.cursor_draw_handler is None:
        state.cursor_draw_handler = bpy.types.SpaceView3D.draw_handler_add(
            draw_cursor_overlays, (), 'WINDOW', 'POST_PIXEL'
        )


def unregister_draw_handler():
    """Remove both draw handlers."""
    state = PluginState()
    if state.draw_handler is not None:
        bpy.types.SpaceView3D.draw_handler_remove(state.draw_handler, 'WINDOW')
        state.draw_handler = None
    if state.cursor_draw_handler is not None:
        bpy.types.SpaceView3D.draw_handler_remove(state.cursor_draw_handler, 'WINDOW')
        state.cursor_draw_handler = None
