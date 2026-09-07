#!/usr/bin/env python3
"""Check the dependency stage without running Docker or a release build."""

import json
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import unittest


PROJECT = Path(__file__).resolve().parent.parent


def metadata(directory):
    result = subprocess.run(
        ["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"],
        cwd=directory,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)["packages"]


class ProductionCacheTest(unittest.TestCase):
    def test_dependency_boundary_and_workspace_stubs(self):
        instructions = [
            line.strip()
            for line in (PROJECT / "Dockerfile").read_text().replace("\\\n", " ").splitlines()
            if line.strip() and not line.startswith("#")
        ]
        dependency_build = next(
            index for index, line in enumerate(instructions)
            if line.startswith("RUN cargo build ")
        )
        real_packages = metadata(PROJECT)

        with tempfile.TemporaryDirectory(prefix="poprako-cache-test-") as directory:
            fixture = Path(directory)

            for line in instructions[:dependency_build]:
                if line.startswith("COPY "):
                    *sources, destination = shlex.split(line)[1:]

                    for source in sources:
                        # A locale, test, or Rust file here would bust the cache.
                        self.assertIn(Path(source).name, {"Cargo.toml", "Cargo.lock"})
                        target = fixture / destination

                        if destination.endswith("/"):
                            target = target / Path(source).name

                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(PROJECT / source, target)

                if line.startswith("RUN mkdir "):
                    subprocess.run(["sh", "-eu", "-c", line[4:]], cwd=fixture, check=True)

            stub_packages = metadata(fixture)
            self.assertEqual(
                {package["name"] for package in real_packages},
                {package["name"] for package in stub_packages},
            )

            for real in real_packages:
                relative_manifest = Path(real["manifest_path"]).relative_to(PROJECT)
                self.assertEqual(
                    (PROJECT / relative_manifest).read_bytes(),
                    (fixture / relative_manifest).read_bytes(),
                )
                stub = next(package for package in stub_packages if package["name"] == real["name"])
                # Preserve libraries, proc macros, and executable target identity.
                expected_targets = {
                    (target["name"], tuple(target["kind"]))
                    for target in real["targets"]
                    if set(target["kind"]) & {"lib", "bin", "proc-macro"}
                }
                actual_targets = {
                    (target["name"], tuple(target["kind"]))
                    for target in stub["targets"]
                    if set(target["kind"]) & {"lib", "bin", "proc-macro"}
                }
                self.assertEqual(expected_targets, actual_targets)

                for target in stub["targets"]:
                    self.assertTrue(Path(target["src_path"]).is_file())

            # Clean every workspace package in release mode before real COPY.
            command = shlex.split(instructions[dependency_build])
            clean = command[command.index("clean") + 1:]
            self.assertIn("--release", clean)
            cleaned_packages = {
                clean[index + 1] for index, value in enumerate(clean)
                if value == "--package"
            }
            self.assertEqual(cleaned_packages, {package["name"] for package in real_packages})

            # Registry sources and target artifacts must survive remote export.
            self.assertNotIn("--mount=type=cache", instructions[dependency_build])


if __name__ == "__main__":
    unittest.main()
