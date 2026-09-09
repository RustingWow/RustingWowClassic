const GITHUB_REPO =
  process.env.GITHUB_REPO ?? "RustingWow/RustingWowClassic";
const GITHUB_CHANGELOG_BRANCH = process.env.GITHUB_CHANGELOG_BRANCH ?? "develop";
const GITHUB_TOKEN = process.env.GITHUB_TOKEN ?? "";
const CACHE_MS = 5 * 60 * 1000;

export type ChangelogSection = {
  features: string[];
  fixes: string[];
  other: string[];
};

export type ChangelogRelease = {
  tag: string;
  name: string;
  publishedAt: string | null;
  url: string;
  unreleased: boolean;
} & ChangelogSection;

type Cache = {
  at: number;
  payload: {
    repo: string;
    branch: string;
    releases: ChangelogRelease[];
  };
};

let cache: Cache | null = null;

export async function getChangelog() {
  if (cache && Date.now() - cache.at < CACHE_MS) {
    return cache.payload;
  }
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "User-Agent": "wowserver-web",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${GITHUB_TOKEN}`;
  }

  const releasesJson = await githubJson<GithubRelease[]>(
    `https://api.github.com/repos/${GITHUB_REPO}/releases?per_page=20`,
    headers,
  );
  const releases = (releasesJson ?? [])
    .filter((release) => !release.draft)
    .map((release) => ({
      tag: release.tag_name,
      name: release.name || release.tag_name,
      publishedAt: release.published_at,
      url: release.html_url,
      unreleased: false,
      ...parseNotes(release.body ?? ""),
    }));

  const unreleased = await unreleasedCard(headers, releases[0]?.tag);
  const payload = {
    repo: GITHUB_REPO,
    branch: GITHUB_CHANGELOG_BRANCH,
    releases: unreleased ? [unreleased, ...releases] : releases,
  };
  cache = { at: Date.now(), payload };
  return payload;
}

async function unreleasedCard(
  headers: Record<string, string>,
  latestTag: string | undefined,
): Promise<ChangelogRelease | null> {
  const path = latestTag
    ? `compare/${encodeURIComponent(latestTag)}...${encodeURIComponent(GITHUB_CHANGELOG_BRANCH)}`
    : `commits?sha=${encodeURIComponent(GITHUB_CHANGELOG_BRANCH)}&per_page=30`;
  const json = await githubJson<GithubCompare | GithubCommit[]>(
    `https://api.github.com/repos/${GITHUB_REPO}/${path}`,
    headers,
  );
  if (!json) {
    return null;
  }
  const commits = Array.isArray(json) ? json : json.commits ?? [];
  const messages = commits
    .map((commit) => commit.commit.message.split("\n")[0]?.trim() ?? "")
    .filter((message) => message && !message.toLowerCase().startsWith("merge "));
  if (messages.length === 0) {
    return null;
  }
  const sections = classifyMessages(messages);
  if (
    sections.features.length === 0 &&
    sections.fixes.length === 0 &&
    sections.other.length === 0
  ) {
    return null;
  }
  return {
    tag: GITHUB_CHANGELOG_BRANCH,
    name: "Unreleased",
    publishedAt: null,
    url: `https://github.com/${GITHUB_REPO}/tree/${GITHUB_CHANGELOG_BRANCH}`,
    unreleased: true,
    ...sections,
  };
}

async function githubJson<T>(url: string, headers: Record<string, string>): Promise<T | null> {
  try {
    const response = await fetch(url, { headers });
    if (!response.ok) {
      return null;
    }
    return (await response.json()) as T;
  } catch {
    return null;
  }
}

function parseNotes(body: string): ChangelogSection {
  const features: string[] = [];
  const fixes: string[] = [];
  const other: string[] = [];
  let bucket: keyof ChangelogSection = "other";

  for (const raw of body.split("\n")) {
    const line = raw.trim();
    if (line.startsWith("#")) {
      const heading = line.replace(/^#+\s*/, "").toLowerCase();
      if (heading.includes("feature")) {
        bucket = "features";
      } else if (heading.includes("fix") || heading.includes("bug")) {
        bucket = "fixes";
      } else if (
        heading.includes("what") ||
        heading.includes("other") ||
        heading.includes("change")
      ) {
        bucket = "other";
      }
      continue;
    }
    const bullet = line.match(/^[-*]\s+(?:[A-Z] of )?(?:\*\*)?(.+)$/);
    if (!bullet) {
      continue;
    }
    const text = cleanBullet(bullet[1]);
    if (!text) {
      continue;
    }
    const kind = kindFromMessage(text) ?? bucket;
    if (kind === "skip") {
      continue;
    }
    if (kind === "features") {
      features.push(stripPrefix(text));
    } else if (kind === "fixes") {
      fixes.push(stripPrefix(text));
    } else {
      other.push(stripPrefix(text));
    }
  }

  return { features, fixes, other };
}

function classifyMessages(messages: string[]): ChangelogSection {
  const features: string[] = [];
  const fixes: string[] = [];
  const other: string[] = [];
  for (const message of messages) {
    const kind = kindFromMessage(message);
    const text = stripPrefix(message);
    if (kind === "features") {
      features.push(text);
    } else if (kind === "fixes") {
      fixes.push(text);
    } else if (kind !== "skip") {
      other.push(text);
    }
  }
  return { features, fixes, other };
}

function kindFromMessage(
  message: string,
): "features" | "fixes" | "skip" | "other" | null {
  if (/^(feat|feature)(\(.+\))?!?:/i.test(message)) {
    return "features";
  }
  if (/^fix(\(.+\))?!?:/i.test(message)) {
    return "fixes";
  }
  if (/^(chore|ci|test|style)(\(.+\))?:/i.test(message)) {
    return "skip";
  }
  return null;
}

function stripPrefix(message: string): string {
  return message
    .replace(
      /^(feat|fix|docs?|perf|refactor|chore|ci|test|style)(\(.+?\))?!?:\s*/i,
      "",
    )
    .replace(/\s+by @\S+ in https?:\S+$/i, "")
    .trim();
}

function cleanBullet(text: string): string {
  return text.replace(/\*\*/g, "").replace(/\s+by @\S+.*$/i, "").trim();
}

type GithubRelease = {
  tag_name: string;
  name: string | null;
  body: string | null;
  html_url: string;
  published_at: string | null;
  draft: boolean;
};

type GithubCommit = {
  commit: { message: string };
};

type GithubCompare = {
  commits?: GithubCommit[];
};
