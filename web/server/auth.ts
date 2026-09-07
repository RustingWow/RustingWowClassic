import argon2 from "argon2";
import type { FastifyReply, FastifyRequest } from "fastify";
import jwt from "jsonwebtoken";

import { assertAuthSchema, pool } from "./db.ts";
import { calculateVerifier, generateSalt, normalizeUsername } from "./srp6.ts";

export const SESSION_COOKIE = "wow_session";
const JWT_SECRET = process.env.JWT_SECRET ?? "dev-wowserver-jwt-secret";

type TokenPayload = {
  sub: number;
  username: string;
};

type AuthBody = {
  username?: string;
  password?: string;
  email?: string;
};

export async function register(
  request: FastifyRequest<{ Body: AuthBody }>,
  reply: FastifyReply,
) {
  const username = normalizeUsername(String(request.body?.username ?? ""));
  const password = String(request.body?.password ?? "");
  const emailRaw = request.body?.email;
  const email =
    typeof emailRaw === "string" && emailRaw.trim() !== ""
      ? emailRaw.trim()
      : null;

  if (!username) {
    return reply
      .code(400)
      .send({ error: "Username must be 2–16 letters or digits." });
  }
  if (password.length < 4 || password.length > 16) {
    return reply.code(400).send({ error: "Password must be 4–16 characters." });
  }
  if (email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    return reply.code(400).send({ error: "Email is not valid." });
  }

  await assertAuthSchema();
  const salt = generateSalt();
  const verifier = calculateVerifier(username, password, salt);
  const passwordHash = await argon2.hash(password);

  try {
    const inserted = await pool.query<{ id: string }>(
      `INSERT INTO auth.accounts
        (username, email, password_hash, srp_salt, srp_verifier)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING id`,
      [username, email, passwordHash, salt, verifier],
    );
    const id = Number(inserted.rows[0].id);
    setSession(reply, { sub: id, username });
    return reply.code(201).send({ id, username, email });
  } catch (error) {
    if (isUniqueViolation(error)) {
      return reply.code(409).send({ error: "That username is already taken." });
    }
    throw error;
  }
}

export async function login(
  request: FastifyRequest<{ Body: AuthBody }>,
  reply: FastifyReply,
) {
  const username = normalizeUsername(String(request.body?.username ?? ""));
  const password = String(request.body?.password ?? "");
  if (!username || password.length === 0) {
    return reply
      .code(400)
      .send({ error: "Username and password are required." });
  }

  await assertAuthSchema();
  const result = await pool.query<{
    id: string;
    username: string;
    email: string | null;
    password_hash: string;
    locked: boolean;
  }>(
    `SELECT id, username, email, password_hash, locked
     FROM auth.accounts
     WHERE username = $1`,
    [username],
  );
  const account = result.rows[0];
  if (!account) {
    return reply.code(401).send({ error: "Invalid username or password." });
  }
  if (account.locked) {
    return reply.code(403).send({ error: "This account is locked." });
  }
  const ok = await argon2.verify(account.password_hash, password);
  if (!ok) {
    return reply.code(401).send({ error: "Invalid username or password." });
  }

  const payload = { sub: Number(account.id), username: account.username };
  setSession(reply, payload);
  return {
    id: payload.sub,
    username: account.username,
    email: account.email,
  };
}

export async function logout(_request: FastifyRequest, reply: FastifyReply) {
  return reply
    .clearCookie(SESSION_COOKIE, { path: "/" })
    .code(204)
    .send();
}

export async function me(request: FastifyRequest, reply: FastifyReply) {
  const token = request.cookies[SESSION_COOKIE];
  if (!token) {
    return reply.code(401).send({ error: "Not signed in." });
  }
  let payload: TokenPayload;
  try {
    payload = jwt.verify(token, JWT_SECRET) as TokenPayload;
  } catch {
    return reply.code(401).send({ error: "Not signed in." });
  }

  await assertAuthSchema();
  const result = await pool.query<{
    id: string;
    username: string;
    email: string | null;
  }>(`SELECT id, username, email FROM auth.accounts WHERE id = $1`, [
    payload.sub,
  ]);
  const account = result.rows[0];
  if (!account) {
    return reply.code(401).send({ error: "Not signed in." });
  }
  return {
    id: Number(account.id),
    username: account.username,
    email: account.email,
  };
}

function setSession(reply: FastifyReply, payload: TokenPayload): void {
  const token = jwt.sign(payload, JWT_SECRET, { expiresIn: "7d" });
  reply.setCookie(SESSION_COOKIE, token, {
    httpOnly: true,
    sameSite: "lax",
    path: "/",
    maxAge: 7 * 24 * 60 * 60,
  });
}

function isUniqueViolation(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    (error as { code: string }).code === "23505"
  );
}
