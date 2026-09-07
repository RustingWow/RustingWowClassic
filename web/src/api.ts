export type Account = {
  id: number;
  username: string;
  email: string | null;
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });
  if (response.status === 204) {
    return undefined as T;
  }
  const body = (await response.json().catch(() => ({}))) as {
    error?: string;
  } & T;
  if (!response.ok) {
    throw new Error(body.error ?? `Request failed (${response.status})`);
  }
  return body;
}

export function fetchMe(): Promise<Account> {
  return request<Account>("/api/me");
}

export function registerAccount(
  username: string,
  password: string,
  email: string,
): Promise<Account> {
  return request<Account>("/api/register", {
    method: "POST",
    body: JSON.stringify({
      username,
      password,
      email: email.trim() === "" ? undefined : email,
    }),
  });
}

export function loginAccount(
  username: string,
  password: string,
): Promise<Account> {
  return request<Account>("/api/login", {
    method: "POST",
    body: JSON.stringify({ username, password }),
  });
}

export function logoutAccount(): Promise<void> {
  return request<void>("/api/logout", { method: "POST" });
}
