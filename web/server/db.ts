import "./env.ts";
import pg from "pg";

const url = process.env.DATABASE_URL ?? "postgres://wow:wow@127.0.0.1:5432/wow";

export const pool = new pg.Pool({
  connectionString: url,
});

export async function assertAuthSchema(): Promise<void> {
  const result = await pool.query(
    `SELECT to_regclass('auth.accounts') AS table_name`,
  );
  if (!result.rows[0]?.table_name) {
    throw new Error(
      "auth.accounts is missing. Start auth-server first so sqlx can create the schema.",
    );
  }
}

export async function ensureWebSchema(): Promise<void> {
  await assertAuthSchema();
  await pool.query("CREATE SCHEMA IF NOT EXISTS web");
  await pool.query(`
    CREATE TABLE IF NOT EXISTS web.topics (
      id BIGSERIAL PRIMARY KEY,
      account_id BIGINT NOT NULL REFERENCES auth.accounts(id) ON DELETE CASCADE,
      title VARCHAR(120) NOT NULL,
      body TEXT NOT NULL,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      CONSTRAINT topics_title_len CHECK (char_length(title) BETWEEN 3 AND 120),
      CONSTRAINT topics_body_len CHECK (char_length(body) BETWEEN 1 AND 8000)
    )
  `);
  await pool.query(`
    CREATE TABLE IF NOT EXISTS web.votes (
      topic_id BIGINT NOT NULL REFERENCES web.topics(id) ON DELETE CASCADE,
      account_id BIGINT NOT NULL REFERENCES auth.accounts(id) ON DELETE CASCADE,
      value SMALLINT NOT NULL CHECK (value IN (-1, 1)),
      PRIMARY KEY (topic_id, account_id)
    )
  `);
  await pool.query(`
    CREATE TABLE IF NOT EXISTS web.comments (
      id BIGSERIAL PRIMARY KEY,
      topic_id BIGINT NOT NULL REFERENCES web.topics(id) ON DELETE CASCADE,
      account_id BIGINT NOT NULL REFERENCES auth.accounts(id) ON DELETE CASCADE,
      body TEXT NOT NULL,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      CONSTRAINT comments_body_len CHECK (char_length(body) BETWEEN 1 AND 2000)
    )
  `);
  await pool.query(`
    CREATE INDEX IF NOT EXISTS topics_created_at_idx
      ON web.topics (created_at DESC)
  `);
  await pool.query(`
    CREATE INDEX IF NOT EXISTS comments_topic_id_idx
      ON web.comments (topic_id, created_at)
  `);
}
