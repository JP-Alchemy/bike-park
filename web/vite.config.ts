import { defineConfig } from "vite";

export default defineConfig({
  // Relative paths so the build can be hosted from any folder (own domain or portal).
  base: "./",
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 1500,
  },
  server: {
    host: true,
  },
});
