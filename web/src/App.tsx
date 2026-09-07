import { FormEvent, useEffect, useState } from "react";

import {
  Account,
  fetchMe,
  loginAccount,
  logoutAccount,
  registerAccount,
} from "./api";

type Screen = "loading" | "login" | "register" | "account";

export default function App() {
  const [screen, setScreen] = useState<Screen>("loading");
  const [account, setAccount] = useState<Account | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    fetchMe()
      .then((me) => {
        setAccount(me);
        setScreen("account");
      })
      .catch(() => {
        setScreen("login");
      });
  }, []);

  async function onRegister(event: FormEvent<HTMLFormElement>) {
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
      setScreen("account");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Registration failed");
    } finally {
      setBusy(false);
    }
  }

  async function onLogin(event: FormEvent<HTMLFormElement>) {
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
      setScreen("account");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setBusy(false);
    }
  }

  async function onLogout() {
    setBusy(true);
    setError(null);
    try {
      await logoutAccount();
      setAccount(null);
      setScreen("login");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Logout failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="shell">
      <header>
        <p className="kicker">Vanilla 1.12.1</p>
        <h1>WoWServer</h1>
        <p className="lede">
          Create an account here, then use the same username and password in the
          game client.
        </p>
      </header>

      {error ? <p className="banner">{error}</p> : null}

      {screen === "loading" ? <p className="muted">Loading…</p> : null}

      {screen === "register" ? (
        <form onSubmit={onRegister} className="card">
          <h2>Create account</h2>
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
            Already have an account?{" "}
            <button type="button" className="link" onClick={() => setScreen("login")}>
              Sign in
            </button>
          </p>
        </form>
      ) : null}

      {screen === "login" ? (
        <form onSubmit={onLogin} className="card">
          <h2>Sign in</h2>
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
            New here?{" "}
            <button
              type="button"
              className="link"
              onClick={() => setScreen("register")}
            >
              Create an account
            </button>
          </p>
        </form>
      ) : null}

      {screen === "account" && account ? (
        <section className="card">
          <h2>Your account</h2>
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
            In <code>realmlist.wtf</code> set <code>set realmlist 127.0.0.1</code>,
            then log into the client with <strong>{account.username}</strong> and
            the password you chose here.
          </p>
          <button type="button" onClick={onLogout} disabled={busy}>
            Sign out
          </button>
        </section>
      ) : null}
    </div>
  );
}
