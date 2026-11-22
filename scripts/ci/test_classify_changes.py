#!/usr/bin/env python3
"""Tests for the change classifier, run in CI before it routes the jobs."""

import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPT = HERE / "classify-changes.py"


def classify(files, event="pull_request"):
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "--event", event, "--output", "-"],
        input="\n".join(files),
        text=True,
        capture_output=True,
        check=True,
    )
    outputs = {}
    for line in result.stdout.splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            outputs[key] = value == "true"
    return outputs


class ClassifyChangesTest(unittest.TestCase):
    def test_documentation_only_runs_nothing(self):
        self.assertEqual(classify(["docs/development-environment.md", "README.md"]),
                         {"rust": False, "renderer": False, "code": False, "full": False})

    def test_renderer_only_skips_rust(self):
        result = classify(["src/renderer/viewState.js"])
        self.assertTrue(result["renderer"])
        self.assertFalse(result["rust"])
        self.assertTrue(result["code"])

    def test_rust_only_skips_renderer_tests(self):
        result = classify(["src-tauri/src/main.rs"])
        self.assertTrue(result["rust"])
        self.assertFalse(result["renderer"])

    def test_native_plugins_are_rust(self):
        result = classify(["native-plugins/stylus/src/lib.rs"])
        self.assertTrue(result["rust"])
        self.assertFalse(result["renderer"])

    def test_workflow_change_runs_everything(self):
        result = classify([".github/workflows/build.yml"])
        self.assertTrue(result["rust"])
        self.assertTrue(result["renderer"])

    def test_package_lock_changes_both_builds(self):
        result = classify(["package-lock.json"])
        self.assertTrue(result["rust"])
        self.assertTrue(result["renderer"])

    def test_unrecognised_input_runs_everything(self):
        result = classify(["new-tool/config.xyz"])
        self.assertTrue(result["rust"])
        self.assertTrue(result["renderer"])

    def test_missing_file_list_runs_everything(self):
        result = classify([])
        self.assertTrue(result["full"])
        self.assertTrue(result["code"])

    def test_push_runs_everything(self):
        result = classify(["docs/development-environment.md"], event="push")
        self.assertTrue(result["full"])
        self.assertTrue(result["rust"])
        self.assertTrue(result["renderer"])


if __name__ == "__main__":
    unittest.main()

