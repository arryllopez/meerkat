# state.py — PluginState singleton
# Singleton was chosen to ensure that only one instance of eaach class can ever
# be possible
from uuid import uuid4

from dataclasses import dataclass, field
from typing import Optional 
from .websocket_client import WebSocketClient


class Singleton(type):
    _instances = {}

    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super().__call__(*args, **kwargs)
        return cls._instances[cls]

    

@dataclass
class PluginState(metaclass=Singleton):
    connected: bool = False
    ws_client: WebSocketClient | None = None
    session_id: str = ""
    user_id: str = field(default_factory=lambda: str(uuid4()))
    display_name: str = ""
    object_map: dict = field(default_factory=dict)       # meerkat_id -> bpy.types.Object
    users: dict = field(default_factory=dict)             # user_id -> {display_name, color, selected_object}
    is_applying_remote_update: bool = False
    asset_library_objects: list = field(default_factory=list)  # names from the shared .blend library
    asset_hierarchy: dict = field(default_factory=dict)          # root_name -> [child_names]
    transform_cache: dict = field(default_factory=dict)        # meerkat_id -> {position, rotation, scale}
    property_cache: dict = field(default_factory=dict)         # meerkat_id -> last sent properties dict
    name_cache: dict = field(default_factory=dict)             # meerkat_id -> last sent obj.name
    last_selected: str | None = None                             # meerkat_id of last selected object (or None)
    draw_handler: object | None = None                           # SpaceView3D POST_VIEW handler
    cursor_draw_handler: object | None = None                    # SpaceView3D POST_PIXEL handler
    reconnecting: bool = False 
    reconnect_attempt :int = 0 
    intentional_disconnect : bool = False 
    evicted: bool = False
    cursor_positions: dict = field(default_factory=dict)
    last_cursor_send: float = 0.0
    _last_mouse: object | None = None                        # (region, rv3d, mx, my) from modal
    
    
