# Linkly

Linkly is a full-stack URL-shortening app built with Rust, Axum, PostgreSQL, Redis, React, TypeScript, and Tailwind CSS. It includes account registration/login, owned short links, optional aliases and expiry, 302 redirects, link management, and privacy-conscious click analytics.

## Run locally with Docker Compose

Requirements: Docker with the Compose plugin.

```sh
cp .env.example .env
docker compose up --build
```

Open the dashboard at <http://localhost:5173>. The API listens at <http://localhost:8080>; Compose runs the SQLx migrations at API startup. The example environment values are for local development only. Replace `JWT_SECRET` and `POSTGRES_PASSWORD` before exposing the service, and set `BASE_URL`, `FRONTEND_ORIGIN`, and `VITE_API_URL` to the public URLs for your deployment.

Useful commands:

```sh
docker compose logs -f api
docker compose down
```

`docker compose down -v` also removes the local database and Redis volumes.

## Deploy the frontend to Vercel

The React/Vite frontend can be deployed to Vercel as a static site. The Rust/Axum API is a separate long-running service in this project, so deploy it to a host that supports Rust services (for example, Railway, Render, or Fly.io) and use managed PostgreSQL and Redis instances. Do not use the local database credentials in production.

1. Deploy the API and configure `DATABASE_URL`, `REDIS_URL`, `JWT_SECRET` (at least 32 bytes), `BASE_URL`, and `FRONTEND_ORIGIN`. `BASE_URL` should be the public API origin used to form short links. `FRONTEND_ORIGIN` should be the Vercel site's exact origin, such as `https://your-project.vercel.app`.
2. Import the repository into Vercel, or deploy with the Vercel CLI from the `frontend` directory. Set the project **Root Directory** to `frontend`; Vercel should detect Vite. Use `npm run build` as the build command and `dist` as the output directory.
3. Add `VITE_API_URL` in the Vercel project environment variables, set to the public API origin (for example, `https://your-api.example.com`), then deploy or redeploy. This value is embedded in the frontend at build time.
4. Open the Vercel URL and test registration, login, link creation, and redirects. If you add a custom frontend domain, update the API's `FRONTEND_ORIGIN` to match it.

See Vercel's [Vite deployment guide](https://vercel.com/docs/frameworks/frontend/vite) for Git-based and CLI deployment options. The API would need to be adapted to Vercel's function runtime to run there instead of on a separate service.

## Run without Docker

Create the local PostgreSQL role and database, then copy `.env.example` to `.env`:

```sh
sudo -u postgres psql -d postgres -v ON_ERROR_STOP=1 -f scripts/setup-local-postgres.sql
cp .env.example .env
```

The setup script uses the development-only `rusturl` password to match `DATABASE_URL`; do not use it outside local development. Replace the password in both the SQL script and `.env` with a strong value before sharing the database or exposing it to a network. The API applies the schema migrations automatically on startup.

Start Redis separately if you want caching (redirects still work without it), then run the API and frontend in separate terminals:

```sh
cargo run
```

```sh
cd frontend
cp .env.example .env
npm install
npm run dev
```

The frontend requires Node.js 20.19+. Set `DATABASE_URL`, `REDIS_URL`, `JWT_SECRET` (at least 32 bytes), `BASE_URL`, and `FRONTEND_ORIGIN` for the API. Redis is optional at runtime: cache connection failures are logged and URL resolution falls back to PostgreSQL.

## API

All JSON request and response bodies use `application/json`.

| Method | Path | Auth | Purpose |
|---|---|---:|---|
| `POST` | `/api/auth/register` | No | Register with `{ "email", "password" }`; returns a bearer token |
| `POST` | `/api/auth/login` | No | Sign in with `{ "email", "password" }` |
| `GET` | `/api/urls` | Yes | List the signed-in user's links and click totals |
| `POST` | `/api/urls` | Yes | Create `{ "original_url", "custom_alias?", "expires_at?" }` |
| `DELETE` | `/api/urls/{code}` | Yes | Delete an owned short link |
| `GET` | `/api/urls/{code}/analytics` | Yes | Return total clicks and daily counts |
| `GET` | `/{code}` | No | Redirect to an active destination with HTTP 302 |
| `GET` | `/health` | No | Liveness endpoint |

Send protected requests with `Authorization: Bearer <token>`. Destinations must be HTTP(S) URLs. Aliases are 3–32 ASCII letters, numbers, hyphens, or underscores. Expired and inactive links return `410 Gone`; unknown links return `404 Not Found`. Analytics store click timestamps only; visitor IP addresses and user agents are not collected.

## Design notes

- PostgreSQL is authoritative. Redis caches active code-to-destination mappings for up to five minutes, shortened to the link's remaining lifetime; cache failures do not prevent redirects.
- A unique database constraint prevents short-code collisions. Generated codes retry on collision; custom-alias collisions return `409 Conflict`.
- Passwords use Argon2id defaults from the Rust `argon2` crate. Bearer tokens are signed JWTs with a seven-day expiry.
- The API applies a 16 KiB request-body limit and only permits the frontend origin and required HTTP methods in CORS.
- Users can only list, inspect, and delete their own links. Redirects are public.
- Click rows are inserted during redirect handling and aggregate queries use indexed URL/time fields. Analytics failures are logged without preventing a valid redirect.

## Tests

```sh
cargo test
```

The API tests cover destination URL and alias validation. The database migration is applied automatically at startup; integration testing against PostgreSQL/Redis can be added as the service grows.
