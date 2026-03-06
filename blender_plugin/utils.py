# utils.py — UUID helpers, transform build/apply, asset library loader
import bpy
import os
from .state import PluginState


def _collect_descendants(obj):
    """Recursively collect all descendant names of an object."""
    result = []
    for child in obj.children:
        result.append(child.name)
        result.extend(_collect_descendants(child))
    return result


def load_asset_library(filepath):
    """Peek inside a .blend file, temporarily link all objects to discover
    the parent-child hierarchy, then store only root names in the dropdown
    and a root -> [descendants] map for placing entire hierarchies."""
    state = PluginState()
    state.asset_library_objects.clear()
    state.asset_hierarchy.clear()

    if not filepath or not os.path.isfile(filepath):
        print(f"[Meerkat] Asset library not found: {filepath}")
        return []

    # Temporarily link all objects to discover hierarchy
    with bpy.data.libraries.load(filepath, link=True) as (data_from, data_to):
        data_to.objects = list(data_from.objects)

    # Find roots (no parent) and map each to its descendants
    roots = []
    for obj in data_to.objects:
        if obj is not None and obj.parent is None:
            descendants = _collect_descendants(obj)
            roots.append(obj.name)
            state.asset_hierarchy[obj.name] = descendants

    # Clean up — remove all temporarily linked objects
    for obj in data_to.objects:
        if obj is not None:
            bpy.data.objects.remove(obj, do_unlink=True)

    state.asset_library_objects = roots
    print(f"[Meerkat] Loaded {len(roots)} root assets from {os.path.basename(filepath)}: {roots}")
    for root, children in state.asset_hierarchy.items():
        print(f"  {root}: {len(children)} children")
    return roots

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