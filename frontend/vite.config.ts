import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	optimizeDeps: {
		exclude: ['gpx_processor'],
		include: [
			'maplibre-gl',
			'@watergis/terrain-rgb',
			'@watergis/svelte-maplibre-measure',
			'@watergis/svelte-maplibre-style-switcher',
			'@watergis/maplibre-gl-export',
			'svelte-maplibre'
		]
	},
	ssr: {
		noExternal: [
			'@watergis/terrain-rgb',
			'@watergis/svelte-maplibre-measure',
			'@watergis/svelte-maplibre-style-switcher',
			'@watergis/maplibre-gl-export'
		]
	},
	server: {
		fs: {
			allow: ['..']
		}
	},
	build: {
		rollupOptions: {
			external: [],
			output: {
				manualChunks: {
					'watergis': [
						'@watergis/terrain-rgb',
						'@watergis/svelte-maplibre-measure',
						'@watergis/svelte-maplibre-style-switcher',
						'@watergis/maplibre-gl-export'
					]
				}
			}
		},
		commonjsOptions: {
			include: [
				/node_modules/,
				/maplibre-gl/,
				/@watergis/
			],
			transformMixedEsModules: true
		}
	}
});
