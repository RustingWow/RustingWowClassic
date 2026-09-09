import { FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { fetchTopics, TopicSummary } from "../api";
import { useAuth } from "../auth";

export function Ideas() {
  const { account } = useAuth();
  const [topics, setTopics] = useState<TopicSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchTopics()
      .then((result) => setTopics(result.topics))
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : "Failed to load ideas");
      });
  }, []);

  return (
    <>
      <section className="card">
        <div className="page-head">
          <div>
            <h2>What kind of server should we build?</h2>
            <p className="lede">
              This emulator is new. Before we lock a direction, we want to hear
              how you would actually play. Post an idea, comment on others, and
              vote. The score helps us see what the community wants.
            </p>
            <p className="hint">
              We are looking for concrete pitches, not one-line slogans. For
              example: a strict <strong>blizzlike</strong> Vanilla realm; an{" "}
              <strong>instant-60 / instant-x</strong> server focused on raids
              and PvP; a high-rate fun server; or a{" "}
              <strong>community with several realms</strong> side by side
              (blizzlike + custom, PvE + PvP, and so on). Say what rates, rules,
              and content you care about, and why that would be fun here.
            </p>
          </div>
          {account ? (
            <Link to="/ideas/new" className="button">
              New idea
            </Link>
          ) : (
            <Link to="/login" className="button ghost">
              Sign in to post
            </Link>
          )}
        </div>
      </section>

      {error ? <p className="banner">{error}</p> : null}
      {topics === null && !error ? <p className="muted">Loading…</p> : null}
      {topics && topics.length === 0 ? (
        <p className="muted">No ideas yet. Be the first to post one.</p>
      ) : null}

      <div className="idea-list">
        {topics?.map((topic) => (
          <article key={topic.id} className="card idea-card">
            <p className="kicker">
              {topic.score} score · {topic.commentCount}{" "}
              {topic.commentCount === 1 ? "comment" : "comments"} ·{" "}
              {topic.username}
            </p>
            <h3>
              <Link to={`/ideas/${topic.id}`}>{topic.title}</Link>
            </h3>
            <p className="idea-excerpt">{topic.body}</p>
          </article>
        ))}
      </div>
    </>
  );
}

export function IdeaFormFields({
  defaultTitle,
  defaultBody,
  submitLabel,
  busy,
  error,
  hint,
  onSubmit,
}: {
  defaultTitle?: string;
  defaultBody?: string;
  submitLabel: string;
  busy: boolean;
  error: string | null;
  hint?: string;
  onSubmit: (title: string, body: string) => void;
}) {
  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    onSubmit(String(form.get("title") ?? ""), String(form.get("body") ?? ""));
  }

  return (
    <form onSubmit={handleSubmit} className="card">
      {hint ? <p className="hint">{hint}</p> : null}
      {error ? <p className="banner">{error}</p> : null}
      <label>
        Title
        <input
          name="title"
          required
          minLength={3}
          maxLength={120}
          defaultValue={defaultTitle}
          placeholder="e.g. Instant-60 PvP realm with weekend BGs"
        />
      </label>
      <label>
        Your idea
        <textarea
          name="body"
          required
          minLength={1}
          maxLength={8000}
          rows={12}
          defaultValue={defaultBody}
          placeholder="Blizzlike, instant-x, several realms in one community… rates, rules, and why it would be fun."
        />
      </label>
      <button type="submit" disabled={busy}>
        {submitLabel}
      </button>
    </form>
  );
}
