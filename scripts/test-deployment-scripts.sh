#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
python3 "$project_root/scripts/test-production-cache.py"

test_root=$(mktemp -d)
fake_bin="${test_root}/bin"
command_log="${test_root}/commands.log"
command_output="${test_root}/command-output.log"
docker_state="${test_root}/docker-state"
deploy_root="${test_root}/deploy"
release_sha=1111111111111111111111111111111111111111
digest_hex=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
source_image="ghcr.io/poprako-dev/poprako-server@sha256:${digest_hex}"
local_image="poprako-server-prod:sha-${release_sha}"
registry_token=test-registry-token
release_dir="${deploy_root}/releases/${release_sha}"

cleanup() {
    exit_status=$?

    trap - EXIT INT TERM

    rm -rf "$test_root"

    exit "$exit_status"
}

fail() {
    echo "deployment script test failed: $1" >&2
    exit 1
}

assert_contains() {
    expected=$1
    file=$2

    grep -F -q -- "$expected" "$file" || fail "missing '$expected' in $file"
}

assert_not_contains() {
    unexpected=$1
    file=$2

    if grep -F -q -- "$unexpected" "$file"; then
        fail "found unexpected '$unexpected' in $file"
    fi
}

assert_order() {
    first=$1
    second=$2
    file=$3
    first_line=$(grep -F -n "$first" "$file" | sed -n '1s/:.*//p')
    second_line=$(grep -F -n "$second" "$file" | sed -n '1s/:.*//p')

    [ -n "$first_line" ] || fail "missing ordered command '$first'"
    [ -n "$second_line" ] || fail "missing ordered command '$second'"
    [ "$first_line" -lt "$second_line" ] || {
        fail "expected '$first' before '$second'"
    }
}

runtime_values() {
    printf '%s\n' \
        test-user \
        "$registry_token" \
        postgres://poprako:password@postgres/db_poprako_server_prod \
        test-jwt-secret \
        72 \
        0123456789abcdef0123456789abcdef \
        test-access-key \
        test-secret-key \
        test-bucket \
        auto \
        https://assets.example.test \
        1
}

run_remote_deploy() {
    runtime_values | PATH="${fake_bin}:$PATH" \
        TEST_COMMAND_LOG="$command_log" \
        TEST_DOCKER_STATE="$docker_state" \
        TEST_LOCAL_IMAGE="$local_image" \
        TEST_SOURCE_IMAGE="$source_image" \
        sh "$project_root/scripts/ga-remote-deploy.sh" \
        "$source_image" \
        poprako-server-prod \
        "sha-${release_sha}" \
        poprako-server-prod \
        "$deploy_root" \
        8888 \
        127.0.0.1 \
        poprako-prod \
        poprako-postgres
}

trap cleanup EXIT INT TERM

mkdir -p "$fake_bin" "$docker_state" "$release_dir/migrations"

cat >"${fake_bin}/docker" <<'EOF'
#!/usr/bin/env sh
set -eu

command_log=${TEST_COMMAND_LOG:?}
docker_state=${TEST_DOCKER_STATE:?}

record_command() {
    printf 'docker' >>"$command_log"

    for command_arg in "$@"; do
        printf ' %s' "$command_arg" >>"$command_log"
    done

    printf '\n' >>"$command_log"
}

record_command "$@"
docker_command=$1
shift

