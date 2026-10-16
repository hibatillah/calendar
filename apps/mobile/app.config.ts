import type { ConfigContext, ExpoConfig } from "expo/config";

export default ({ config }: ConfigContext): ExpoConfig => {
  const production = process.env.EAS_BUILD_PROFILE === "production";
  return {
    ...config,
    name: "Calendar",
    slug: "calendar",
    version: "0.1.0",
    ios: { ...config.ios, buildNumber: "1" },
    android: { ...config.android, versionCode: 1 },
    plugins: (config.plugins ?? []).map(plugin =>
      Array.isArray(plugin) && plugin[0] === "expo-build-properties"
        ? [plugin[0], { ...plugin[1], android: { ...plugin[1]?.android, usesCleartextTraffic: !production } }]
        : plugin),
  };
};
