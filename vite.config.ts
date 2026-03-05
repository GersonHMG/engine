import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	// Prevent vite from obscuring Rust errors
	clearScreen: false,
	server: {
		port: 5173,
		strictPort: true,
		fs: {
			allow: [path.resolve('.')]
		}
	},
	// Env variables starting with TAURI_ will be exposed
	envPrefix: ['VITE_', 'TAURI_']
});
