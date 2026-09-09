export type NewsItem = {
  id: string;
  date: string;
  title: string;
  summary: string;
};

export const news: NewsItem[] = [
  {
    id: "launch",
    date: "2026-09-09",
    title: "A new Vanilla emulator, written in Rust",
    summary:
      "WoWServer is a from-scratch 1.12.1 backend. Auth, world, and the account portal share one credential store — not a fork of the usual C++ cores.",
  },
  {
    id: "why-rust",
    date: "2026-09-09",
    title: "Why not another TrinityCore fork?",
    summary:
      "Most public Vanilla emulators are C++ descendants of MaNGOS. This project is a new codebase: memory safety, a smaller surface, and room to evolve the protocol layer without carrying decades of fork history.",
  },
  {
    id: "how-to-play",
    date: "2026-09-09",
    title: "Create an account, then log in with the 1.12.1 client",
    summary:
      "Register on this site, point realmlist.wtf at the server, and use the same username and password in the game. Details are on the Connect page.",
  },
];
