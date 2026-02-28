# utils.py — UUID helpers, transform build/apply, asset library loader
import bpy
import os
from .state import PluginState


def load_asset_library(filepath):
    """Peek inside a .blend file and return a list of object names.
    Does NOT import anything — just reads the names so the dropdown inside of belnder can populate"""
    state = PluginState()
    state.asset_library_objects.clear()

    if not filepath or not os.path.isfile(filepath):
        print(f"[Meerkat] Asset library not found: {filepath}")
        return []

    names = []
    with bpy.data.libraries.load(filepath) as (data_from, data_to):
        names = list(data_from.objects)

    state.asset_library_objects = names
    print(f"[Meerkat] Loaded {len(names)} assets from {os.path.basename(filepath)}: {names}")
    return names

#helper to return object id , expected to return a string 
def get_meerkat_id(obj) -> str | None: 
    return obj.get("meerkat_id") 

#helper to find object given uuid
def find_object_by_meerkat_id(uuid):
    for obj in bpy.data.objects:
        if obj.get("meerkat_id") == uuid:
            return obj
    return None

#   convert vector and euler --> list 
# all would be in this format [x,y,z] 
def build_transform(obj):
    return {
        "position": list(obj.location),
        "rotation": list(obj.rotation_euler),
        "scale": list(obj.scale),
    }