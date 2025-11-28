import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export",
  images: {
    unoptimized: true,
  },
  // Disable server-side features for Tauri
  trailingSlash: true,
};

export default nextConfig;
