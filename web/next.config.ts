import type { NextConfig } from "next";

const nextConfig: NextConfig = {
	output: "export",
	poweredByHeader: false,
	generateBuildId: async () => "market-time-web",
};

export default nextConfig;
