import { useEffect, useState } from "react";

import { Changelog, ChangelogRelease, fetchChangelog } from "../api";

export function ChangelogPage() {
  const [data, setData] = useState<Changelog | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchChangelog()
      .then(setData)
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : "Failed to load changelog");
      });
  }, []);

  return (
    <>
      <section className="card">
        <h2>Changelog</h2>
        <p className="lede">
          Each GitHub release on{" "}
          <code>{data?.branch ?? "develop"}</code> becomes a card here:
          features and bug fixes, grouped from conventional commits.
        </p>
        {data ? (
          <p className="hint">
            Source:{" "}
            <a
              href={`https://github.com/${data.repo}`}
              target="_blank"
              rel="noreferrer"
            >
              {data.repo}
            </a>
          </p>
        ) : null}
      </section>

      {error ? <p className="banner">{error}</p> : null}
      {!data && !error ? <p className="muted">Loading…</p> : null}
      {data && data.releases.length === 0 ? (
        <p className="muted">
          No releases yet. Tag <code>v0.1.0</code> on develop and push it to
          publish the first card.
        </p>
      ) : null}

      <div className="idea-list">
        {data?.releases.map((release) => (
          <ReleaseCard key={release.tag} release={release} />
        ))}
      </div>
    </>
  );
}

function ReleaseCard({ release }: { release: ChangelogRelease }) {
  const empty =
    release.features.length === 0 &&
    release.fixes.length === 0 &&
    release.other.length === 0;

  return (
    <article className="card">
      <p className="kicker">
        {release.unreleased ? "On the develop branch" : release.tag}
        {release.publishedAt ? ` · ${formatDate(release.publishedAt)}` : null}
      </p>
      <h3>
        <a href={release.url} target="_blank" rel="noreferrer">
          {release.name}
        </a>
      </h3>
      {empty ? (
        <p className="muted">No user-facing notes in this range.</p>
      ) : (
        <>
          <NoteList title="Features" items={release.features} />
          <NoteList title="Bug fixes" items={release.fixes} />
          <NoteList title="Other" items={release.other} />
        </>
      )}
    </article>
  );
}

function NoteList({ title, items }: { title: string; items: string[] }) {
  if (items.length === 0) {
    return null;
  }
  return (
    <div>
      <h4 className="section-title">{title}</h4>
      <ul className="steps">
        {items.map((item, index) => (
          <li key={`${title}-${index}`}>{item}</li>
        ))}
      </ul>
    </div>
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
