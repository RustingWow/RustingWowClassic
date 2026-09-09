import { Link } from "react-router-dom";

import { useAuth } from "../auth";

export function Connect() {
  const { site } = useAuth();
  const host = site?.realmlistHost ?? "127.0.0.1";
  const discord = site?.discordInviteUrl ?? "https://discord.gg/your-invite";

  return (
    <section className="card">
      <h2>How to connect</h2>
      <ol className="steps">
        <li>
          Install a clean <strong>World of Warcraft 1.12.1</strong> client (build
          5875).
        </li>
        <li>
          <Link to="/register">Create an account</Link> on this website. The
          game uses the same username and password.
        </li>
        <li>
          In <code>realmlist.wtf</code> (or <code>WTF/Config.wtf</code>) set:
          <pre>
            <code>set realmlist {host}</code>
          </pre>
        </li>
        <li>
          Log in. The realm is named <strong>WoWServer</strong>.
        </li>
      </ol>
      <p className="hint">
        Usernames are 2–16 letters or digits. The client treats them as
        uppercase. Passwords must be 4–16 characters.
      </p>
      <p className="hint">
        Stuck? Ask on{" "}
        <a href={discord} target="_blank" rel="noreferrer">
          Discord
        </a>
        .
      </p>
    </section>
  );
}
