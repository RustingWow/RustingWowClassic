# WoWServer

Emulatore backend Vanilla **1.12.1** (build 5875) in Rust, spezzato in due microservizi:

- `auth-server` — login SRP6 sulla porta **3724** e API interna HTTP
- `world-server` — sessione world sulla porta **8085**, elenco personaggi e spawn

Nessun database. Le credenziali valide sono le coppie `userN` / `passN` (`user1`/`pass1`, `user42`/`pass42`, …). Ogni account ha un Human Warrior `UserN` a Northshire Abbey.

## Prerequisiti

- [rustup](https://rustup.rs) — il file `rust-toolchain.toml` installa Rust **1.98.1** al primo `cargo`

```bash
rustup update
cargo build --workspace
```

## Avvio

Due terminali, prima Auth poi World:

```bash
cargo run -p auth-server
cargo run -p world-server
```

Smoke test senza client Blizzard:

```bash
cargo run -p test-client -- user1 pass1
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

Login: `user1` / `pass1`. Il realm si chiama **WoWServer**.

## Variabili d'ambiente

| Variabile | Default | Servizio |
| --- | --- | --- |
| `AUTH_BIND` | `0.0.0.0:3724` | auth-server |
| `AUTH_INTERNAL_BIND` | `127.0.0.1:9090` | auth-server |
| `WORLD_PUBLIC_ADDR` | `127.0.0.1:8085` | auth-server (indirizzo nel realm list) |
| `WORLD_BIND` | `0.0.0.0:8085` | world-server |
| `AUTH_INTERNAL_URL` | `http://127.0.0.1:9090` | world-server |
| `LOG_UNHANDLED_PACKETS` | `false` | world-server (logga i pacchetti client ricevuti e non gestiti: `1`/`true`/`yes`/`on`) |

## Test

```bash
cargo test --workspace
```
