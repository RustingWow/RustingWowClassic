import { FormEvent, useState } from "react";
import { Link, Navigate, useNavigate } from "react-router-dom";

import { loginAccount } from "../api";
import { useAuth } from "../auth";

export function Login() {
  const { account, ready, setAccount } = useAuth();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (ready && account) {
    return <Navigate to="/account" replace />;
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    setBusy(true);
    setError(null);
    try {
      const me = await loginAccount(
        String(form.get("username") ?? ""),
        String(form.get("password") ?? ""),
      );
      setAccount(me);
      navigate("/account");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={onSubmit} className="card narrow">
      <h2>Sign in</h2>
      <p className="hint">Use the same credentials as the game client.</p>
      {error ? <p className="banner">{error}</p> : null}
      <label>
        Username
        <input name="username" autoComplete="username" required />
      </label>
      <label>
        Password
        <input
          name="password"
          type="password"
          autoComplete="current-password"
          required
        />
      </label>
      <button type="submit" disabled={busy}>
        Sign in
      </button>
      <p className="switch">
        New here? <Link to="/register">Create an account</Link>
      </p>
    </form>
  );
}
