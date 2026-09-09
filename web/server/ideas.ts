import type { FastifyReply, FastifyRequest } from "fastify";

import { optionalAccount, requireAccount } from "./auth.ts";
import { pool } from "./db.ts";

type IdParams = { id: string };

type TopicBody = {
  title?: string;
  body?: string;
};

type VoteBody = {
  value?: number;
};

type CommentBody = {
  body?: string;
};

const TITLE_MIN = 3;
const TITLE_MAX = 120;
const BODY_MAX = 8000;
const COMMENT_MAX = 2000;

type TopicRow = {
  id: string;
  account_id: string;
  username: string;
  title: string;
  body: string;
  created_at: Date;
  updated_at: Date;
  score: string | number;
  comment_count: string | number;
};

type CommentRow = {
  id: string;
  account_id: string;
  username: string;
  body: string;
  created_at: Date;
};

export async function listTopics() {
  const result = await pool.query<TopicRow>(
    `SELECT
        t.id,
        t.account_id,
        a.username,
        t.title,
        t.body,
        t.created_at,
        t.updated_at,
        COALESCE((SELECT SUM(value) FROM web.votes WHERE topic_id = t.id), 0) AS score,
        (SELECT COUNT(*) FROM web.comments WHERE topic_id = t.id) AS comment_count
     FROM web.topics t
     JOIN auth.accounts a ON a.id = t.account_id
     ORDER BY score DESC, t.created_at DESC`,
  );
  return { topics: result.rows.map((row) => serializeTopic(row, true)) };
}

export async function createTopic(
  request: FastifyRequest<{ Body: TopicBody }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const parsed = parseTopic(request.body);
  if ("error" in parsed) {
    return reply.code(400).send({ error: parsed.error });
  }
  const inserted = await pool.query<TopicRow>(
    `INSERT INTO web.topics (account_id, title, body)
     VALUES ($1, $2, $3)
     RETURNING id, account_id, title, body, created_at, updated_at`,
    [account.id, parsed.title, parsed.body],
  );
  const row = inserted.rows[0];
  return reply.code(201).send(
    serializeTopic({
      ...row,
      username: account.username,
      score: 0,
      comment_count: 0,
    }),
  );
}

export async function getTopic(
  request: FastifyRequest<{ Params: IdParams }>,
  reply: FastifyReply,
) {
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid topic id." });
  }
  const topic = await loadTopic(id);
  if (!topic) {
    return reply.code(404).send({ error: "Topic not found." });
  }
  const viewer = await optionalAccount(request);
  const comments = await pool.query<CommentRow>(
    `SELECT c.id, c.account_id, a.username, c.body, c.created_at
     FROM web.comments c
     JOIN auth.accounts a ON a.id = c.account_id
     WHERE c.topic_id = $1
     ORDER BY c.created_at ASC`,
    [id],
  );
  let myVote: number | null = null;
  if (viewer) {
    const vote = await pool.query<{ value: number }>(
      `SELECT value FROM web.votes WHERE topic_id = $1 AND account_id = $2`,
      [id, viewer.id],
    );
    myVote = vote.rows[0]?.value ?? null;
  }
  return {
    ...serializeTopic(topic),
    myVote,
    comments: comments.rows.map(serializeComment),
  };
}

export async function updateTopic(
  request: FastifyRequest<{ Params: IdParams; Body: TopicBody }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid topic id." });
  }
  const topic = await loadTopic(id);
  if (!topic) {
    return reply.code(404).send({ error: "Topic not found." });
  }
  if (Number(topic.account_id) !== account.id) {
    return reply.code(403).send({ error: "You can only edit your own topic." });
  }
  const parsed = parseTopic(request.body);
  if ("error" in parsed) {
    return reply.code(400).send({ error: parsed.error });
  }
  await pool.query(
    `UPDATE web.topics
     SET title = $2, body = $3, updated_at = NOW()
     WHERE id = $1`,
    [id, parsed.title, parsed.body],
  );
  const updated = await loadTopic(id);
  return serializeTopic(updated!);
}

export async function deleteTopic(
  request: FastifyRequest<{ Params: IdParams }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid topic id." });
  }
  const topic = await loadTopic(id);
  if (!topic) {
    return reply.code(404).send({ error: "Topic not found." });
  }
  if (Number(topic.account_id) !== account.id) {
    return reply
      .code(403)
      .send({ error: "You can only delete your own topic." });
  }
  await pool.query(`DELETE FROM web.topics WHERE id = $1`, [id]);
  return reply.code(204).send();
}

