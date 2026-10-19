import { describe, expect, it } from "vitest";
import { repositoryUrl, selectDownloadRelease } from "./releases";

function release(number: number, draft = false) {
  const tag = `preview-${number}`;

  return {
    tag_name: tag,
    draft,
    published_at: "2026-10-19T00:00:00Z",
    assets: [
      {
        name: "calendar-windows-x64.zip",
        size: 100,
        state: "uploaded",
        browser_download_url: `${repositoryUrl}/releases/download/${tag}/calendar-windows-x64.zip`,
      },
    ],
  };
}

describe("release downloads", () => {
  it("uses the newest main build, not the order builds finished in", () => {
    expect(
      selectDownloadRelease([release(2), release(10), release(11, true)])?.tag,
    ).toBe("preview-10");
  });

  it("does not invent missing platform links or accept external asset URLs", () => {
    const candidate = release(1);
    candidate.assets[0].browser_download_url = "https://example.com/untrusted.zip";

    expect(selectDownloadRelease([candidate])?.assets).toEqual({});
  });

  it("ignores incomplete uploads and unpublished releases", () => {
    const candidate = release(1);
    candidate.assets[0].state = "starter";

    expect(selectDownloadRelease([candidate])?.assets).toEqual({});
    expect(selectDownloadRelease([{ ...candidate, published_at: null }])).toBeNull();
    expect(selectDownloadRelease([])).toBeNull();
  });
});
