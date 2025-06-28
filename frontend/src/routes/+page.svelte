<script lang="ts">
  import { onMount } from 'svelte';
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
      import init, { process_gpx_files } from 'gpx_processor';

  let mapContainer: HTMLDivElement;
  let map: maplibregl.Map;
  let isLoading = false;
  let error = '';
  let segmentCount = 0;
  let currentBasemap = 'osm';
  let showLabels = true;
  let showOutlines = true;

  const basemaps = {
    osm: {
      name: 'OpenStreetMap',
      style: {
        version: 8,
        sources: {
          'raster-tiles': {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '© OpenStreetMap contributors'
          }
        },
        layers: [
          {
            id: 'simple-tiles',
            type: 'raster',
            source: 'raster-tiles',
            minzoom: 0,
            maxzoom: 22
          }
        ]
      }
    },
    white: {
      name: 'White (Protomaps)',
      style: {
        version: 8,
        sources: {
          'protomaps': {
            type: 'raster',
            tiles: ['https://api.protomaps.com/tiles/v3/{z}/{x}/{y}.png?key=YOUR_KEY'],
            tileSize: 256,
            attribution: '© Protomaps'
          },
          'carto-light': {
            type: 'raster', 
            tiles: ['https://a.basemaps.cartocdn.com/light_all/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '© CARTO'
          }
        },
        layers: [
          {
            id: 'white-tiles',
            type: 'raster',
            source: 'carto-light',
            minzoom: 0,
            maxzoom: 22
          }
        ]
      }
    },
    dark: {
      name: 'Dark',
      style: {
        version: 8,
        sources: {
          'carto-dark': {
            type: 'raster',
            tiles: ['https://a.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '© CARTO'
          }
        },
        layers: [
          {
            id: 'dark-tiles',
            type: 'raster',
            source: 'carto-dark',
            minzoom: 0,
            maxzoom: 22
          }
        ]
      }
    },
    satellite: {
      name: 'Satellite',
      style: {
        version: 8,
        sources: {
          'satellite': {
            type: 'raster',
            tiles: ['https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'],
            tileSize: 256,
            attribution: '© Esri'
          }
        },
        layers: [
          {
            id: 'satellite-tiles',
            type: 'raster',
            source: 'satellite',
            minzoom: 0,
            maxzoom: 19
          }
        ]
      }
    }
  };

  async function handleFiles(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    console.log('Processing', input.files.length, 'files');
    isLoading = true;
    error = '';
    
    try {
      // Convert files to Uint8Array buffers
      const buffers = await Promise.all(
        Array.from(input.files).map(f => f.arrayBuffer().then(buf => new Uint8Array(buf)))
      );

      console.log('Converted files to buffers:', buffers.length);

      // Create JS array for WASM function
      const jsArray = new Array();
      for (const buffer of buffers) {
        jsArray.push(buffer);
      }

      console.log('Calling WASM function with', jsArray.length, 'buffers');

      // Process GPX files with WASM
      const result = process_gpx_files(jsArray);
      const heatmapResult = result as { tracks: any[], max_frequency: number };
      
      console.log('WASM returned', heatmapResult.tracks.length, 'tracks with max frequency:', heatmapResult.max_frequency);
      
      // Debug frequency distribution
      const frequencies = heatmapResult.tracks.map(t => t.frequency);
      const freqCounts = frequencies.reduce((acc, freq) => {
        acc[freq] = (acc[freq] || 0) + 1;
        return acc;
      }, {} as Record<number, number>);
      console.log('Frequency distribution:', freqCounts);
      console.log('Unique frequencies:', [...new Set(frequencies)].sort((a, b) => a - b));
      
      // Calculate percentiles for better distribution
      const sortedFreqs = frequencies.slice().sort((a, b) => a - b);
      const p50 = sortedFreqs[Math.floor(sortedFreqs.length * 0.5)];
      const p75 = sortedFreqs[Math.floor(sortedFreqs.length * 0.75)];
      const p90 = sortedFreqs[Math.floor(sortedFreqs.length * 0.9)];
      console.log(`Percentiles: 50th=${p50}, 75th=${p75}, 90th=${p90}, max=${heatmapResult.max_frequency}`);
      
      segmentCount = heatmapResult.tracks.length;

      if (heatmapResult.tracks.length === 0) {
        throw new Error('No tracks found in GPX/FIT files. Please check that the files contain valid GPS tracks.');
      }

      // Convert to GeoJSON with percentile-based opacity
      const geojson = {
        type: "FeatureCollection" as const,
        features: heatmapResult.tracks.map(track => ({
          type: "Feature" as const,
          geometry: {
            type: "LineString" as const,
            coordinates: track.coordinates.map((coord: number[]) => [coord[1], coord[0]]) // [lon, lat]
          },
          properties: { 
            frequency: track.frequency,
            opacity_hot: calculateOpacityPercentile(track.frequency, p75, p90, heatmapResult.max_frequency, 'hot'),
            opacity_medium: calculateOpacityPercentile(track.frequency, p50, p75, p90, 'medium')
          }
        }))
      };

      // Clear existing layers
      ['heatmap-cold', 'heatmap-medium', 'heatmap-hot'].forEach(layerId => {
        if (map.getLayer(layerId)) {
          map.removeLayer(layerId);
        }
      });
      ['heatmap'].forEach(sourceId => {
        if (map.getSource(sourceId)) {
          map.removeSource(sourceId);
        }
      });

      // Add the single source for all layers
      map.addSource('heatmap', {
        type: 'geojson',
        data: geojson
      });

      // Layer 1: Cold (blue, lowest z-index)
      map.addLayer({
        id: 'heatmap-cold',
        type: 'line',
        source: 'heatmap',
        paint: {
          'line-color': '#0066ff', // Blue
          'line-width': [
            'interpolate', 
            ['linear'], 
            ['zoom'],
            1, 2,
            5, 3,
            10, 4,
            15, 6
          ],
          'line-opacity': 0.6
        },
        layout: {
          'line-join': 'round',
          'line-cap': 'round'
        }
      });

      // Layer 2: Medium (red, middle z-index)
      map.addLayer({
        id: 'heatmap-medium',
        type: 'line',
        source: 'heatmap',
        paint: {
          'line-color': '#ff3300', // Red
          'line-width': [
            'interpolate', 
            ['linear'], 
            ['zoom'],
            1, 1.5,
            5, 2.5,
            10, 3.5,
            15, 5.5
          ],
          'line-opacity': ['get', 'opacity_medium']
        },
        layout: {
          'line-join': 'round',
          'line-cap': 'round'
        }
      });

      // Layer 3: Hot (yellow, highest z-index)
      map.addLayer({
        id: 'heatmap-hot',
        type: 'line',
        source: 'heatmap',
        paint: {
          'line-color': '#ffff00', // Yellow
          'line-width': [
            'interpolate', 
            ['linear'], 
            ['zoom'],
            1, 1,
            5, 2,
            10, 3,
            15, 5
          ],
          'line-opacity': ['get', 'opacity_hot']
        },
        layout: {
          'line-join': 'round',
          'line-cap': 'round'
        }
      });
      
      console.log('Strava-style multi-layer heatmap added successfully');

      // Fit map to show all tracks
      if (heatmapResult.tracks.length > 0) {
        const bounds = new maplibregl.LngLatBounds();
        heatmapResult.tracks.forEach(track => {
          track.coordinates.forEach((coord: number[]) => {
            bounds.extend([coord[1], coord[0]]); // [lon, lat]
          });
        });
        map.fitBounds(bounds, { padding: 50 });
      }

    } catch (err) {
      error = `Error processing GPX/FIT files: ${err}`;
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  // Calculate opacity based on percentiles for better visual distribution
  function calculateOpacityPercentile(frequency: number, threshold1: number, threshold2: number, threshold3: number, layer: 'hot' | 'medium'): number {
    if (layer === 'hot') {
      // Yellow layer: only show for frequencies > 75th percentile
      if (frequency <= threshold1) {
        return 0;
      }
      // Smooth transition from 75th percentile to max
      const range = threshold3 - threshold1;
      if (range <= 0) return 0.8;
      return Math.min(0.9, 0.2 + (frequency - threshold1) / range * 0.7);
    } else {
      // Red layer: show for frequencies > 50th percentile
      if (frequency <= threshold1) {
        return 0;
      }
      // Smooth transition from 50th to 75th percentile
      const range = threshold2 - threshold1;
      if (range <= 0) return 0.6;
      const normalized = Math.min(1, (frequency - threshold1) / range);
      return 0.2 + normalized * 0.5;
    }
  }

  // Calculate opacity based on frequency using Strava's power function approach
  function calculateOpacity(frequency: number, maxFrequency: number, layer: 'hot' | 'medium'): number {
    if (maxFrequency <= 1) {
      return layer === 'hot' ? 0 : 0;
    }
    
    const normalized = frequency / maxFrequency;
    
    if (layer === 'hot') {
      // Yellow layer: only show for frequencies > 60% of max
      if (frequency <= maxFrequency * 0.6) {
        return 0;
      }
      return Math.pow((frequency - maxFrequency * 0.6) / (maxFrequency * 0.4), 2) * 0.9;
    } else {
      // Red layer: show for frequencies > 30% of max
      if (frequency <= maxFrequency * 0.3) {
        return 0;
      }
      return Math.pow((frequency - maxFrequency * 0.3) / (maxFrequency * 0.7), 1.2) * 0.7;
    }
  }

  function clearHeatmap() {
    // Remove all heatmap layers
    ['heatmap-cold', 'heatmap-medium', 'heatmap-hot'].forEach(layerId => {
      if (map.getLayer(layerId)) {
        map.removeLayer(layerId);
      }
    });
    
    // Remove all heatmap sources
    ['heatmap'].forEach(sourceId => {
      if (map.getSource(sourceId)) {
        map.removeSource(sourceId);
      }
    });
    
    segmentCount = 0;
  }

  function switchBasemap(basemapKey: string) {
    if (!map || currentBasemap === basemapKey) return;
    
    // Store current heatmap data
    const heatmapSource = map.getSource('heatmap');
    const heatmapData = heatmapSource ? (heatmapSource as maplibregl.GeoJSONSource)._data : null;
    
    currentBasemap = basemapKey;
    const newStyle = basemaps[basemapKey].style;
    
    // Apply labels and outlines settings
    const modifiedStyle = applyLabelAndOutlineSettings(newStyle);
    
    map.setStyle(modifiedStyle);
    
    // Re-add heatmap layers after style loads
    map.once('style.load', () => {
      if (heatmapData) {
        restoreHeatmapLayers(heatmapData);
      }
    });
  }

  function applyLabelAndOutlineSettings(baseStyle: any) {
    let style = JSON.parse(JSON.stringify(baseStyle)); // Deep copy
    
    if (currentBasemap === 'satellite' && (showLabels || showOutlines)) {
      // For satellite, add vector overlay for labels/outlines
      style.sources['vector-overlay'] = {
        type: 'raster',
        tiles: showLabels 
          ? ['https://a.basemaps.cartocdn.com/light_only_labels/{z}/{x}/{y}.png']
          : ['https://a.basemaps.cartocdn.com/light_nolabels/{z}/{x}/{y}.png'],
        tileSize: 256,
        attribution: '© CARTO'
      };
      
      style.layers.push({
        id: 'vector-overlay',
        type: 'raster',
        source: 'vector-overlay',
        minzoom: 0,
        maxzoom: 22,
        paint: {
          'raster-opacity': showOutlines ? 0.6 : (showLabels ? 0.8 : 0)
        }
      });
    }
    
    return style;
  }

  function restoreHeatmapLayers(heatmapData: any) {
    if (!heatmapData) return;
    
    // Add the source
    map.addSource('heatmap', {
      type: 'geojson',
      data: heatmapData
    });

    // Add all three layers in order (same as in handleFiles)
    map.addLayer({
      id: 'heatmap-cold',
      type: 'line',
      source: 'heatmap',
      paint: {
        'line-color': '#0066ff',
        'line-width': [
          'interpolate', 
          ['linear'], 
          ['zoom'],
          1, 2, 5, 3, 10, 4, 15, 6
        ],
        'line-opacity': 0.6
      },
      layout: {
        'line-join': 'round',
        'line-cap': 'round'
      }
    });

    map.addLayer({
      id: 'heatmap-medium',
      type: 'line',
      source: 'heatmap',
      paint: {
        'line-color': '#ff3300',
        'line-width': [
          'interpolate', 
          ['linear'], 
          ['zoom'],
          1, 1.5, 5, 2.5, 10, 3.5, 15, 5.5
        ],
        'line-opacity': ['get', 'opacity_medium']
      },
      layout: {
        'line-join': 'round',
        'line-cap': 'round'
      }
    });

    map.addLayer({
      id: 'heatmap-hot',
      type: 'line',
      source: 'heatmap',
      paint: {
        'line-color': '#ffff00',
        'line-width': [
          'interpolate', 
          ['linear'], 
          ['zoom'],
          1, 1, 5, 2, 10, 3, 15, 5
        ],
        'line-opacity': ['get', 'opacity_hot']
      },
      layout: {
        'line-join': 'round',
        'line-cap': 'round'
      }
    });
  }

  function getHeatmapPaintConfig(tier: string, maxCount: number) {
    const configs = {
      low: {
        paint: {
          'line-color': [
            'interpolate', 
            ['linear'], 
            ['get', 'count'],
            1, '#0066ff',
            Math.floor(maxCount * 0.2), '#3366ff'
          ],
          'line-width': [
            'interpolate', 
            ['linear'], 
            ['zoom'],
            1, 1.5, 5, 2, 10, 3, 15, 4
          ],
          'line-opacity': 0.7
        }
      },
      medium: {
        paint: {
          'line-color': [
            'interpolate', 
            ['linear'], 
            ['get', 'count'],
            Math.floor(maxCount * 0.2), '#9929ff',
            Math.floor(maxCount * 0.6), '#ff6600'
          ],
          'line-width': [
            'interpolate', 
            ['linear'], 
            ['zoom'],
            1, 2, 5, 3, 10, 5, 15, 7
          ],
          'line-opacity': 0.8
        }
      },
      high: {
        paint: {
          'line-color': [
            'interpolate', 
            ['linear'], 
            ['get', 'count'],
            Math.floor(maxCount * 0.6), '#ff8800',
            maxCount, '#ffff00'
          ],
          'line-width': [
            'interpolate', 
            ['linear'], 
            ['zoom'],
            1, 3, 5, 5, 10, 8, 15, 12
          ],
          'line-opacity': 0.9
        }
      }
    };
    
    return configs[tier] || configs.low;
  }

  function toggleLabels() {
    showLabels = !showLabels;
    switchBasemap(currentBasemap); // Refresh the map with new settings
  }

  function toggleOutlines() {
    showOutlines = !showOutlines;
    switchBasemap(currentBasemap); // Refresh the map with new settings
  }

  onMount(async () => {
    try {
      console.log('Starting initialization...');
      
      // Wait a bit to ensure DOM is ready
      await new Promise(resolve => setTimeout(resolve, 100));
      console.log('Map container:', mapContainer);
      
      if (!mapContainer) {
        throw new Error('Map container not found');
      }
      
      // Initialize WASM module with explicit path to WASM file
      await init('/gpx_processor_bg.wasm');
      console.log('WASM initialized successfully');

      // Create map
      map = new maplibregl.Map({
        container: mapContainer,
        style: basemaps[currentBasemap].style,
        center: [-96, 37.8],
        zoom: 3
      });

      console.log('Map created:', map);

      // Add navigation controls
      map.addControl(new maplibregl.NavigationControl());

      // Add load event listener to check if map loads
      map.on('load', () => {
        console.log('Map loaded successfully');
      });

      map.on('error', (e) => {
        console.error('Map error:', e);
      });

      map.on('style.load', () => {
        console.log('Map style loaded');
      });

    } catch (err) {
      error = `Failed to initialize: ${err}`;
      console.error('Initialization error:', err);
    }
  });
</script>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  #map {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 100%;
    height: 100vh;
  }

  .controls {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 1000;
    background: white;
    padding: 1rem;
    border-radius: 8px;
    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
    max-width: 350px;
    min-width: 300px;
  }

  .upload-section {
    margin-bottom: 1rem;
  }

  .file-input {
    width: 100%;
    padding: 0.5rem;
    border: 2px dashed #ccc;
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.2s;
  }

  .file-input:hover {
    border-color: #007acc;
  }

  .status {
    margin-top: 0.5rem;
    font-size: 0.9rem;
  }

  .loading {
    color: #007acc;
  }

  .error {
    color: #e74c3c;
  }

  .success {
    color: #27ae60;
  }

  .clear-btn {
    background: #e74c3c;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .clear-btn:hover {
    background: #c0392b;
  }

  .clear-btn:disabled {
    background: #bdc3c7;
    cursor: not-allowed;
  }

  .basemap-controls {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #eee;
  }

  .basemap-controls h4 {
    margin: 0 0 0.5rem 0;
    font-size: 0.9rem;
    color: #333;
  }

  .basemap-options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
  }

  .basemap-btn {
    background: #f8f9fa;
    border: 1px solid #dee2e6;
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
    color: #495057;
  }

  .basemap-btn:hover {
    background: #e9ecef;
    border-color: #adb5bd;
  }

  .basemap-btn.active {
    background: #007acc;
    border-color: #007acc;
    color: white;
  }

  .basemap-btn:disabled {
    background: #e9ecef;
    border-color: #dee2e6;
    color: #6c757d;
    cursor: not-allowed;
  }

  .overlay-controls {
    margin-top: 0.5rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    margin-bottom: 0.25rem;
    font-size: 0.8rem;
    color: #555;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    margin-right: 0.5rem;
    cursor: pointer;
  }

  .legend {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #eee;
  }

  .legend h4 {
    margin: 0 0 0.5rem 0;
    font-size: 0.9rem;
    color: #333;
  }

  .legend-item {
    display: flex;
    align-items: center;
    margin-bottom: 0.25rem;
    font-size: 0.8rem;
  }

  .legend-color {
    width: 20px;
    height: 3px;
    margin-right: 0.5rem;
  }

  h1 {
    margin: 0 0 1rem 0;
    font-size: 1.2rem;
    color: #333;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: #555;
  }
</style>

<div class="controls">
  <h1>GPX/FIT Heatmap Viewer</h1>
  
  <div class="upload-section">
    <label for="gpx-files">Upload GPX/FIT Files:</label>
    <input 
      id="gpx-files"
      type="file" 
      multiple 
      accept=".gpx,.fit" 
      class="file-input"
      on:change={handleFiles}
      disabled={isLoading}
    >
    
    <div class="status">
      {#if isLoading}
        <div class="loading">Processing GPX/FIT files...</div>
      {:else if error}
        <div class="error">{error}</div>
      {:else if segmentCount > 0}
        <div class="success">
          Loaded {segmentCount} route segments
        </div>
      {/if}
    </div>
  </div>

  {#if segmentCount > 0}
    <button 
      class="clear-btn" 
      on:click={clearHeatmap}
      disabled={isLoading}
    >
      Clear Heatmap
    </button>
  {/if}

  <div class="basemap-controls">
    <h4>Basemap</h4>
    <div class="basemap-options">
      {#each Object.entries(basemaps) as [key, basemap]}
        <button 
          class="basemap-btn {currentBasemap === key ? 'active' : ''}"
          on:click={() => switchBasemap(key)}
          disabled={isLoading}
        >
          {basemap.name}
        </button>
      {/each}
    </div>
    
    {#if currentBasemap === 'satellite'}
      <div class="overlay-controls">
        <label class="checkbox-label">
          <input 
            type="checkbox" 
            bind:checked={showLabels}
            on:change={toggleLabels}
            disabled={isLoading}
          >
          Show Labels
        </label>
        <label class="checkbox-label">
          <input 
            type="checkbox" 
            bind:checked={showOutlines}
            on:change={toggleOutlines}
            disabled={isLoading}
          >
          Show Outlines
        </label>
      </div>
    {/if}
  </div>

  <div class="legend">
    <h4>Heat Intensity</h4>
    <div class="legend-item">
      <div class="legend-color" style="background-color: #0066ff;"></div>
      <span>Base level (all routes)</span>
    </div>
    <div class="legend-item">
      <div class="legend-color" style="background-color: #ff3300;"></div>
      <span>Medium usage</span>
    </div>
    <div class="legend-item">
      <div class="legend-color" style="background-color: #ffff00;"></div>
      <span>High usage</span>
    </div>
    <p style="font-size: 0.75rem; color: #666; margin-top: 0.5rem;">
      Routes are shown with overlapping layers to create heat effect
    </p>
  </div>
</div>

<div bind:this={mapContainer} id="map"></div>
