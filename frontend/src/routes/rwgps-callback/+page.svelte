<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { browser } from '$app/environment';

  let status = 'Processing RideWithGPS authorization...';
  let isError = false;

  onMount(async () => {
    if (!browser) return;

    try {
      // Get authorization code from URL
      const code = $page.url.searchParams.get('code');
      const error = $page.url.searchParams.get('error');

      if (error) {
        status = `Authorization failed: ${error}`;
        isError = true;
        return;
      }

      if (!code) {
        status = 'No authorization code received';
        isError = true;
        return;
      }

      // Import RideWithGPS service and exchange code for token
      const { ridewithgpsService } = await import('../../lib/ridewithgps');
      
      const success = await ridewithgpsService.exchangeCodeForToken(code);
      
      if (success) {
        status = 'Successfully connected to RideWithGPS! Redirecting...';
        setTimeout(() => {
          goto('/');
        }, 2000);
      } else {
        status = 'Failed to connect to RideWithGPS';
        isError = true;
      }
    } catch (err) {
      console.error('RideWithGPS callback error:', err);
      status = `Error: ${err}`;
      isError = true;
    }
  });
</script>

<div class="callback-container">
  <div class="callback-card">
    <h1>RideWithGPS Connection</h1>
    <div class="status" class:error={isError}>
      {status}
    </div>
    {#if !isError}
      <div class="spinner"></div>
    {/if}
    {#if isError}
      <a href="/" class="back-link">Return to App</a>
    {/if}
  </div>
</div>

<style>
  .callback-container {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    background: #f5f5f5;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  .callback-card {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
    text-align: center;
    max-width: 400px;
    width: 90%;
  }

  h1 {
    color: #333;
    margin-bottom: 1rem;
    font-size: 1.5rem;
  }

  .status {
    margin-bottom: 1rem;
    color: #666;
    font-size: 1rem;
    line-height: 1.5;
  }

  .status.error {
    color: #e74c3c;
  }

  .spinner {
    border: 3px solid #f3f3f3;
    border-top: 3px solid #ca4e02;
    border-radius: 50%;
    width: 30px;
    height: 30px;
    animation: spin 1s linear infinite;
    margin: 1rem auto;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .back-link {
    display: inline-block;
    background: #ca4e02;
    color: white;
    padding: 0.75rem 1.5rem;
    text-decoration: none;
    border-radius: 4px;
    transition: background 0.2s;
  }

  .back-link:hover {
    background: #a03e02;
  }
</style>
