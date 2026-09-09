export type Account = {
  id: number;
  username: string;
  email: string | null;
};

export type SiteConfig = {
  discordInviteUrl: string;
  realmlistHost: string;
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  if (init?.body !== undefined && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  const response = await fetch(path, {
    credentials: "include",
    ...init,
    headers,
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

export function fetchSite(): Promise<SiteConfig> {
  return request<SiteConfig>("/api/site");
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

export type TopicSummary = {
  id: number;
  accountId: number;
  username: string;
  title: string;
  body: string;
  createdAt: string;
  updatedAt: string;
  score: number;
  commentCount: number;
};

export type Comment = {
  id: number;
  accountId: number;
  username: string;
  body: string;
  createdAt: string;
};

export type TopicDetail = TopicSummary & {
  myVote: number | null;
  comments: Comment[];
};

export function fetchTopics(): Promise<{ topics: TopicSummary[] }> {
  return request<{ topics: TopicSummary[] }>("/api/topics");
}

export function fetchTopic(id: number): Promise<TopicDetail> {
  return request<TopicDetail>(`/api/topics/${id}`);
}

export function createTopic(
  title: string,
  body: string,
): Promise<TopicSummary> {
  return request<TopicSummary>("/api/topics", {
    method: "POST",
    body: JSON.stringify({ title, body }),
  });
}

export function updateTopic(
  id: number,
  title: string,
  body: string,
): Promise<TopicSummary> {
  return request<TopicSummary>(`/api/topics/${id}`, {
    method: "PATCH",
    body: JSON.stringify({ title, body }),
  });
}

export function deleteTopic(id: number): Promise<void> {
  return request<void>(`/api/topics/${id}`, { method: "DELETE" });
}

export function voteTopic(
  id: number,
  value: 1 | -1,
): Promise<TopicSummary & { myVote: number | null }> {
  return request<TopicSummary & { myVote: number | null }>(
    `/api/topics/${id}/vote`,
    {
      method: "PUT",
      body: JSON.stringify({ value }),
    },
  );
}

export function unvoteTopic(
  id: number,
): Promise<TopicSummary & { myVote: number | null }> {
  return request<TopicSummary & { myVote: number | null }>(
    `/api/topics/${id}/vote`,
    { method: "DELETE" },
  );
}

export function createComment(topicId: number, body: string): Promise<Comment> {
  return request<Comment>(`/api/topics/${topicId}/comments`, {
    method: "POST",
    body: JSON.stringify({ body }),
  });
}

export function deleteComment(id: number): Promise<void> {
  return request<void>(`/api/comments/${id}`, { method: "DELETE" });
}

export type ChangelogRelease = {
  tag: string;
  name: string;
  publishedAt: string | null;
  url: string;
  unreleased: boolean;
  features: string[];
  fixes: string[];
  other: string[];
};

export type Changelog = {
  repo: string;
  branch: string;
  releases: ChangelogRelease[];
};

export function fetchChangelog(): Promise<Changelog> {
  return request<Changelog>("/api/changelog");
}
