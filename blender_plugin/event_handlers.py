# event_handlers.py — server event -> Blender action dispatch
import bpy
import queue
from .state import PluginState


def timer_function():
    timer: float = 0.05
    state = PluginState()
    if not state.connected or not state.ws_client:
        return timer

    while True:
        try:
            msg = state.ws_client.incoming.get_nowait()
        except queue.Empty:
            break

        event_type = msg.get("event_type")
        payload = msg.get("payload")

        if event_type == "FullStateSync":
            handle_full_state_sync(payload)
        elif event_type == "ObjectCreated":
            handle_object_created(payload)

    return timer


def handle_full_state_sync(payload):
    state = PluginState()
    state.is_applying_remote_update = True

    try:
        # 1. Delete all existing Meerkat-managed objects from the scene
        objs_to_remove = [obj for obj in bpy.data.objects if "meerkat_id" in obj]
        for obj in objs_to_remove:
            bpy.data.objects.remove(obj, do_unlink=True)
        state.object_map.clear()

        # 2. Recreate objects from the session snapshot
        session = payload.get("session", {})
        objects = session.get("objects", {})
        for obj_id, obj_data in objects.items():
            _create_object_from_snapshot(obj_id, obj_data)

        # 3. Rebuild user list
        state.users.clear()
        users = session.get("users", {})
        for user_id, user_data in users.items():
            state.users[user_id] = {
                "display_name": user_data.get("display_name", "Unknown"),
                "color": user_data.get("color", [200, 200, 200]),
                "selected_object": user_data.get("selected_object"),
            }

        print(f"[Meerkat] FullStateSync: {len(objects)} objects, {len(users)} users")

    finally:
        state.is_applying_remote_update = False


def _create_object_from_snapshot(obj_id, obj_data):
    """Dispatch to the correct Blender object creator by type.
    Phase 3 fills in the real creation logic — for now just logs."""
    obj_type = obj_data.get("object_type")
    name = obj_data.get("name", obj_type)
    print(f"[Meerkat] Would create {obj_type} '{name}' id={obj_id}")


def handle_object_created(payload):
    pass