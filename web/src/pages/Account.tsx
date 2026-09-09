import { useState } from "react";
import { Link, Navigate } from "react-router-dom";

import { logoutAccount } from "../api";
import { useAuth } from "../auth";

export function Account() {
  const { account, ready, setAccount, site } = useAuth();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const host = site?.realmlistHost ?? "127.0.0.1";

  if (ready && !account) {
    return <Navigate to="/login" replace />;
  }

  if (!account) {
    return <p className="muted">Loading…</p>;
  }

  async function onLogout() {
    setBusy(true);
    setError(null);
    try {
      await logoutAccount();
      setAccount(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Logout failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="card narrow">
      <h2>Your account</h2>
      {error ? <p className="banner">{error}</p> : null}
      <dl>
        <div>
          <dt>Username</dt>
          <dd>{account.username}</dd>
        </div>
        <div>
          <dt>Account id</dt>
          <dd>{account.id}</dd>
        </div>
        {account.email ? (
          <div>
            <dt>Email</dt>
            <dd>{account.email}</dd>
          </div>
        ) : null}
      </dl>
      <p className="hint">
        In <code>realmlist.wtf</code> set{" "}
        <code>set realmlist {host}</code>, then log into the client with{" "}
        <strong>{account.username}</strong> and the password you chose here.
        Full steps are on the <Link to="/connect">Connect</Link> page.
      </p>
      <button type="button" onClick={() => void onLogout()} disabled={busy}>
        Sign out
      </button>
    </section>
  );
}
