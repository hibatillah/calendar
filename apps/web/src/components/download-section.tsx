import { marketingButtonClasses } from "@/lib/marketing-styles";
import { ArrowUpRight, Download, Globe, Laptop, Smartphone } from "lucide-react";
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
    <section
      className={`
        download-section max-w-277.5 m-[0_auto_100px] text-center scroll-mt-7.5 [&_>_p]:text-[#868c80]
        [&_>_p]:text-[14px] [&_>_p]:mt-4.5 [.download-page_&]:pt-25 [.download-page_&]:min-h-[65vh]
        max-[1000px]:ml-5 max-[1000px]:mr-5 max-[700px]:m-[0_12px_65px] max-[700px]:[&_h2]:text-[35px]
        max-[700px]:[&_>_p]:text-[12px]
      `}
      id="download"
    >
      <span
        className={`
          section-eyebrow text-[9px] font-[550] tracking-[1.7px] block text-[#718078] mb-[23px]
          [.privacy-strip_&]:mb-[15px] max-[700px]:text-[8px] max-[700px]:tracking-[1.3px] max-[700px]:mb-4.5
        `}
      >
        A PLACE FOR EVERY DAY
      </span>
      <h2>Make yourself some room.</h2>
      <p>Calendar is in preview. Pick your platform to try it.</p>
      {(loading || failed || !release) && (
        <p role="status" aria-live="polite">
          {loading
            ? "Checking the latest release…"
            : failed
              ? "Could not load downloads. Please try again shortly."
              : "The first downloadable release is on its way."}
        </p>
      )}
      <div
        className={`
          platform-grid grid grid-cols-3 gap-3.5 mt-10 text-left [&_article]:[border:1px_solid_#e0e4da]
          [&_article]:rounded-[14px] [&_article]:p-[25px_20px] [&_article]:flex [&_article]:flex-col
          [&_article]:items-start [&_article_>_svg]:text-[#7c8a72] [&_article_>_svg]:mb-5 [&_article_>_svg]:w-6
          [&_h3]:text-[21px] [&_h3]:tracking-[-0.5px] [&_p]:text-[#657369] [&_p]:text-[12px]
          [&_p]:m-[20px_0_25px] [&_p]:flex-1 max-[1000px]:grid-cols-3 max-[700px]:gap-2.5
          max-[700px]:grid-cols-1 max-[700px]:[&_article]:p-[20px_14px] max-[700px]:[&_p]:text-[11px]
          max-[700px]:[&_h3]:text-[19px]
        `}
      >
        {platforms.map((platform) => {
          const downloadUrl =
            platform.key === "web"
              ? "/app"
              : platform.key === "windows"
                ? (release?.assets.windows ?? release?.assets.windowsPortable)
                : release?.assets.android;
          const portableWindows =
            platform.key === "windows" &&
            !release?.assets.windows &&
            !!release?.assets.windowsPortable;

          return (
            <article
              key={platform.key}
              className={`platform-${platform.key} bg-[#ecf7e9] border-[#d0e6c9]!`}
            >
              <platform.icon />
              <h3>{platform.title}</h3>
              <span
                className={`platform-status [border:1px_solid_#e0e4da] bg-[#f2f3ed] rounded-[6px] p-[2px_6px] mt-[7px] text-[9px] text-[#93998c] ${downloadUrl ? " available [.platform-status&]:text-[#667d53] [.platform-status&]:bg-[#ecf0e6]" : ""}`}
              >
                {platform.key === "web"
                  ? "In development"
                  : downloadUrl
                    ? "Developer preview"
                    : "Not available yet"}
              </span>
              <p>
                {portableWindows
                  ? "Portable x64 desktop app and local backend. Extract the ZIP to run it. Unsigned preview; Google OAuth setup is required."
                  : platform.description}
              </p>
              {downloadUrl ? (
                <Button asChild className={marketingButtonClasses}>
                  <a
                    href={downloadUrl}
                    aria-label={
                      portableWindows ? "Download Windows ZIP" : platform.label
                    }
                  >
                    {portableWindows ? "Windows portable ZIP" : platform.buttonText}
                    {platform.key === "web" ? <ArrowUpRight /> : <Download />}
                  </a>
                </Button>
              ) : (
                <span className="unavailable-download text-[#a7ac9f] text-[10px]">
                  No download available
                </span>
              )}
            </article>
          );
        })}
      </div>
      <p className="download-note [.download-section_&]:text-[10px] [.download-section_&]:text-[#9b9f94] [.download-section_&]:mt-5">
        Downloads are public GitHub Release assets. See the{" "}
        <a href={`${repositoryUrl}/blob/main/CONTRIBUTING.md`}>setup guide</a> before
        installing.
      </p>
    </section>
  );
}
