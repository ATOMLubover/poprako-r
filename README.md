# poprako-server

PopRaKo manga translation project management backend, built with Rust, Axum,
Diesel, and PostgreSQL.

The service provides APIs for teams, comics, chapters, pages, translation
units, assignments, archives, and termbases. It is currently under active
development.

## Requirements

- Rust 1.95 with Rust 2024 edition support
- PostgreSQL and `diesel_cli`
- Cloudflare R2 credentials

Docker is additionally required for container builds. Deno 2.9 is required for
the TypeScript HTTP integration tests.

## Run locally

Create the local environment file:

```sh
cp .env.example .env
```

Set the PostgreSQL, JWT, and Cloudflare R2 values in `.env`, then initialize the
database and start the server:

```sh
diesel database setup
cargo run -p poprako-server --bin poprako-server
```

The API is served under `/api/v1` and listens on the address configured in
`app_config.toml`.

## Common commands

```sh
# Apply pending migrations
diesel migration run

# Run Rust tests
cargo test --workspace

# Run repository checks
cargo check --workspace --all-targets --all-features
sh fmt/run-check.sh

# Generate docs/swagger.json
cargo run -p poprako-swagger > docs/swagger.json
```

The `justfile` provides optional shortcuts for local development. It is not a
project requirement and is not used as the CI/CD interface.

The HTTP integration suite requires a dedicated disposable database:

```sh
INTEGRATION_DATABASE_URL=postgres://USER:PASSWORD@localhost:5432/db_poprako_integration \
  scripts/api-integration-test.sh
```

The integration script drops its target database after the run. Do not point it
at a database containing data that must be preserved.

## Documentation

- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Release policy](RELEASE.md)
- [HTTP integration test inventory](tests/integration-tests/TESTCASES.md)
- [Organization transfer checklist](docs/organization-transfer-readiness.md)
- [Agent and project conventions](AGENTS.md)
- [OpenAPI specification](docs/swagger.json)

## License

[MIT](LICENSE)
