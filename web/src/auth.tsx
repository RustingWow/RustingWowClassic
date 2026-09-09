import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

import { Account, fetchMe, fetchSite, SiteConfig } from "./api";

type AuthContextValue = {
  account: Account | null;
  site: SiteConfig | null;
  ready: boolean;
  setAccount: (account: Account | null) => void;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [account, setAccount] = useState<Account | null>(null);
  const [site, setSite] = useState<SiteConfig | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    void Promise.all([
      fetchMe()
        .then(setAccount)
        .catch(() => setAccount(null)),
      fetchSite()
        .then(setSite)
        .catch(() =>
          setSite({
            discordInviteUrl: "https://discord.gg/your-invite",
            realmlistHost: "127.0.0.1",
          }),
        ),
    ]).finally(() => setReady(true));
  }, []);

  return (
    <AuthContext.Provider value={{ account, site, ready, setAccount }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const value = useContext(AuthContext);
  if (!value) {
    throw new Error("useAuth must be used inside AuthProvider");
  }
  return value;
}
