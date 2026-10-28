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

### How to Run
```bash
cargo run                 # starts server on 0.0.0.0:3000 (or set PORT)
cargo test                # runs unit and HTTP integration tests
```
Example requests (also available in `example.http`):
```bash
curl "http://localhost:3000/file-icon?path=my-app/src/main.rs"
curl "http://localhost:3000/folder-icon?path=my-app/src"
curl "http://localhost:3000/icon/525"
```

### Endpoints
- `GET /file-icon?path=my-app/src/main.rs` -> `{"id": 525}` (or `404` JSON error)
- `GET /folder-icon?path=my-app/src` -> `{"id": 1054}` (or `404` JSON error)
- `GET /icon/{id}` -> `image/svg+xml` with `Cache-Control: public, max-age=31536000, immutable`
- `GET /health` -> `{"status": "ok"}`

### Design Choices & Tradeoffs

- **Web framework**: Picked `axum` for minimal boilerplate, type-safe extractors (`Path`, `Query`, `Json`), and compatibility with the `tower` middleware ecosystem, paired with `axum-test` for HTTP integration tests.
- **Two-step lookup API**: `/file-icon` and `/folder-icon` return JSON (`{"id": 525}`) while `/icon/{id}` serves the SVG. Returning the ID instead of the raw SVG lets clients deduplicate requests when 100 `.rs` files in a project tree all map to `/icon/525`.
- **Typed error handling**: I preferred using `thiserror` (`EngineError` and `ApiError`) instead of `anyhow` so failure modes are explicit enums that map directly to HTTP status codes and JSON responses (`{"error": "..."}`). For Axum's built-in extractor rejections (`400` on missing `?path=` or non-numeric `/icon/abc`, which default to `text/plain`), I would add a single `axum::middleware::map_response` layer in production to wrap any plain-text `4xx`/`5xx` body into JSON rather than cluttering handlers with custom wrapper extractors.
- **Panic safety on edge-case paths**: Replaced `.unwrap()` with `?` on `path.file_name()` and `path.extension()` in `get_icon_for_file` and `get_icon_for_folder` so extensionless files (`Makefile`, `LICENSE`) or paths without a basename (`/`, `..`, empty) return `None` (`404`) instead of panicking.
- **Compile-time SVG embedding & caching**: Embedded all SVGs into the binary at compile time (`icons_data.bin` for SVGs data and an 8-byte per entry `icons_index.bin` with `(offset, len)` pairs) just like the FST tables. This removes runtime disk I/O, gives O(1) memory slice lookups and lets `/icon/{id}` serve `Cache-Control: public, max-age=31536000, immutable` so clients and CDNs fetch each SVG only once.
- **Case sensitivity**: Kept lookups case-sensitive rather than lowercasing inputs at runtime. Most entries in `lookup_table.rs` are lowercase (`license`), so `LICENSE` returns `404`, but a few entries have capital letters (`BUILD.bazel`, `Earthfile`, `Pulumi.yaml`) and would break if inputs were blindly lowercased. This should be solved by lowercasing all table keys in `build.rs` or falling back to lowercase after an exact match.
- **No shared state (`Arc<AppState>`)**: Omitted since the service has no mutable runtime state. Both the FST lookup maps and SVG bytes are immutable and embedded at compile time, avoiding state-passing overhead or locks.
- **Batching over pagination**: Each query resolves a single path to one icon ID, so pagination does not fit. For an IDE loading a full project tree at once, a `POST /icons/batch` endpoint with a bounded list of paths (e.g. max 500) is a much better fit to avoid hundreds of round-trips.

### Testing
- 7 unit tests in `src/lib.rs` covering common extensions, folders, extensionless files, embedded SVG lookups, case sensitivity, and panic safety on malformed paths (`""`, `"/"`, `".."`).
- 11 HTTP integration tests in `src/controllers/lookup.rs` covering `200` lookups, trailing slashes on folders, `404` unknown files/folders/IDs, `400` missing query params, fallback routes, `/health`, and `Cache-Control` headers.

### Productionizing: Deployment, Observability & Traffic Surges

- With FST maps and SVGs compiled into the binary, the service ships as a single static executable in a distroless container, with `/health` for Kubernetes probes, `Ctrl+C` / `SIGTERM` graceful shutdown draining in-flight requests and bind address/port configured via environment variables.
- Replace `env_logger` with `tracing` and `tower-http`'s `TraceLayer` for structured JSON logs and OpenTelemetry spans, plus Prometheus metrics for latency, throughput, and `404` miss rates (which also highlights missing extensions in `lookup_table.rs`).
- Lookups are lock-free and served straight from memory, so a single instance already handles high throughput. For a big surge I would put a CDN in front of `/icon/{id}` (absorbing SVG bandwidth via `Cache-Control: immutable`), add the `POST /icons/batch` endpoint, enable `CompressionLayer` (Gzip/Brotli shrinks SVGs significantly) plus timeouts and rate limiting and autoscale stateless replicas horizontally.

### Product, Team & Business Evolution

- **If teammates don't know Rust**: Right now, adding an icon means editing a Rust file (`icons/lookup_table.rs`) that must stay in strict alphabetical order or the build panics. Moving that mapping to a simple `icons.json` file lets designers or frontend devs add icons without touching Rust, while keeping the server code free of custom macros and exposing an OpenAPI spec makes it easy to work with.
- **Preparing for business logic changes**: Compile-time embedding is why `get_icon_for_file`, `get_icon_for_folder`, and `get_icon_svg` are synchronous today (they only slice memory and never wait on I/O). If the product adds custom icon themes or live updates without redeploying, we would move assets to external storage (S3 + Redis/in-memory cache) and make those lookup functions `async` so I/O doesn't block Tokio threads.
- **Framing Cost vs. Risk for Clients**: Choosing Rust carries a higher onboarding cost than other languages, but catches whole classes of bugs at compile time and eliminates GC latency spikes and high cloud bills (one small binary runs on a cheap VM). Similarly, a client can skip paying for batching, external storage, or full tracing to ship an MVP faster, as long as they know the concrete risks they accept.

### AI-Assisted Workflow Notes
- **Where AI sped things up**: Scaffolding the Axum routes, `thiserror` (`ApiError` / `EngineError`) definitions, serde request/response structs, `axum-test` integration tests, and the `Ctrl+C` / `SIGTERM` graceful shutdown boilerplate.
- **Where I needed to intervene**:
  - AI suggested wrapping Axum's `Path` and `Query` in custom `ValidatedPath` / `ValidatedQuery` extractors and a custom `ApiResult<T>` alias; I removed them since standard `Path`/`Query` and explicit `Result<T, ApiError>` signatures are simpler for any engineer to read.
  - AI suggested adding a runtime lowercase fallback (`exact_match.or_else(|| lowercase_match)`) so `LICENSE` would match `license` without breaking capitalized keys (`BUILD.bazel`, `Earthfile`). I chose to keep the library's exact-matching behavior unchanged, add a unit test for it, and document lowercasing the table keys in `build.rs` instead.
  - In `build.rs`, AI initially used two loops and an intermediate `Vec<(u32, u32)>` to build the SVG index, which was less efficient than just zero-initializing the slice (`vec![0u8; (max_id + 1) * 8]`) and using one loop instead.