case "$docker_command" in
    buildx)
        printf 'buildx-config %s\n' "${BUILDX_CONFIG:-}" >>"$command_log"
        printf 'buildx-builder %s\n' "${BUILDX_BUILDER:-}" >>"$command_log"
        metadata_file=

        while [ "$#" -gt 0 ]; do
            case "$1" in
                --metadata-file)
                    metadata_file=$2
                    shift 2
                    ;;
                *)
                    shift
                    ;;
            esac
        done

        if [ -n "$metadata_file" ]; then
            printf '{"containerimage.digest":"sha256:%s"}\n' \
                aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
                >"$metadata_file"
        fi
        ;;
    container)
        container_command=$1
        shift

        case "$container_command" in
            inspect)
                if [ "${1:-}" = "--format" ]; then
                    inspect_format=$2
                    target_container=$3

                    case "$inspect_format" in
                        *State.Health*)
                            if [ "${TEST_NEW_HEALTH_FAIL:-0}" = "1" ] && \
                                [ -f "${docker_state}/new-current" ]; then
                                printf 'unhealthy\n'
                            else
                                printf 'healthy\n'
                            fi
                            ;;
                        *Config.Image*)
                            printf '%s\n' "$TEST_LOCAL_IMAGE"
                            ;;
                    esac

                    exit 0
                fi

                target_container=$1
                [ -f "${docker_state}/${target_container}" ]
                ;;
        esac
        ;;
    exec)
        case "$*" in
            *detailed-metrics*)
                printf 'http_requests_total 1\nhttp_responses_total 1\n'
                ;;
        esac
        ;;
    image)
        image_command=$1
        shift

        case "$image_command" in
            inspect)
                if [ "${1:-}" = "--format" ]; then
                    inspect_format=$2

                    case "$inspect_format" in
                        *RepoDigests*) printf '%s\n' "$TEST_SOURCE_IMAGE" ;;
                        *Id*) printf 'sha256:local-image-id\n' ;;
                    esac
                fi
                ;;
            ls)
                printf '%s\n' "$TEST_LOCAL_IMAGE"
                ;;
            rm) ;;
        esac
        ;;
    info) ;;
    login)
        IFS= read -r registry_token
        [ -n "$registry_token" ]
        ;;
    logout) ;;
    logs) ;;
    network) ;;
    ps)
        printf 'poprako-server-prod Up 1 second 127.0.0.1:8888->8888/tcp\n'
        ;;
    pull)
        [ "${TEST_PULL_FAIL:-0}" != "1" ]
        ;;
    rename)
        source_container=$1
        target_container=$2
        mv "${docker_state}/${source_container}" \
            "${docker_state}/${target_container}"

        if [ "$source_container" = "poprako-server-prod-previous" ]; then
            rm -f "${docker_state}/new-current"
        fi
        ;;
    rm)
        target_container=

        for command_arg in "$@"; do
            case "$command_arg" in
                -*) ;;
                *) target_container=$command_arg ;;
            esac
        done

        rm -f "${docker_state}/${target_container}"

        if [ "$target_container" = "poprako-server-prod" ]; then
            rm -f "${docker_state}/new-current"
        fi
        ;;
    run)
        touch "${docker_state}/poprako-server-prod"
        touch "${docker_state}/new-current"
        ;;
    start) ;;
    stop) ;;
    tag) ;;
    *)
        echo "unexpected docker command: $docker_command" >&2
        exit 1
        ;;
esac
EOF

cat >"${fake_bin}/jq" <<'EOF'
#!/usr/bin/env sh
set -eu

printf 'sha256:%s\n' \
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
EOF

cat >"${fake_bin}/ssh" <<'EOF'
#!/usr/bin/env sh
set -eu

printf 'ssh' >>"${TEST_COMMAND_LOG:?}"

remote_command=

for command_arg in "$@"; do
    printf ' %s' "$command_arg" >>"$TEST_COMMAND_LOG"

    remote_command=$command_arg
done

printf '\n' >>"$TEST_COMMAND_LOG"

case "$remote_command" in
    *ga-remote-deploy.sh*)
        while IFS= read -r ignored_input; do
            :
        done
        ;;
esac
EOF

cat >"${fake_bin}/scp" <<'EOF'
#!/usr/bin/env sh
set -eu

printf 'scp' >>"${TEST_COMMAND_LOG:?}"

for command_arg in "$@"; do
    printf ' %s' "$command_arg" >>"$TEST_COMMAND_LOG"
done

printf '\n' >>"$TEST_COMMAND_LOG"
EOF

cat >"${release_dir}/ga-apply-migrations.sh" <<'EOF'
#!/usr/bin/env sh
set -eu

printf 'migration\n' >>"${TEST_COMMAND_LOG:?}"
EOF

chmod +x \
    "${fake_bin}/docker" \
    "${fake_bin}/jq" \
    "${fake_bin}/scp" \
    "${fake_bin}/ssh"
: >"$command_log"

build_metadata="${test_root}/build-metadata.json"
github_output="${test_root}/github-output"
initial_docker_config="${test_root}/docker-config"
mkdir -p "${initial_docker_config}/buildx"

PATH="${fake_bin}:$PATH" \
BUILD_METADATA_FILE="$build_metadata" \
BUILDX_BUILDER=test-builder \
CACHE_FROM=type=registry,ref=ghcr.io/poprako-dev/poprako-server:buildcache \
CACHE_TO=type=registry,ref=ghcr.io/poprako-dev/poprako-server:buildcache,mode=max,image-manifest=true,oci-mediatypes=true,ignore-error=true \
DOCKER_CONFIG="$initial_docker_config" \
GHCR_TOKEN="$registry_token" \
GHCR_USERNAME=test-user \
GITHUB_OUTPUT="$github_output" \
IMAGE_NAME="ghcr.io/poprako-dev/poprako-server:sha-${release_sha}" \
RUNNER_TEMP="$test_root" \
TEST_COMMAND_LOG="$command_log" \
TEST_DOCKER_STATE="$docker_state" \
TEST_LOCAL_IMAGE="$local_image" \
TEST_SOURCE_IMAGE="$source_image" \
sh "$project_root/scripts/ci-build-prod.sh" >"$command_output" 2>&1

assert_contains "docker buildx build" "$command_log"
assert_contains "--progress plain" "$command_log"
assert_contains "buildx-config ${initial_docker_config}/buildx" "$command_log"
assert_contains "buildx-builder test-builder" "$command_log"
assert_contains "--cache-from type=registry" "$command_log"
assert_contains "--cache-to type=registry" "$command_log"
assert_contains "--push" "$command_log"
assert_contains "image_ref=${source_image}" "$github_output"
assert_not_contains "$registry_token" "$command_log"
assert_not_contains "$registry_token" "$command_output"

