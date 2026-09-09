import { useEffect, useState } from "react";
import { Navigate, useNavigate, useParams } from "react-router-dom";

import { fetchTopic, TopicDetail, updateTopic } from "../api";
import { useAuth } from "../auth";
import { IdeaFormFields } from "./Ideas";

export function IdeaEdit() {
  const { id } = useParams();
  const topicId = Number(id);
  const { account, ready } = useAuth();
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

  if (ready && !account) {
    return <Navigate to="/login" replace />;
  }
  if (!Number.isFinite(topicId) || topicId <= 0) {
    return <Navigate to="/ideas" replace />;
  }
  if (error && !topic) {
    return <p className="banner">{error}</p>;
  }
  if (!topic) {
    return <p className="muted">Loading…</p>;
  }
  if (account && account.id !== topic.accountId) {
    return <Navigate to={`/ideas/${topic.id}`} replace />;
  }

  async function onSubmit(title: string, body: string) {
    setBusy(true);
    setError(null);
    try {
      await updateTopic(topicId, title, body);
      navigate(`/ideas/${topicId}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not save idea");
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <h2>Edit idea</h2>
      <IdeaFormFields
        defaultTitle={topic.title}
        defaultBody={topic.body}
        submitLabel="Save"
        busy={busy}
        error={error}
        onSubmit={(title, body) => void onSubmit(title, body)}
      />
    </>
  );
}
