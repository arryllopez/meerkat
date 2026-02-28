# event_handlers.py — server event -> Blender action dispatch
import queue
from .state import PluginState

def timer_function(): 
    timer : float = 0.05
    state = PluginState()
    if not state.connected or not state.ws_client:
        return timer
    
    # while the message queue is filled, drain it
    while True:
        try:
            msg = state.ws_client.incoming.get_nowait()
        except queue.Empty:
            break
        
        # extract the event type and the payload
        event_type = msg.get("event_type")
        payload = msg.get("payload")

        #handlers here will handle the event type in blender
        if event_type == "FullStateSync":
            handle_full_state_sync(payload)
        elif event_type == "ObjectCreated":
            handle_object_created(payload)
        
    return timer

            


def handle_full_state_sync(payload):
    pass
def handle_object_created(payload):
    pass