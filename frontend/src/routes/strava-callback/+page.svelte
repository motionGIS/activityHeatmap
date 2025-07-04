<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';

  let status = 'Processing...';

  onMount(async () => {
    if (!browser) return;
    
    try {
      const { stravaService } = await import('../../lib/strava');
      
      const urlParams = new URLSearchParams(window.location.search);
      const code = urlParams.get('code');
      const errorParam = urlParams.get('error');

      if (errorParam || !code) {
        status = `Failed: ${errorParam || 'No code'}`;
        return;
      }

      const success = await stravaService.exchangeCodeForToken(code);
      
      if (success) {
        status = 'Success! Redirecting...';
        setTimeout(() => goto('/'), 1000);
      } else {
        status = 'Failed to authorize';
      }
    } catch (err) {
      status = `Error: ${err}`;
    }
  });
</script>

<p>{status}</p>
{#if status.includes('Failed') || status.includes('Error')}
  <a href="/">Return to app</a>
{/if}
