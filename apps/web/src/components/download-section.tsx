import { ArrowUpRight, Globe, Laptop, Smartphone } from "lucide-react";
import { useEffect, useState } from "react";
import {
  fetchDownloadRelease,
  repositoryUrl,
  type DownloadRelease,
} from "../lib/releases";
import { Button } from "./ui/button";

export function DownloadSection() {
  const [release, setRelease] = useState<DownloadRelease | null>(null);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    const controller = new AbortController();

    fetchDownloadRelease(controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setRelease(value);
      })
      .catch(() => {
        if (!controller.signal.aborted) setFailed(true);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, []);

  const platforms = [
    {
      key: "windows",
      title: "Windows",
      label: "Download Windows MSI",
      buttonText: "Windows installer",
      icon: Laptop,
      description:
        "Install the x64 desktop app and local backend, with shortcuts and standard Windows uninstall. Unsigned preview; Google OAuth setup is required.",
    },
    {
      key: "android",
      title: "Android",
      label: "Download Android APK",
      buttonText: "Android APK",
      icon: Smartphone,
      description:
        "Installable preview for testing. Requires access to your local Calendar backend; not a Play Store release.",
    },
    {
      key: "web",
      title: "Web",
      label: "Open web calendar",
      buttonText: "Open calendar",
      icon: Globe,
      description:
        "Your calendar in the browser. Hosted Google sign-in is being prepared; the local web preview is available now.",
    },
  ] as const;

  return (
    <section className="download-section" id="download">
      <span className="section-eyebrow">A PLACE FOR EVERY DAY</span>
      <h2>Make yourself some room.</h2>
      <p>Calendar is in preview. Pick your platform to try it.</p>
      <p role="status" aria-live="polite">
        {loading
          ? "Checking the latest release…"
          : failed
            ? "Could not load downloads. Check GitHub Releases below."
            : release
              ? `Latest preview: ${release.tag}`
              : "The first downloadable release is on its way."}
      </p>
      <div className="platform-grid">
        {platforms.map((platform) => {
          const downloadUrl =
            platform.key === "web" ? "/app" : release?.assets[platform.key];

          return (
            <article key={platform.key}>
              <platform.icon />
              <h3>{platform.title}</h3>
              <span className={`platform-status${downloadUrl ? " available" : ""}`}>
                {platform.key === "web"
                  ? "In development"
                  : downloadUrl
                    ? "Developer preview"
                    : "Not available yet"}
              </span>
              <p>{platform.description}</p>
              {downloadUrl ? (
                <Button asChild className="marketing-button">
                  <a href={downloadUrl} aria-label={platform.label}>
                    {platform.buttonText}
                    <ArrowUpRight />
                  </a>
                </Button>
              ) : (
                <span className="unavailable-download">No download available</span>
              )}
            </article>
          );
        })}
      </div>
      <p className="download-note">
        <a href={release?.url ?? `${repositoryUrl}/releases`}>
          Release notes and all files
        </a>
        {release?.assets.windowsPortable && (
          <>
            {" "}
            · <a href={release.assets.windowsPortable}>Windows portable ZIP</a>
          </>
        )}
        {release?.assets.checksums && (
          <>
            {" "}
            · <a href={release.assets.checksums}>SHA-256 checksums</a>
          </>
        )}
        {release?.assets.web && (
          <>
            {" "}
            · <a href={release.assets.web}>Web build (developers)</a>
          </>
        )}
      </p>
      <p className="download-note">
        Downloads are public GitHub Release assets. See the{" "}
        <a href={`${repositoryUrl}/blob/main/CONTRIBUTING.md`}>setup guide</a> before
        installing.
      </p>
    </section>
  );
}
