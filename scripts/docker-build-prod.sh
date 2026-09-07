#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

image_name="${IMAGE_NAME:-poprako-server-prod:latest}"
platform="${PLATFORM:-linux/amd64}"
cache_from=${CACHE_FROM:-}
cache_to=${CACHE_TO:-}
metadata_file=${BUILD_METADATA_FILE:-}

case "${PUSH:-0}" in
    0)
        output_flag="--load"
        ;;
    1)
        output_flag="--push"
        ;;
    *)
        echo "PUSH must be 0 or 1" >&2
        exit 1
        ;;
esac

set -- docker buildx build \
    --progress plain \
    --ulimit nofile=65536 \
    --platform "$platform" \
    --tag "$image_name"

if [ -n "$cache_from" ]; then
    set -- "$@" --cache-from "$cache_from"
fi

if [ -n "$cache_to" ]; then
    set -- "$@" --cache-to "$cache_to"
fi

if [ -n "$metadata_file" ]; then
    set -- "$@" --metadata-file "$metadata_file"
fi

set -- "$@" "$output_flag" "$project_root"

"$@"
