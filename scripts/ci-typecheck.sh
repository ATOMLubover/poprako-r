#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
integration_root="$project_root/tests/integration-tests"

cd "$integration_root"

deno ci
deno task check
