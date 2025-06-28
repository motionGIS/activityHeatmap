<script lang="ts">
  import { onMount } from 'svelte';
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { browser } from '$app/environment';
  
  // Import Strava service conditionally
  let stravaService: any = null;
  
  // WASM module functions - will be loaded dynamically
  let wasmInit: any = null;
  let processGpxFiles: any = null;
  let processPolylines: any = null;

  let mapContainer: HTMLDivElement;
  let map: maplibregl.Map;
  let isLoading = false;
  let error = '';
  let segmentCount = 0;
  let currentBasemap = 'white';
  let currentHeatmapData: any = null; // Store heatmap data persistently

  // Strava-related state
  let isStravaAuthenticated = false;
  let stravaAthlete: any = null;
  let isImportingStrava = false;
  let stravaActivityCount = 0;
  let stravaImportProgress = 0;
  let cachedActivities: any[] = [];

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
      if (!processGpxFiles) {
        throw new Error('WASM module not loaded');
      }
      const result = processGpxFiles(jsArray);
      const heatmapResult = result as { tracks: any[], max_frequency: number };
      
      console.log('WASM returned', heatmapResult.tracks.length, 'tracks with max frequency:', heatmapResult.max_frequency);
      
      // Use the shared rendering function
      await renderHeatmapResult(heatmapResult);

    } catch (err) {
      error = `Error processing GPX/FIT files: ${err}`;
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  // Strava functions
  function connectToStrava() {
    if (!stravaService) return;
    const authUrl = stravaService.getAuthUrl();
    window.location.href = authUrl;
  }

  function disconnectFromStrava() {
    if (!stravaService) return;
    stravaService.logout();
    isStravaAuthenticated = false;
    stravaAthlete = null;
  }

  async function importStravaActivities() {
    if (!stravaService || !stravaService.isAuthenticated()) {
      error = 'Not connected to Strava';
      return;
    }

    isImportingStrava = true;
    stravaImportProgress = 0;
    error = '';

    try {
      console.log('Fetching Strava activities...');
      
      let activities: any[] = [];
      
      // Check if we have cached activities in localStorage
      if (browser) {
        const cached = localStorage.getItem('strava_activities');
        const cacheTimestamp = localStorage.getItem('strava_activities_timestamp');
        
        // Use cache if it's less than 1 hour old
        const cacheAge = cacheTimestamp ? Date.now() - parseInt(cacheTimestamp) : Infinity;
        const cacheExpiry = 60 * 60 * 1000; // 1 hour in milliseconds
        
        if (cached && cacheAge < cacheExpiry) {
          activities = JSON.parse(cached);
          stravaActivityCount = activities.length;
          stravaImportProgress = 50;
          console.log(`Using cached ${activities.length} activities from localStorage`);
        }
      }
      
      // Fetch fresh data if no valid cache
      if (activities.length === 0) {
        activities = await stravaService.fetchAllActivities((count) => {
          stravaActivityCount = count;
          stravaImportProgress = Math.min(50, count); // First 50% for fetching
        });
        
        // Cache the activities in localStorage
        if (browser && activities.length > 0) {
          localStorage.setItem('strava_activities', JSON.stringify(activities));
          localStorage.setItem('strava_activities_timestamp', Date.now().toString());
          console.log(`Cached ${activities.length} activities to localStorage`);
        }
      }

      console.log(`Processing ${activities.length} activities from Strava`);

      if (activities.length === 0) {
        error = 'No activities found in your Strava account';
        return;
      }

      // Extract polylines from activities
      const polylines = stravaService.getPolylinesFromActivities(activities);
      
      if (polylines.length === 0) {
        error = 'No GPS data found in your Strava activities';
        return;
      }

      console.log(`Processing ${polylines.length} polylines...`);
      stravaImportProgress = 75;

      // Process polylines with our WASM function
      if (!processPolylines) {
        throw new Error('WASM module not loaded');
      }
      const jsArray = new Array();
      for (const polyline of polylines) {
        jsArray.push(polyline);
      }

      const result = processPolylines(jsArray);
      const heatmapResult = result as { tracks: any[], max_frequency: number };
      
      console.log('WASM returned', heatmapResult.tracks.length, 'tracks with max frequency:', heatmapResult.max_frequency);
      
      stravaImportProgress = 90;

      // Use the same rendering logic as GPX files
      await renderHeatmapResult(heatmapResult);
      
      stravaImportProgress = 100;
      console.log('Strava import completed successfully');

    } catch (err) {
      error = `Error importing Strava activities: ${err}`;
      console.error(err);
    } finally {
      isImportingStrava = false;
      stravaImportProgress = 0;
    }
  }

  // Extract heatmap rendering logic into a reusable function
  async function renderHeatmapResult(heatmapResult: { tracks: any[], max_frequency: number }) {
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
      throw new Error('No tracks found in data. Please check that the files contain valid GPS tracks.');
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

    // Store heatmap data persistently
    currentHeatmapData = geojson;

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
  }

  // Calculate opacity based on percentiles for better visual distribution
  function calculateOpacityPercentile(frequency: number, threshold1: number, threshold2: number, threshold3: number, layer: 'hot' | 'medium'): number {
    if (layer === 'hot') {
      // Yellow layer: more generous - show for frequencies > 60th percentile (was 75th)
      const hotThreshold = threshold1 * 0.8; // Lower threshold
      if (frequency <= hotThreshold) {
        return 0;
      }
      // Smooth transition from 60th percentile to max
      const range = threshold3 - hotThreshold;
      if (range <= 0) return 0.8;
      return Math.min(0.9, 0.3 + (frequency - hotThreshold) / range * 0.6);
    } else {
      // Red layer: more generous - show for frequencies > 30th percentile (was 50th)
      const mediumThreshold = threshold1 * 0.6; // Much lower threshold
      if (frequency <= mediumThreshold) {
        return 0;
      }
      // Smooth transition from 30th to 60th percentile
      const range = threshold1 - mediumThreshold;
      if (range <= 0) return 0.6;
      const normalized = Math.min(1, (frequency - mediumThreshold) / range);
      return 0.3 + normalized * 0.4;
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
    currentHeatmapData = null; // Clear stored data
  }

  function switchBasemap(basemapKey: string) {
    if (!map || currentBasemap === basemapKey) return;
    
    currentBasemap = basemapKey;
    const newStyle = basemaps[basemapKey as keyof typeof basemaps].style;
    
    map.setStyle(newStyle);
    
    // Re-add heatmap layers after style loads
    map.once('style.load', () => {
      if (currentHeatmapData) {
        restoreHeatmapLayers(currentHeatmapData);
      }
    });
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

  onMount(async () => {
    try {
      console.log('Starting initialization...');
      
      // Wait a bit to ensure DOM is ready
      await new Promise(resolve => setTimeout(resolve, 100));
      console.log('Map container:', mapContainer);
      
      if (!mapContainer) {
        throw new Error('Map container not found');
      }
      
      // Initialize WASM module dynamically in browser only
      if (browser) {
        const wasmModule = await import('/static/gpx_processor.js');
        await wasmModule.default('/static/gpx_processor_bg.wasm');
        processGpxFiles = wasmModule.process_gpx_files;
        processPolylines = wasmModule.process_polylines;
        console.log('WASM initialized successfully');
      }

      // Create map
      map = new maplibregl.Map({
        container: mapContainer,
        style: basemaps[currentBasemap as keyof typeof basemaps].style,
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

      // Check Strava authentication status
      if (browser) {
        // Import Strava service dynamically to avoid SSR issues
        const { stravaService: service } = await import('../lib/strava');
        stravaService = service;
        
        isStravaAuthenticated = stravaService.isAuthenticated();
        if (isStravaAuthenticated) {
          stravaAthlete = await stravaService.fetchAthleteInfo();
        }
      }

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

  /* Strava Integration Styles */
  .strava-section {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #eee;
  }

  .strava-section h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: #333;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .strava-icon {
    flex-shrink: 0;
  }

  .strava-connect-btn {
    width: 100%;
    background: #fc4c02;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.75rem 1rem;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .strava-connect-btn:hover:not(:disabled) {
    background: #e04402;
  }

  .strava-connect-btn:disabled {
    background: #bdc3c7;
    cursor: not-allowed;
  }

  .strava-info {
    margin-top: 0.5rem;
    font-size: 0.8rem;
    color: #666;
    text-align: center;
  }

  .strava-connected {
    background: #f8f9fa;
    border: 1px solid #e9ecef;
    border-radius: 6px;
    padding: 1rem;
  }

  .athlete-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .athlete-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    object-fit: cover;
  }

  .connection-status {
    font-size: 0.8rem;
    color: #28a745;
  }

  .strava-actions {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .import-btn {
    flex: 1;
    background: #28a745;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    transition: background 0.2s;
  }

  .import-btn:hover:not(:disabled) {
    background: #218838;
  }

  .import-btn:disabled {
    background: #6c757d;
    cursor: not-allowed;
  }

  .disconnect-btn {
    background: #6c757d;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  .disconnect-btn:hover:not(:disabled) {
    background: #5a6268;
  }

  .disconnect-btn:disabled {
    background: #adb5bd;
    cursor: not-allowed;
  }

  .import-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid transparent;
    border-top: 2px solid currentColor;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .import-progress {
    margin-top: 0.5rem;
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background: #e9ecef;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }

  .progress-fill {
    height: 100%;
    background: #fc4c02;
    transition: width 0.3s ease;
  }

  .progress-text {
    font-size: 0.8rem;
    color: #666;
    text-align: center;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>

<div class="controls">
  <!-- Strava Integration Section -->
  {#if browser && stravaService}
    <div class="strava-section">
    {#if !isStravaAuthenticated}
      <button 
        class="strava-connect-btn" 
        on:click={connectToStrava}
        disabled={isLoading || isImportingStrava}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
          <path d="M15.387 17.064l-4.834-10.67h3.731l2.652 6.234 2.652-6.234H23.25l-4.833 10.67c-.25.548-.735.861-1.266.861-.53 0-1.015-.313-1.264-.861z" fill="currentColor"/>
          <path d="M8.83 6.394L4 17.064c-.25.548-.735.861-1.266.861-.53 0-1.015-.313-1.264-.861L.25 6.394h3.73l2.65 6.234L9.28 6.394H8.83z" fill="currentColor"/>
        </svg>
        Connect to Strava
      </button>
      <p class="strava-info">Import all your Strava activities to create a comprehensive heatmap</p>
    {:else}
      <div class="strava-connected">
        <div class="athlete-info">
          {#if stravaAthlete}
            <img src={stravaAthlete.profile} alt="Profile" class="athlete-avatar">
            <div>
              <strong>{stravaAthlete.firstname} {stravaAthlete.lastname}</strong>
              <div class="connection-status">Connected to Strava</div>
            </div>
          {:else}
            <div>
              <strong>Connected to Strava</strong>
              <div class="connection-status">Ready to import activities</div>
            </div>
          {/if}
        </div>
        
        <div class="strava-actions">
          <button 
            class="import-btn" 
            on:click={importStravaActivities}
            disabled={isLoading || isImportingStrava}
          >
            {#if isImportingStrava}
              <div class="import-spinner"></div>
              Importing Activities...
            {:else}
              Import All Activities
            {/if}
          </button>
          
          <button 
            class="disconnect-btn" 
            on:click={disconnectFromStrava}
            disabled={isLoading || isImportingStrava}
          >
            Disconnect
          </button>
        </div>
        
        {#if isImportingStrava}
          <div class="import-progress">
            <div class="progress-bar">
              <div class="progress-fill" style="width: {stravaImportProgress}%"></div>
            </div>
            <div class="progress-text">
              {#if stravaActivityCount > 0}
                Fetched {stravaActivityCount} activities... {stravaImportProgress}%
              {:else}
                Connecting to Strava... {stravaImportProgress}%
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
  {/if}

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
