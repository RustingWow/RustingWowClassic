import { FormEvent, useEffect, useState } from "react";
import { Link, Navigate, useNavigate, useParams } from "react-router-dom";

import {
  Comment,
  createComment,
  deleteComment,
  deleteTopic,
  fetchTopic,
  TopicDetail,
  unvoteTopic,
  voteTopic,
} from "../api";
import { useAuth } from "../auth";

export function IdeaDetail() {
  const { id } = useParams();
  const topicId = Number(id);
  const { account } = useAuth();
  const navigate = useNavigate();
  const [topic, setTopic] = useState<TopicDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!Number.isFinite(topicId) || topicId <= 0) {
      return;
    }
    fetchTopic(topicId)
      .then(setTopic)
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : "Failed to load idea");
      });
  }, [topicId]);

  if (!Number.isFinite(topicId) || topicId <= 0) {
    return <Navigate to="/ideas" replace />;
  }

  const isAuthor = account !== null && topic !== null && account.id === topic.accountId;
  const canVote = account !== null && topic !== null && !isAuthor;

  async function onVote(value: 1 | -1) {
    if (!topic) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next =
        topic.myVote === value
          ? await unvoteTopic(topic.id)
          : await voteTopic(topic.id, value);
      setTopic({ ...topic, score: next.score, myVote: next.myVote });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Vote failed");
    } finally {
      setBusy(false);
    }
  }

  async function onDeleteTopic() {
    if (!topic || !window.confirm("Delete this idea?")) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await deleteTopic(topic.id);
      navigate("/ideas");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Delete failed");
      setBusy(false);
    }
  }

  async function onComment(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!topic) {
      return;
    }
    const form = event.currentTarget;
    const body = String(new FormData(form).get("body") ?? "");
    setBusy(true);
    setError(null);
    try {
      const comment = await createComment(topic.id, body);
      setTopic({
        ...topic,
        comments: [...topic.comments, comment],
        commentCount: topic.commentCount + 1,
      });
      form.reset();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Comment failed");
    } finally {
      setBusy(false);
    }
  }

  async function onDeleteComment(comment: Comment) {
    if (!topic) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await deleteComment(comment.id);
      setTopic({
        ...topic,
        comments: topic.comments.filter((item) => item.id !== comment.id),
        commentCount: Math.max(0, topic.commentCount - 1),
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not delete comment");
    } finally {
      setBusy(false);
    }
  }

  if (error && !topic) {
    return <p className="banner">{error}</p>;
  }
  if (!topic) {
    return <p className="muted">Loading…</p>;
  }

  return (
    <>
      <p className="switch">
        <Link to="/ideas">All ideas</Link>
      </p>
      {error ? <p className="banner">{error}</p> : null}
      <article className="card">
        <div className="page-head">
          <div>
            <p className="kicker">
              {topic.username} · {formatDate(topic.createdAt)}
            </p>
            <h2>{topic.title}</h2>
          </div>
          <div className="vote-box">
            <button
              type="button"
              className={topic.myVote === 1 ? "vote active" : "vote"}
              disabled={busy || !canVote}
              onClick={() => void onVote(1)}
              title={canVote ? "Upvote" : isAuthor ? "You cannot vote on your own idea" : "Sign in to vote"}
            >
              +
            </button>
            <span className="score">{topic.score}</span>
            <button
              type="button"
              className={topic.myVote === -1 ? "vote active" : "vote"}
              disabled={busy || !canVote}
              onClick={() => void onVote(-1)}
              title={canVote ? "Downvote" : isAuthor ? "You cannot vote on your own idea" : "Sign in to vote"}
            >
              −
            </button>
          </div>
        </div>
        <p className="idea-body">{topic.body}</p>
        {isAuthor ? (
          <div className="cta-row">
            <Link to={`/ideas/${topic.id}/edit`} className="button ghost">
              Edit
            </Link>
            <button type="button" className="danger" onClick={() => void onDeleteTopic()} disabled={busy}>
              Delete
            </button>
          </div>
        ) : null}
      </article>

      <section className="card">
        <h3>
          Comments ({topic.commentCount})
        </h3>
        {topic.comments.length === 0 ? (
          <p className="muted">No comments yet.</p>
        ) : (
          <ul className="comment-list">
            {topic.comments.map((comment) => (
              <li key={comment.id}>
                <p className="kicker">
                  {comment.username} · {formatDate(comment.createdAt)}
                </p>
                <p className="idea-body">{comment.body}</p>
                {account?.id === comment.accountId ? (
                  <button
                    type="button"
                    className="link"
                    disabled={busy}
                    onClick={() => void onDeleteComment(comment)}
                  >
                    Delete
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
        {account ? (
          <form onSubmit={(event) => void onComment(event)} className="comment-form">
            <label>
              Add a comment
              <textarea name="body" required maxLength={2000} rows={4} />
            </label>
            <button type="submit" disabled={busy}>
              Comment
            </button>
          </form>
        ) : (
          <p className="hint">
            <Link to="/login">Sign in</Link> to comment.
          </p>
        )}
      </section>
    </>
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleDateString("en-GB", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
