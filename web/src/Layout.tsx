import { NavLink } from "react-router-dom";
import type { ReactNode } from "react";

import { useAuth } from "./auth";

export function Layout({ children }: { children: ReactNode }) {
  const { account, site } = useAuth();
  const discord = site?.discordInviteUrl ?? "https://discord.gg/your-invite";

  return (
    <div className="shell">
      <header className="site-header">
        <div>
          <p className="kicker">Vanilla 1.12.1</p>
          <NavLink to="/" className="brand">
            WoWServer
          </NavLink>
        </div>
        <nav className="nav">
          <NavLink to="/" end>
            Home
          </NavLink>
          <NavLink to="/ideas">Ideas</NavLink>
          <NavLink to="/changelog">Changelog</NavLink>
          <NavLink to="/connect">Connect</NavLink>
          {account ? (
            <NavLink to="/account">Account</NavLink>
          ) : (
            <>
              <NavLink to="/register">Register</NavLink>
              <NavLink to="/login">Sign in</NavLink>
            </>
          )}
          <a href={discord} target="_blank" rel="noreferrer">
            Discord
          </a>
        </nav>
      </header>
      <main>{children}</main>
      <footer className="site-footer">
        <p>
          Same account for the website and the game client. Questions? Join{" "}
          <a href={discord} target="_blank" rel="noreferrer">
            Discord
          </a>
          .
        </p>
      </footer>
    </div>
  );
}
