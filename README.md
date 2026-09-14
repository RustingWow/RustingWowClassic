# WoWServer

Emulatore backend Vanilla **1.12.1** (build 5875) in Rust, con portale di registrazione e PostgreSQL condiviso.

- `auth-server` — login SRP6 sulla porta **3724**, API interna HTTP (sessioni world + creazione/verifica account), schema Postgres `auth`
- `world-server` — sessione world sulla porta **8085** (ruoli combined/gateway/map)
- `web` — portale (home, news, registrazione, guida) sulla porta **3000**; scrive sullo stesso `auth.accounts` usato dal gioco

Il portale e l’auth-server condividono Postgres: registrazione e login web funzionano anche se l’auth-server è spento. Per il client 1.12.1 l’auth-server deve essere in ascolto sulla 3724. Lo schema `auth` lo crea sqlx al primo avvio dell’auth-server; lo schema `web` (idee, voti, commenti) lo crea il portale all’avvio.

Le tabelle si creano da sole al primo avvio (sqlx, equivalente Flyway). Redis resta opzionale per le sessioni live, non per gli account.

Il world-server crea lo schema `world` vuoto (creature, item, vendor, loot, gossip, quest). I dati CMaNGOS classic-db stanno in `db/*.sql.gz` e **non** partono con `migrate!`: dopo il primo avvio del world-server, carica il catalogo a mano:

```bash
export DATABASE_URL=postgres://wow:wow@127.0.0.1:5432/wow
./db/load.sh
```

Poi riavvia il world-server. Senza quel load restano solo i due NPC demo di Northshire. Dettagli e re-convert: [`db/README.md`](db/README.md).

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
export AUTH_INTERNAL_TOKEN=dev-internal-token

cargo run -p auth-server
cargo run -p world-server

cd web && npm install && npm run dev
```

Il portale è su [http://127.0.0.1:3000](http://127.0.0.1:3000) (API e UI nello stesso processo). Registrati lì, poi usa **lo stesso username e password** nel client 1.12.1.

In produzione, Postgres + portale (l’auth-server va avviato a parte per il login in-game; lo schema va creato almeno una volta):

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

## DBC del client

I `.dbc` stanno negli MPQ del client (`Data/*.MPQ`). Sono materiale Blizzard: **non** vanno in git. Estraili una volta dalla tua installazione 1.12.1:

```bash
cargo run -p extract-dbc -- --client /path/to/WoW --out data/dbc
```

Oppure `WOW_CLIENT_PATH` e `DBC_DIR`. Funziona sia col client retail (`Data/enUS/locale-enUS.MPQ`) sia con pack tipo RetroWoW (`Data/dbc.MPQ`, senza cartella locale). Il world-server legge `data/dbc/FactionTemplate.dbc` all’avvio (ostilità / cursore spada). Se il file manca, usa le maschere builtin. Altri DBC restano su disco per usi successivi.

Vedi [`db/COPYRIGHT.md`](db/COPYRIGHT.md).

## Comandi GM

Il world-server intercetta i messaggi che iniziano con `.` o `!` (CMaNGOS Classic). Livelli: `0` player, `1` moderator, `2` GM, `3` admin. Dopo la migrazione auth, promuovi un account e **ri-loggati**:

```sql
UPDATE accounts SET gmlevel = 3 WHERE username = 'TUONOME';
```

`.help` elenca i comandi del tuo livello. Quelli non ancora supportati dal mondo rispondono `Command not implemented yet.`

## Variabili d'ambiente

| Variabile | Default | Servizio |
| --- | --- | --- |
| `DATABASE_URL` | *(obbligatoria)* | auth-server, world-server, portale |
| `AUTH_BIND` | `0.0.0.0:3724` | auth-server |
| `AUTH_INTERNAL_BIND` | `127.0.0.1:9090` | auth-server |
| `AUTH_INTERNAL_TOKEN` | *(opzionale)* | auth-server e world-server (header `X-Auth-Internal-Token`; il world lo usa per `.account set gmlevel`) |
| `WORLD_PUBLIC_ADDR` | `127.0.0.1:8085` | auth-server (indirizzo nel realm list) |
| `WORLD_BIND` | `0.0.0.0:8085` | world-server |
| `AUTH_INTERNAL_URL` | `http://127.0.0.1:9090` | world-server |
| `REDIS_URL` | *(opzionale)* | auth-server, world-server |
| `LOG_UNHANDLED_PACKETS` | `false` | world-server |
| `DBC_DIR` | `data/dbc` | world-server (cartella dei `.dbc` estratti) |
| `WOW_CLIENT_PATH` | *(obbligatoria per extract-dbc)* | `extract-dbc` (installazione 1.12.1) |
| `JWT_SECRET` | `dev-wowserver-jwt-secret` | portale web |
| `PORT` | `3000` | portale web |
| `DISCORD_INVITE_URL` | `https://discord.gg/your-invite` | portale web |
| `REALMLIST_HOST` | `127.0.0.1` | portale web (testo della guida) |
| `GITHUB_REPO` | `RustingWow/RustingWowClassic` | portale (changelog da GitHub Releases) |
| `GITHUB_CHANGELOG_BRANCH` | `develop` | portale (card Unreleased) |
| `GITHUB_TOKEN` | *(opzionale)* | portale (repo privato / rate limit) |

Redis opzionale: `export REDIS_URL=redis://127.0.0.1:6379` dopo `docker compose up -d redis`.

## Test

```bash
cargo test --workspace
cd web && npm test && npm run build
```

Il test `enter_world` semina `user1`/`pass1` in memoria e **non** richiede Postgres.

## Changelog e release

I commit conventional (`feat:`, `fix:`, …) sono la fonte delle note. L’agente Cursor li scrive in quel formato. Per pubblicare una card su [http://127.0.0.1:3000/changelog](http://127.0.0.1:3000/changelog):

```bash
git checkout develop
git tag v0.1.0
git push origin develop v0.1.0
```

Il workflow `.github/workflows/release.yml` genera le note con git-cliff e crea la GitHub Release. Il portale legge le release (e i commit ancora non taggati su `develop`) via API GitHub.