: >"$command_log"
: >"$command_output"
rm -f "${docker_state}/poprako-server-prod" "${docker_state}/new-current"

run_remote_deploy >"$command_output" 2>&1

assert_contains "docker pull ${source_image}" "$command_log"
assert_contains "docker tag ${source_image} ${local_image}" "$command_log"
assert_contains "migration" "$command_log"
assert_contains "docker run" "$command_log"
assert_order "docker pull" "migration" "$command_log"
assert_order "migration" "docker run" "$command_log"
assert_not_contains "$registry_token" "$command_log"
assert_not_contains "$registry_token" "$command_output"

: >"$command_log"
: >"$command_output"
rm -f "${docker_state}/poprako-server-prod" "${docker_state}/new-current"

if runtime_values | PATH="${fake_bin}:$PATH" \
    TEST_COMMAND_LOG="$command_log" \
    TEST_DOCKER_STATE="$docker_state" \
    TEST_LOCAL_IMAGE="$local_image" \
    TEST_PULL_FAIL=1 \
    TEST_SOURCE_IMAGE="$source_image" \
    sh "$project_root/scripts/ga-remote-deploy.sh" \
    "$source_image" \
    poprako-server-prod \
    "sha-${release_sha}" \
    poprako-server-prod \
    "$deploy_root" \
    8888 \
    127.0.0.1 \
    poprako-prod \
    poprako-postgres >"$command_output" 2>&1; then
    fail "remote deployment succeeded after an image pull failure"
fi

assert_contains "docker pull ${source_image}" "$command_log"
assert_not_contains "migration" "$command_log"
assert_not_contains "docker stop" "$command_log"
assert_not_contains "docker run" "$command_log"

: >"$command_log"
: >"$command_output"
touch "${docker_state}/poprako-server-prod"
rm -f "${docker_state}/new-current"

if TEST_NEW_HEALTH_FAIL=1 run_remote_deploy >"$command_output" 2>&1; then
    fail "remote deployment succeeded with an unhealthy new container"
fi

assert_contains "docker rename poprako-server-prod poprako-server-prod-previous" \
    "$command_log"
assert_contains "docker rename poprako-server-prod-previous poprako-server-prod" \
    "$command_log"
assert_contains "previous release restored" "$command_output"

: >"$command_log"
: >"$command_output"

PATH="${fake_bin}:$PATH" \
DATABASE_URL=postgres://poprako:password@postgres/db_poprako_server_prod \
DEPLOY_BIND_HOST=127.0.0.1 \
DEPLOY_DOCKER_NETWORK=poprako-prod \
DEPLOY_HOST=prod.example.test \
DEPLOY_KNOWN_HOSTS="prod.example.test ssh-ed25519 test-key" \
DEPLOY_PORT=22 \
DEPLOY_POSTGRES_CONTAINER=poprako-postgres \
DEPLOY_PUBLIC_PORT=8888 \
DEPLOY_ROOT="$deploy_root" \
DEPLOY_SHA="$release_sha" \
DEPLOY_SOURCE_IMAGE="$source_image" \
DEPLOY_SSH_PRIVATE_KEY=test-private-key \
DEPLOY_USER=poprako-deploy \
GHCR_TOKEN="$registry_token" \
GHCR_USERNAME=test-user \
JWT_EXPIRATION_HOURS=72 \
JWT_SECRET=test-jwt-secret \
POPRAKO_SNOWFLAKE_NODE_ID=1 \
R2_ACCESS_KEY_ID=test-access-key \
R2_ACCOUNT_ID=0123456789abcdef0123456789abcdef \
R2_BUCKET_NAME=test-bucket \
R2_CUSTOM_DOMAIN=https://assets.example.test \
R2_REGION=auto \
R2_SECRET_ACCESS_KEY=test-secret-key \
RUNNER_TEMP="$test_root" \
TEST_COMMAND_LOG="$command_log" \
sh "$project_root/scripts/ci-deploy-production.sh" \
    >"$command_output" 2>&1

assert_contains "scp" "$command_log"
assert_contains "migrations" "$command_log"
assert_contains "scripts/ga-remote-deploy.sh" "$command_log"
assert_contains "$source_image" "$command_log"
assert_not_contains ".tar" "$command_log"
assert_not_contains "docker build" "$command_log"
assert_not_contains "$registry_token" "$command_log"
assert_not_contains "$registry_token" "$command_output"

if sh "$project_root/scripts/ga-remote-deploy.sh" \
    ghcr.io/poprako-dev/poprako-server@sha256:invalid \
    poprako-server-prod \
    "sha-${release_sha}" \
    poprako-server-prod \
    "$deploy_root" \
    8888 \
    127.0.0.1 \
    poprako-prod \
    poprako-postgres >"$command_output" 2>&1; then
    fail "remote deployment accepted an invalid source digest"
fi

echo "deployment script tests passed"
