import { defineConfig } from 'vite';

// Only the shared client bundle is built by Vite. src/build/generate.ts then renders every page
// as a real folder/index.html and points it at the hashed files listed in the manifest.
export default defineConfig({
  build: {
    outDir: process.env.OUT_DIR ?? 'report_web',
    emptyOutDir: true,
    manifest: true,
    target: 'es2022',
    rollupOptions: {
      input: 'src/client/main.ts',
      output: {
        entryFileNames: 'assets/[name]-[hash].js',
        chunkFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
    },
  },
});
