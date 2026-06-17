import { defineConfig } from "vite";
import { resolve } from "path";

// Content scripts must be IIFE — no ES module support in isolated world
export default defineConfig({
  build: {
    outDir: "dist",
    emptyOutDir: false,
    lib: {
      entry: resolve(__dirname, "src/content/index.ts"),
      name: "VaultPassContent",
      formats: ["iife"],
      fileName: () => "content.js",
    },
    rollupOptions: {
      output: {
        extend: false,
      },
    },
  },
});
