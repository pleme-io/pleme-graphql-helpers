# pleme-graphql-helpers

GraphQL utilities library for Pleme platform - schema helpers, federation, pagination

## Installation

```toml
[dependencies]
pleme-graphql-helpers = "0.1"
```

## Usage

```rust
use pleme_graphql_helpers::{pagination::CursorPagination, guards::AuthGuard};

#[Object]
impl Query {
    #[graphql(guard = "AuthGuard::new()")]
    async fn users(&self, ctx: &Context<'_>, pagination: CursorInput) -> Result<Connection<User>> {
        CursorPagination::paginate(ctx, pagination).await
    }
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `errors` | pleme-error integration |
| `full` | All features enabled |

Enable features in your `Cargo.toml`:

```toml
pleme-graphql-helpers = { version = "0.1", features = ["full"] }
```

## Development

This project uses [Nix](https://nixos.org/) for reproducible builds:

```bash
nix develop            # Dev shell with Rust toolchain
nix run .#check-all    # cargo fmt + clippy + test
nix run .#publish      # Publish to crates.io (--dry-run supported)
nix run .#regenerate   # Regenerate Cargo.nix
```

## License

MIT - see [LICENSE](LICENSE) for details.
