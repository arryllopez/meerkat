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
