"""
Meerkat Blender Plugin — Test Runner

Run with:
    blender --background --python blender_plugin/tests/run_tests.py

Runs all test modules against real Blender objects with a mock WebSocket client.
"""
import sys
import os

# Add project root to path
project_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

from blender_plugin.tests.helpers import TestResult
from blender_plugin.tests import (
    test_transforms,
    test_properties,
    test_names,
    test_deletion,
    test_echo_suppression,
    test_full_state_sync,
    test_event_dispatch,
)

TEST_MODULES = [
    test_event_dispatch,
    test_transforms,
    test_properties,
    test_names,
    test_deletion,
    test_echo_suppression,
    test_full_state_sync,
]


def main():
    print("=" * 60)
    print("  Meerkat Blender Plugin — Test Suite")
    print("=" * 60)

    result = TestResult()

    for module in TEST_MODULES:
        try:
            module.run(result)
        except Exception as e:
            result.fail(f"{module.__name__} (module-level crash)", str(e))

    all_passed = result.summary()

    # Exit with non-zero code on failure for CI
    if not all_passed:
        sys.exit(1)


if __name__ == "__main__":
    main()
