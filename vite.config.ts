import path from "node:path";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error provided by Node types at build time
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;
const src = path.resolve(process.cwd(), "src");

export default defineConfig(() => ({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": src,
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
