import { Link } from "react-router-dom";

import { useAuth } from "../auth";
import { news } from "../content/news";

export function Home() {
  const { site } = useAuth();
  const discord = site?.discordInviteUrl ?? "https://discord.gg/your-invite";

  return (
    <>
      <section className="hero card">
        <p className="kicker">A new project</p>
        <h1>Vanilla, rewritten in Rust</h1>
        <p className="lede">
          WoWServer is a from-scratch emulator for World of Warcraft 1.12.1
          (build 5875). Unlike TrinityCore, AzerothCore, and the other public
          cores — all written in C++ and descended from MaNGOS — this backend is
          a new codebase in Rust.
        </p>
        <p className="lede">
          Create an account here, then use the same username and password in the
          official 1.12.1 client.
        </p>
        <div className="cta-row">
          <Link to="/register" className="button">
            Create an account
          </Link>
          <Link to="/ideas" className="button ghost">
            Share your custom-server idea
          </Link>
          <Link to="/changelog" className="button ghost">
            Changelog
          </Link>
          <Link to="/connect" className="button ghost">
            How to connect
          </Link>
        </div>
      </section>

      <section className="discord card">
        <h2>Discord</h2>
        <p>
          The server is young. Come talk about bugs, missing spells, and what to
          build next — the invite is the fastest way in.
        </p>
        <a className="button" href={discord} target="_blank" rel="noreferrer">
          Join the Discord
        </a>
      </section>

      <section>
        <h2 className="section-title">News</h2>
        <div className="news-list">
          {news.map((item) => (
            <article key={item.id} className="card news-card">
              <p className="kicker">{item.date}</p>
              <h3>{item.title}</h3>
              <p>{item.summary}</p>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}
