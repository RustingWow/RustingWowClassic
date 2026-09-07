import path from "node:path";
import { fileURLToPath } from "node:url";

import cookie from "@fastify/cookie";
import staticFiles from "@fastify/static";
import Fastify from "fastify";

import "./env.ts";
import { login, logout, me, register } from "./auth.ts";
import { pool } from "./db.ts";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const isProduction = process.env.NODE_ENV === "production";
const port = Number(process.env.PORT ?? 3000);

const app = Fastify({ logger: true });

await app.register(cookie);

app.setErrorHandler((error, _request, reply) => {
  app.log.error(error);
  const message = error instanceof Error ? error.message : "Server error";
  const status = message.includes("auth.accounts is missing") ? 503 : 500;
  void reply.code(status).send({ error: message });
});

app.get("/api/health", async () => ({ ok: true }));
app.post("/api/register", register);
app.post("/api/login", login);
app.post("/api/logout", logout);
app.get("/api/me", me);

if (isProduction) {
  const dist = path.join(root, "dist");
  await app.register(staticFiles, {
    root: dist,
    wildcard: false,
  });
  app.setNotFoundHandler((request, reply) => {
    if (request.method !== "GET" || request.url.startsWith("/api")) {
      return reply.code(404).send({ error: "Not found" });
    }
    return reply.sendFile("index.html");
  });
} else {
  const middie = (await import("@fastify/middie")).default;
  const { createServer } = await import("vite");
  await app.register(middie);
  const vite = await createServer({
    root,
    server: { middlewareMode: true },
    appType: "spa",
  });
  app.use((req, res, next) => {
    if (req.url?.startsWith("/api")) {
      next();
      return;
    }
    vite.middlewares(req, res, next);
  });
}

await app.listen({ port, host: "0.0.0.0" });

let shuttingDown = false;
async function shutdown() {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;
  await app.close();
  await pool.end();
}

process.on("SIGINT", () => {
  void shutdown();
});
process.on("SIGTERM", () => {
  void shutdown();
});
