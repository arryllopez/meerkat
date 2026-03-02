"""Tests for user presence — join, leave, color storage, panel data, FullStateSync users."""
from blender_plugin.event_handlers import (
    handle_user_joined,
    handle_user_left,
    handle_full_state_sync,
)
from blender_plugin.state import PluginState
from blender_plugin.tests.helpers import reset_state, clear_scene, TestResult


def run(result):
    print("\n--- Presence Tests ---")

    # ── UserJoined stores display_name ──

    clear_scene()
    state, mock_ws = reset_state()

    handle_user_joined({
        "user_id": "user-alice",
        "display_name": "Alice",
        "color": [231, 76, 60],
    })

    if state.users.get("user-alice", {}).get("display_name") == "Alice":
        result.ok("UserJoined stores display_name")
    else:
        result.fail("UserJoined stores display_name")

    # ── UserJoined stores color as-is ──

    if state.users["user-alice"]["color"] == [231, 76, 60]:
        result.ok("UserJoined stores color from palette")
    else:
        result.fail("UserJoined stores color from palette",
                     f"got {state.users['user-alice']['color']}")

    # ── UserJoined initializes selected_object to None ──

    if state.users["user-alice"]["selected_object"] is None:
        result.ok("UserJoined initializes selected_object to None")
    else:
        result.fail("UserJoined initializes selected_object to None")

    # ── Multiple users can join ──

    handle_user_joined({
        "user_id": "user-bob",
        "display_name": "Bob",
        "color": [46, 204, 113],
    })

    if len(state.users) == 2:
        result.ok("Multiple users stored in state.users")
    else:
        result.fail("Multiple users stored in state.users",
                     f"expected 2, got {len(state.users)}")

    # ── Each user gets their own color ──

    if state.users["user-alice"]["color"] != state.users["user-bob"]["color"]:
        result.ok("Different users have different colors")
    else:
        result.fail("Different users have different colors")

    # ── UserLeft removes the correct user ──

    handle_user_left({"user_id": "user-alice"})

    if "user-alice" not in state.users:
        result.ok("UserLeft removes correct user")
    else:
        result.fail("UserLeft removes correct user")

    if "user-bob" in state.users:
        result.ok("UserLeft preserves other users")
    else:
        result.fail("UserLeft preserves other users")

    # ── UserLeft for nonexistent user is no-op ──

    try:
        handle_user_left({"user_id": "ghost-user"})
        result.ok("UserLeft for nonexistent user → no crash")
    except Exception as e:
        result.fail("UserLeft for nonexistent user → no crash", str(e))

    # ── UserJoined with missing color uses default ──

    clear_scene()
    state, mock_ws = reset_state()

    handle_user_joined({
        "user_id": "user-no-color",
        "display_name": "NoColor",
    })

    if state.users["user-no-color"]["color"] == [200, 200, 200]:
        result.ok("UserJoined with missing color uses default [200,200,200]")
    else:
        result.fail("UserJoined with missing color uses default",
                     f"got {state.users['user-no-color']['color']}")

    # ── UserJoined with missing display_name uses 'Unknown' ──

    handle_user_joined({
        "user_id": "user-no-name",
        "color": [52, 152, 219],
    })

    if state.users["user-no-name"]["display_name"] == "Unknown":
        result.ok("UserJoined with missing display_name uses 'Unknown'")
    else:
        result.fail("UserJoined with missing display_name uses 'Unknown'",
                     f"got {state.users['user-no-name']['display_name']}")

    # ── FullStateSync rebuilds user list ──

    clear_scene()
    state, mock_ws = reset_state()

    # Add a stale user that should be wiped
    state.users["stale-user"] = {
        "display_name": "Stale",
        "color": [0, 0, 0],
        "selected_object": None,
    }

    handle_full_state_sync({
        "session": {
            "objects": {},
            "users": {
                "user-charlie": {
                    "display_name": "Charlie",
                    "color": [155, 89, 182],
                    "selected_object": None,
                },
                "user-dana": {
                    "display_name": "Dana",
                    "color": [230, 126, 34],
                    "selected_object": None,
                },
            },
        }
    })

    if "stale-user" not in state.users:
        result.ok("FullStateSync clears stale users")
    else:
        result.fail("FullStateSync clears stale users")

    if len(state.users) == 2:
        result.ok("FullStateSync populates correct user count")
    else:
        result.fail("FullStateSync populates correct user count",
                     f"expected 2, got {len(state.users)}")

    if state.users.get("user-charlie", {}).get("display_name") == "Charlie":
        result.ok("FullStateSync stores user Charlie correctly")
    else:
        result.fail("FullStateSync stores user Charlie correctly")

    if state.users.get("user-dana", {}).get("color") == [230, 126, 34]:
        result.ok("FullStateSync stores user Dana color correctly")
    else:
        result.fail("FullStateSync stores user Dana color correctly")

    # ── FullStateSync identifies local user by display_name ──

    clear_scene()
    state, mock_ws = reset_state()
    state.display_name = "Me"

    handle_full_state_sync({
        "session": {
            "objects": {},
            "users": {
                "server-assigned-id": {
                    "display_name": "Me",
                    "color": [52, 152, 219],
                    "selected_object": None,
                },
                "other-user-id": {
                    "display_name": "Other",
                    "color": [241, 196, 15],
                    "selected_object": None,
                },
            },
        }
    })

    if state.user_id == "server-assigned-id":
        result.ok("FullStateSync matches local user_id by display_name")
    else:
        result.fail("FullStateSync matches local user_id by display_name",
                     f"got {state.user_id}")

    # ── Rejoining user gets overwritten (not duplicated) ──

    clear_scene()
    state, mock_ws = reset_state()

    handle_user_joined({
        "user_id": "user-repeat",
        "display_name": "Repeat",
        "color": [231, 76, 60],
    })
    handle_user_joined({
        "user_id": "user-repeat",
        "display_name": "Repeat-Updated",
        "color": [46, 204, 113],
    })

    if len([k for k in state.users if k == "user-repeat"]) == 1:
        result.ok("Rejoining user overwrites, not duplicates")
    else:
        result.fail("Rejoining user overwrites, not duplicates")

    if state.users["user-repeat"]["display_name"] == "Repeat-Updated":
        result.ok("Rejoining user gets updated display_name")
    else:
        result.fail("Rejoining user gets updated display_name")

    if state.users["user-repeat"]["color"] == [46, 204, 113]:
        result.ok("Rejoining user gets updated color")
    else:
        result.fail("Rejoining user gets updated color")
