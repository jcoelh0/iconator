# Mainmatter Rust Assessment

## Context

This repository contains a simple file-icon lookup library as may be used by IDEs for their project tree sidebar. `get_icon_for_file` and `get_icon_for_folder` accept a filepath and return the numeric ID of a corresponding icon if one is found. Each icon ID corresponds to an svg icon in `./icons`.

## Assignment

1. Implement a web server than allows users to query for file/folder icons over the network. 
2. Write a short (<= 1 page) design/implementation note describing:
  - Design choices and tradeoffs you made (high-level architecture, web server choice, libraries you picked etc.)
  - How you would productionize this service (deployment, observability, etc.)
  
Keep your audience in mind. We’re not interested in a perfect solution but one where we see your thought-process and working style in action.
  
You may add these notes at the end of this file if you wish.


## Solution

### Design Choices & Tradeoffs

- Picked `axum` on `tokio` for minimal boilerplate, type-safe extractors (`Path`, `Query`, `Json`), and compatibility with the `tower` middleware ecosystem, paired with `axum-test` for HTTP integration tests.
- Split the API into two steps: `/file-icon` and `/folder-icon` return JSON (`{"id": 525}`) while `/icon/{id}` serves the SVG. Returning the ID instead of the raw SVG lets clients deduplicate requests when 100 `.rs` files in a project tree all map to `/icon/525`.
- Error handling uses `thiserror` (`EngineError` and `ApiError`) instead of `anyhow` so failure modes are explicit enums that map directly to HTTP status codes and consistent JSON responses (`{"error": "..."}`).
- Didn't add `Arc<AppState>` since the service has no mutable runtime state; both the FST lookup maps and SVG bytes are immutable and embedded at compile time, avoiding state-passing overhead or locks.
- Considered pagination, but each query resolves a single path to one icon ID. For an IDE loading a full project tree at once, a `POST /icons/batch` endpoint with a bounded list of paths (e.g. max 500) is a much better fit to avoid hundreds of round-trips.
- Replaced `.unwrap()` with `?` on `path.file_name()` and `path.extension()` in `get_icon_for_file` and `get_icon_for_folder` so extensionless files (`Makefile`, `LICENSE`) or paths without a basename (`/`, `..`, empty) return `None` (`404`) instead of panicking.
- Kept lookups case-sensitive rather than lowercasing inputs at runtime. Most entries in `lookup_table.rs` are lowercase (`license`), so `LICENSE` returns `404`, but a few entries are mixed-case (`BUILD.bazel`, `Earthfile`, `Pulumi.yaml`) and would break if inputs were blindly lowercased; this should be solved by normalizing keys in `build.rs` or falling back to lowercase after an exact match.
- Embedded all SVGs into the binary at compile time (`icons_data.bin` and an 8-byte-per-entry `icons_index.bin` with `(offset, len)` pairs) just like the FST tables. This removes runtime disk I/O, gives $O(1)$ memory slice lookups, and lets `/icon/{id}` serve `Cache-Control: public, max-age=31536000, immutable` so clients and CDNs fetch each SVG only once.

### Testing
- 7 unit tests in `src/lib.rs` covering common extensions, folders, extensionless files, embedded SVG lookups, case sensitivity, and panic safety on malformed paths (`""`, `"/"`, `".."`).
- 11 HTTP integration tests in `src/controllers/lookup.rs` covering `200` lookups, trailing slashes on folders, `404` unknown files/folders/IDs, `400` missing query params, fallback routes, `/health`, and `Cache-Control` headers.

### Productionizing: Deployment, Observability & Traffic Surges

- **Deployment**: With FST maps and SVGs compiled into the binary, the service ships as a single static executable in a distroless container, with `/health` for Kubernetes probes, `Ctrl+C` / `SIGTERM` graceful shutdown draining in-flight requests, and bind address/port configured via environment variables.
- **Observability**: Replace `env_logger` with `tracing` and `tower-http`'s `TraceLayer` for structured JSON logs and OpenTelemetry spans, plus Prometheus metrics for latency, throughput, and `404` miss rates (which also highlights missing extensions in `lookup_table.rs`).
- **Handling a Big Traffic Surge**: Lookups are lock-free and served straight from memory, so a single instance already handles high throughput. For a massive surge: put a CDN in front of `/icon/{id}` (absorbing SVG bandwidth via `Cache-Control: immutable`), add the `POST /icons/batch` endpoint, enable `CompressionLayer` (Gzip/Brotli shrinks SVGs by ~70–80%) plus timeouts and rate limiting, and autoscale stateless replicas horizontally.

### Product, Team & Business Evolution

- **Supporting teammates who don't know Rust**: Editing `icons/lookup_table.rs` requires keeping the Rust array strictly sorted (or `fst::Map` panics at build time) and recompiling. For designers or frontend engineers, `build.rs` should read from a simple `icons.json` validated in CI, and the server should publish an OpenAPI spec to generate typed clients without touching Rust.
- **Preparing for business logic changes**: Compile-time embedding is why `get_icon_for_file`, `get_icon_for_folder`, and `get_icon_svg` are synchronous today (they only slice memory and never wait on I/O). If the product adds custom icon themes or live updates without redeploying, we would move assets to external storage (S3 + Redis/in-memory cache), make those lookup functions `async` so I/O doesn't block Tokio threads, and scope routes by theme/version (e.g. `/v1/themes/{theme}/file-icon`).
- **Framing Cost vs. Risk for Clients**: Choosing Rust carries a higher onboarding cost than other languages, but removes the risk of GC latency spikes, runtime crashes and high cloud bills (one 10 MB binary runs on a cheap VM). Similarly, a client can skip paying for batching, external storage, or full tracing to ship an MVP faster, as long as they know the concrete risks they accept.

### AI-Assisted Workflow Notes
- **Where AI sped things up**: Scaffolding the Axum routes, `thiserror` (`ApiError` / `EngineError`) definitions, serde request/response structs, `axum-test` integration tests, and the `Ctrl+C` / `SIGTERM` graceful shutdown boilerplate.
- **Where human judgment was needed**:
  - AI suggested wrapping Axum's `Path` and `Query` in custom `ValidatedPath` / `ValidatedQuery` extractors and a custom `ApiResult<T>` alias; I removed them since standard `Path`/`Query` and explicit `Result<T, ApiError>` signatures are simpler for any engineer to read.
  - AI initially suggested lowercasing all lookup inputs at runtime so `LICENSE` would match `license`, which would have broken mixed-case keys (`BUILD.bazel`, `Earthfile`) and added per-request allocations.
  - In `build.rs`, AI initially used two loops and an intermediate `Vec<(u32, u32)>` to build the SVG index, which was less efficient than just zero-initializing the slice (`vec![0u8; (max_id + 1) * 8]`)and using one loop instead.



