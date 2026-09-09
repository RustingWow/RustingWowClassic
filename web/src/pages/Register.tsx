import { FormEvent, useState } from "react";
import { Link, Navigate, useNavigate } from "react-router-dom";

import { registerAccount } from "../api";
import { useAuth } from "../auth";

export function Register() {
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
      const me = await registerAccount(
        String(form.get("username") ?? ""),
        String(form.get("password") ?? ""),
        String(form.get("email") ?? ""),
      );
      setAccount(me);
      navigate("/account");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Registration failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={onSubmit} className="card narrow">
      <h2>Create account</h2>
      <p className="hint">
        This username and password are the ones you will type in the game
        client.
      </p>
      {error ? <p className="banner">{error}</p> : null}
      <label>
        Username
        <input
          name="username"
          autoComplete="username"
          required
          minLength={2}
          maxLength={16}
          pattern="[A-Za-z0-9]+"
          title="2–16 letters or digits"
        />
      </label>
      <label>
        Password
        <input
          name="password"
          type="password"
          autoComplete="new-password"
          required
          minLength={4}
          maxLength={16}
        />
      </label>
      <label>
        Email <span className="muted">(optional)</span>
        <input name="email" type="email" autoComplete="email" />
      </label>
      <button type="submit" disabled={busy}>
        Register
      </button>
      <p className="switch">
        Already have an account? <Link to="/login">Sign in</Link>
      </p>
    </form>
  );
}
