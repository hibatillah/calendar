export const repositoryUrl = "https://github.com/hibatillah/calendar";
export const releasesApiUrl =
  "https://api.github.com/repos/hibatillah/calendar/releases?per_page=100";

export const releaseAssetNames = {
  windows: "calendar-windows-x64.zip",
  android: "calendar-android-preview.apk",
  ios: "calendar-ios-simulator.tar.gz",
  web: "calendar-web-build.tar.gz",
  checksums: "SHA256SUMS.txt",
} as const;

interface ReleaseAsset {
  name: string;
  size: number;
  state: string;
  browser_download_url: string;
}

interface RepositoryRelease {
  tag_name: string;
  draft: boolean;
  published_at: string | null;
  assets: ReleaseAsset[];
}

export interface DownloadRelease {
  tag: string;
  url: string;
  assets: Partial<Record<keyof typeof releaseAssetNames, string>>;
}

// Preview numbers follow main pushes, even if an older build finishes later.
export function selectDownloadRelease(
  releases: RepositoryRelease[],
): DownloadRelease | null {
  const release = releases
    .filter(
      (candidate) =>
        !candidate.draft &&
        candidate.published_at &&
        /^preview-\d+$/.test(candidate.tag_name),
    )
    .sort(
      (left, right) => Number(right.tag_name.slice(8)) - Number(left.tag_name.slice(8)),
    )[0];

  if (!release) return null;

  const assets: DownloadRelease["assets"] = {};

  for (const [platform, name] of Object.entries(releaseAssetNames)) {
    const expectedUrl = `${repositoryUrl}/releases/download/${release.tag_name}/${name}`;
    const asset = release.assets.find(
      (candidate) =>
        candidate.name === name &&
        candidate.state === "uploaded" &&
        candidate.size > 0 &&
        candidate.browser_download_url === expectedUrl,
    );

    if (asset) assets[platform as keyof typeof releaseAssetNames] = expectedUrl;
  }

  return {
    tag: release.tag_name,
    url: `${repositoryUrl}/releases/tag/${release.tag_name}`,
    assets,
  };
}

let cachedRelease: { value: DownloadRelease | null; expiresAt: number } | undefined;

export async function fetchDownloadRelease(
  signal: AbortSignal,
): Promise<DownloadRelease | null> {
  if (cachedRelease && cachedRelease.expiresAt > Date.now()) return cachedRelease.value;

  const response = await fetch(releasesApiUrl, {
    signal,
    headers: { Accept: "application/vnd.github+json" },
  });

  if (!response.ok) throw new Error("Release information is temporarily unavailable.");

  const releases: unknown = await response.json();

  if (!Array.isArray(releases)) throw new Error("Unexpected release response.");

  const value = selectDownloadRelease(releases);
  cachedRelease = { value, expiresAt: Date.now() + 5 * 60_000 };

  return value;
}
