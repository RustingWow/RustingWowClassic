import { useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";

import { createTopic } from "../api";
import { useAuth } from "../auth";
import { IdeaFormFields } from "./Ideas";

export function IdeaNew() {
  const { account, ready } = useAuth();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (ready && !account) {
    return <Navigate to="/login" replace />;
  }

  async function onSubmit(title: string, body: string) {
    setBusy(true);
    setError(null);
    try {
      const topic = await createTopic(title, body);
      navigate(`/ideas/${topic.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not publish idea");
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <h2>New idea</h2>
      <IdeaFormFields
        submitLabel="Publish"
        busy={busy}
        error={error}
        hint="Tell us the kind of realm you want: blizzlike Vanilla, instant-x, a fun high-rate server, or a community that hosts more than one type at once."
        onSubmit={(title, body) => void onSubmit(title, body)}
      />
    </>
  );
}
