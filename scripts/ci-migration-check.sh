#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if [ "${CI_MIGRATION_DATABASE:-}" != "1" ]; then
    echo "CI_MIGRATION_DATABASE=1 is required for destructive migration checks" >&2
    exit 1
fi

case "${DATABASE_URL:-}" in
    postgres://*/db_poprako_ci | postgresql://*/db_poprako_ci)
        ;;
    *)
        echo "DATABASE_URL must target the dedicated db_poprako_ci database" >&2
        exit 1
        ;;
esac

if ! command -v diesel >/dev/null 2>&1; then
    cargo install \
        --locked \
        --no-default-features \
        --features postgres \
        --version 2.3.7 \
        diesel_cli
fi

diesel migration run --config-file /dev/null

# CD replays every up.sql in one transaction, without Diesel's history table.
# Exercise that path on an existing schema so a second deployment cannot fail
# on an already-created table, index, or seed row.
command -v psql >/dev/null 2>&1 || {
    echo "psql is required for production migration replay checks" >&2
    exit 1
}

migration_batch=$(mktemp)
trap 'rm -f "$migration_batch"' EXIT
trap 'exit 1' INT TERM

LC_ALL=C
export LC_ALL

{
    printf 'BEGIN;\n'

    for migration_dir in migrations/*; do
        [ -d "$migration_dir" ] || continue

        cat "$migration_dir/up.sql"
        printf '\n'
    done

    printf 'COMMIT;\n'
} >"$migration_batch"

psql "$DATABASE_URL" --no-psqlrc --set ON_ERROR_STOP=1 --file "$migration_batch"
psql "$DATABASE_URL" --no-psqlrc --set ON_ERROR_STOP=1 --file "$migration_batch"

diesel migration revert --all --config-file /dev/null
diesel migration run --config-file /dev/null
