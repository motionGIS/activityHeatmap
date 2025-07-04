import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	optimizeDeps: {
		exclude: ['gpx_processor'],
		include: [
			'maplibre-gl',
			'svelte-maplibre'
		]
	},
	ssr: {
		noExternal: [
		]
	},
	server: {
		fs: {
			allow: ['..']
		}
	},
	build: {
		rollupOptions: {
			external: ['zlib', 'fs', 'path', 'stream', 'util'],
			output: {
				manualChunks: undefined
			}
		},
		commonjsOptions: {
			include: [
				/node_modules/,
				/maplibre-gl/,
			],
			transformMixedEsModules: true
		}
	}
});
