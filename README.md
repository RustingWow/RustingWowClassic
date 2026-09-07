# WoWServer

Emulatore backend Vanilla **1.12.1** (build 5875) in Rust, con portale di registrazione e PostgreSQL condiviso.

- `auth-server` — login SRP6 sulla porta **3724**, API interna HTTP, schema Postgres `auth`
- `world-server` — sessione world sulla porta **8085** (ruoli combined/gateway/map)
- `web` — portale unico (API + React) sulla porta **3000**; le stesse credenziali valgono nel client WoW

Le tabelle si creano da sole al primo avvio (sqlx, equivalente Flyway). Redis resta opzionale per le sessioni live, non per gli account.

## Prerequisiti

- [rustup](https://rustup.rs) — `rust-toolchain.toml` installa Rust **1.98.1**
- [Docker](https://www.docker.com/) per Postgres (e Redis, se serve il cluster)
- Node.js 20+ per il portale

```bash
rustup update
docker compose up -d postgres
cargo build --workspace
```

## Avvio

Esporta l’URL del database, poi Auth, World e il portale:

```bash
export DATABASE_URL=postgres://wow:wow@127.0.0.1:5432/wow

cargo run -p auth-server
cargo run -p world-server

cd web && npm install && npm run dev
```

Il portale è su [http://127.0.0.1:3000](http://127.0.0.1:3000) (API e UI nello stesso processo). Registrati lì, poi usa **lo stesso username e password** nel client 1.12.1.

In produzione, un solo container:

```bash
docker compose up -d postgres portal
```

Smoke test (serve un account già registrato, oppure il seed in-memory usato solo dai test):

```bash
cargo run -p test-client -- ALICE secret123
```

Health check interno Auth:

```bash
curl http://127.0.0.1:9090/health
```

## Client ufficiale 1.12.1

In `realmlist.wtf` (o `WTF/Config.wtf`):

```
set realmlist 127.0.0.1
```

Il realm si chiama **WoWServer**. Username: 2–16 caratteri alfanumerici (il gioco li tratta in maiuscolo).

## Variabili d'ambiente

| Variabile | Default | Servizio |
| --- | --- | --- |
| `DATABASE_URL` | *(obbligatoria)* | auth-server, world-server, portale |
| `AUTH_BIND` | `0.0.0.0:3724` | auth-server |
| `AUTH_INTERNAL_BIND` | `127.0.0.1:9090` | auth-server |
| `WORLD_PUBLIC_ADDR` | `127.0.0.1:8085` | auth-server (indirizzo nel realm list) |
| `WORLD_BIND` | `0.0.0.0:8085` | world-server |
| `AUTH_INTERNAL_URL` | `http://127.0.0.1:9090` | world-server |
| `REDIS_URL` | *(opzionale)* | auth-server, world-server |
| `LOG_UNHANDLED_PACKETS` | `false` | world-server |
| `JWT_SECRET` | `dev-wowserver-jwt-secret` | portale web |
| `PORT` | `3000` | portale web |

Redis opzionale: `export REDIS_URL=redis://127.0.0.1:6379` dopo `docker compose up -d redis`.

## Test

```bash
cargo test --workspace
cd web && npm test
```

Il test `enter_world` semina `user1`/`pass1` in memoria e **non** richiede Postgres.
