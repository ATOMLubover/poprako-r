#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
env_file="${ENV_FILE:-$project_root/.env}"
api_base_url="${API_BASE_URL:-http://127.0.0.1:8888}"
start_api_server="${START_API_SERVER:-1}"
run_migrations="${RUN_MIGRATIONS:-1}"
server_pid=""

if [ -f "$env_file" ]; then
    set -a
    # shellcheck disable=SC1090
    . "$env_file"
    set +a
fi

RUST_LOG="${INTEGRATION_RUST_LOG:-$RUST_LOG}"

export RUST_LOG

integration_database_url="${INTEGRATION_DATABASE_URL:-}"

if [ -z "$integration_database_url" ]; then
    echo "INTEGRATION_DATABASE_URL must be set" >&2
    exit 1
fi

# Override DATABASE_URL so diesel and the API server both target the
# integration database instead of the dev database.
export DATABASE_URL="$integration_database_url"
export API_BASE_URL="$api_base_url"

# Derive the maintenance connection (postgres database on the same server)
# and the integration database name from INTEGRATION_DATABASE_URL, so we can
# DROP the integration database after the run.
db_name="${integration_database_url##*/}"
maintenance_url="${integration_database_url%/*}/postgres"

case "$db_name" in
    *_integration) ;;
    *)
        echo "integration database name must end with _integration" >&2
        exit 1
        ;;
esac

drop_integration_db() {
    psql "$maintenance_url" \
        -c "DROP DATABASE IF EXISTS \"$db_name\" WITH (FORCE)" \
        >/dev/null 2>&1
}

cleanup() {
    if [ -n "$server_pid" ]; then
        kill "$server_pid" >/dev/null 2>&1 || true
        wait "$server_pid" >/dev/null 2>&1 || true
    fi

    drop_integration_db || true
}

trap cleanup EXIT INT TERM

cd "$project_root"

if [ "$run_migrations" = "1" ]; then
    # `diesel database setup` creates the integration database if it does not
    # exist yet, then runs pending migrations. It is idempotent on subsequent
    # runs.
    diesel database setup
    psql "$integration_database_url" \
        --set ON_ERROR_STOP=1 \
        --file tests/fixtures/integration-bootstrap.sql
fi

if [ "$start_api_server" = "1" ]; then
    existing_status=$(curl -s -o /dev/null -w '%{http_code}' "$api_base_url/api/health" || true)

    if [ "$existing_status" = "200" ]; then
        echo "API server already responds at $api_base_url; stop it or set START_API_SERVER=0" >&2
        exit 1
    fi

    cargo run -p poprako-server --bin poprako-server &
    server_pid="$!"

    health_ok=0
    health_attempt=1

    while [ "$health_attempt" -le 60 ]; do
        if ! kill -0 "$server_pid" >/dev/null 2>&1; then
            wait "$server_pid" || true
            echo "API server process exited before becoming healthy" >&2
            exit 1
        fi

        status=$(curl -s -o /dev/null -w '%{http_code}' "$api_base_url/api/health" || true)

        if [ "$status" = "200" ]; then
            health_ok=1
            break
        fi

        health_attempt=$((health_attempt + 1))
        sleep 1
    done

    if [ "$health_ok" != "1" ]; then
        echo "API server did not become healthy at $api_base_url" >&2
        exit 1
    fi
fi

cd "$project_root/tests/integration-tests"

deno task api
