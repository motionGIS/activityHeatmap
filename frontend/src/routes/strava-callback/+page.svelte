<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';

  let loading = true;
  let error = '';
  let success = false;

  onMount(async () => {
    if (!browser) return;
    
    try {
      // Import Strava service dynamically to avoid SSR issues
      const { stravaService } = await import('../../lib/strava');
      
      // Get the authorization code from URL parameters
      const urlParams = new URLSearchParams(window.location.search);
      const code = urlParams.get('code');
      const errorParam = urlParams.get('error');

      if (errorParam) {
        error = `Strava authorization failed: ${errorParam}`;
        loading = false;
        return;
      }

      if (!code) {
        error = 'No authorization code received from Strava';
        loading = false;
        return;
      }

      // Exchange code for access token
      const tokenSuccess = await stravaService.exchangeCodeForToken(code);
      
      if (tokenSuccess) {
        success = true;
        // Redirect back to main page after a short delay
        setTimeout(() => {
          goto('/');
        }, 2000);
      } else {
        error = 'Failed to complete Strava authorization';
      }
    } catch (err) {
      error = `Error during Strava authorization: ${err}`;
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>Strava Authorization</title>
</svelte:head>

<div class="callback-container">
  <div class="callback-content">
    <h1>Strava Authorization</h1>
    
    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <p>Completing Strava authorization...</p>
      </div>
    {:else if success}
      <div class="success">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
          <polyline points="22,4 12,14.01 9,11.01"></polyline>
        </svg>
        <h2>Success!</h2>
        <p>You've been successfully connected to Strava.</p>
        <p>Redirecting back to the heatmap viewer...</p>
      </div>
    {:else if error}
      <div class="error">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="15" y1="9" x2="9" y2="15"></line>
          <line x1="9" y1="9" x2="15" y2="15"></line>
        </svg>
        <h2>Authorization Failed</h2>
        <p>{error}</p>
        <a href="/" class="back-button">Go Back to Heatmap Viewer</a>
      </div>
    {/if}
  </div>
</div>

<style>
  .callback-container {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  .callback-content {
    background: white;
    padding: 3rem;
    border-radius: 12px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
    text-align: center;
    max-width: 400px;
    width: 90%;
  }

  h1 {
    margin: 0 0 2rem 0;
    color: #333;
    font-size: 1.8rem;
  }

  h2 {
    margin: 1rem 0;
    color: #333;
    font-size: 1.4rem;
  }

  .loading {
    color: #666;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid #f3f3f3;
    border-top: 4px solid #fc4c02;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem auto;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .success {
    color: #27ae60;
  }

  .success svg {
    color: #27ae60;
    margin-bottom: 1rem;
  }

  .error {
    color: #e74c3c;
  }

  .error svg {
    color: #e74c3c;
    margin-bottom: 1rem;
  }

  .back-button {
    display: inline-block;
    margin-top: 1.5rem;
    padding: 0.75rem 1.5rem;
    background: #007acc;
    color: white;
    text-decoration: none;
    border-radius: 6px;
    font-weight: 500;
    transition: background 0.2s;
  }

  .back-button:hover {
    background: #005a99;
  }

  p {
    margin: 0.5rem 0;
    color: #666;
    line-height: 1.5;
  }
</style>
