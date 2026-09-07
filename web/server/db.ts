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