export async function voteTopic(
  request: FastifyRequest<{ Params: IdParams; Body: VoteBody }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid topic id." });
  }
  const value = request.body?.value;
  if (value !== 1 && value !== -1) {
    return reply.code(400).send({ error: "Vote must be 1 or -1." });
  }
  const topic = await loadTopic(id);
  if (!topic) {
    return reply.code(404).send({ error: "Topic not found." });
  }
  if (Number(topic.account_id) === account.id) {
    return reply.code(403).send({ error: "You cannot vote on your own idea." });
  }
  await pool.query(
    `INSERT INTO web.votes (topic_id, account_id, value)
     VALUES ($1, $2, $3)
     ON CONFLICT (topic_id, account_id) DO UPDATE SET value = EXCLUDED.value`,
    [id, account.id, value],
  );
  const updated = await loadTopic(id);
  return { ...serializeTopic(updated!), myVote: value };
}

export async function unvoteTopic(
  request: FastifyRequest<{ Params: IdParams }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid topic id." });
  }
  const topic = await loadTopic(id);
  if (!topic) {
    return reply.code(404).send({ error: "Topic not found." });
  }
  await pool.query(
    `DELETE FROM web.votes WHERE topic_id = $1 AND account_id = $2`,
    [id, account.id],
  );
  const updated = await loadTopic(id);
  return { ...serializeTopic(updated!), myVote: null };
}

export async function createComment(
  request: FastifyRequest<{ Params: IdParams; Body: CommentBody }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid topic id." });
  }
  const body = String(request.body?.body ?? "").trim();
  if (body.length === 0 || body.length > COMMENT_MAX) {
    return reply
      .code(400)
      .send({ error: `Comment must be 1–${COMMENT_MAX} characters.` });
  }
  const topic = await loadTopic(id);
  if (!topic) {
    return reply.code(404).send({ error: "Topic not found." });
  }
  const inserted = await pool.query<CommentRow>(
    `INSERT INTO web.comments (topic_id, account_id, body)
     VALUES ($1, $2, $3)
     RETURNING id, account_id, body, created_at`,
    [id, account.id, body],
  );
  const row = inserted.rows[0];
  return reply.code(201).send(
    serializeComment({
      ...row,
      username: account.username,
    }),
  );
}

export async function deleteComment(
  request: FastifyRequest<{ Params: IdParams }>,
  reply: FastifyReply,
) {
  const account = await requireAccount(request, reply);
  if (!account) {
    return;
  }
  const id = parseId(request.params.id);
  if (id === null) {
    return reply.code(400).send({ error: "Invalid comment id." });
  }
  const result = await pool.query<{ account_id: string }>(
    `SELECT account_id FROM web.comments WHERE id = $1`,
    [id],
  );
  const comment = result.rows[0];
  if (!comment) {
    return reply.code(404).send({ error: "Comment not found." });
  }
  if (Number(comment.account_id) !== account.id) {
    return reply
      .code(403)
      .send({ error: "You can only delete your own comment." });
  }
  await pool.query(`DELETE FROM web.comments WHERE id = $1`, [id]);
  return reply.code(204).send();
}

async function loadTopic(id: number): Promise<TopicRow | null> {
  const result = await pool.query<TopicRow>(
    `SELECT
        t.id,
        t.account_id,
        a.username,
        t.title,
        t.body,
        t.created_at,
        t.updated_at,
        COALESCE((SELECT SUM(value) FROM web.votes WHERE topic_id = t.id), 0) AS score,
        (SELECT COUNT(*) FROM web.comments WHERE topic_id = t.id) AS comment_count
     FROM web.topics t
     JOIN auth.accounts a ON a.id = t.account_id
     WHERE t.id = $1`,
    [id],
  );
  return result.rows[0] ?? null;
}

function parseTopic(
  body: TopicBody | undefined,
): { title: string; body: string } | { error: string } {
  const title = String(body?.title ?? "").trim();
  const text = String(body?.body ?? "").trim();
  if (title.length < TITLE_MIN || title.length > TITLE_MAX) {
    return {
      error: `Title must be ${TITLE_MIN}–${TITLE_MAX} characters.`,
    };
  }
  if (text.length === 0 || text.length > BODY_MAX) {
    return { error: `Idea text must be 1–${BODY_MAX} characters.` };
  }
  return { title, body: text };
}

function parseId(raw: string): number | null {
  if (!/^\d+$/.test(raw)) {
    return null;
  }
  const id = Number(raw);
  return Number.isSafeInteger(id) && id > 0 ? id : null;
}

function serializeTopic(row: TopicRow, excerpt = false) {
  const body = excerpt ? excerptBody(row.body) : row.body;
  return {
    id: Number(row.id),
    accountId: Number(row.account_id),
    username: row.username,
    title: row.title,
    body,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    score: Number(row.score),
    commentCount: Number(row.comment_count),
  };
}

function serializeComment(row: CommentRow) {
  return {
    id: Number(row.id),
    accountId: Number(row.account_id),
    username: row.username,
    body: row.body,
    createdAt: row.created_at,
  };
}

function excerptBody(body: string): string {
  if (body.length <= 280) {
    return body;
  }
  return `${body.slice(0, 280).trimEnd()}…`;
}
