import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { 
  VITE_STRAVA_CLIENT_ID, 
  VITE_STRAVA_CLIENT_SECRET 
} from '$env/static/private';

export const POST: RequestHandler = async ({ request }) => {
  try {
    const { code } = await request.json();

    if (!code) {
      return json({ error: 'Missing authorization code' }, { status: 400 });
    }

    // Exchange the authorization code for an access token
    const tokenResponse = await fetch('https://www.strava.com/oauth/token', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        client_id: VITE_STRAVA_CLIENT_ID,
        client_secret: VITE_STRAVA_CLIENT_SECRET,
        code: code,
        grant_type: 'authorization_code',
      }),
    });

    if (!tokenResponse.ok) {
      const errorText = await tokenResponse.text();
      console.error('Strava token exchange failed:', errorText);
      return json({ error: 'Token exchange failed' }, { status: tokenResponse.status });
    }

    const tokenData = await tokenResponse.json();
    return json(tokenData);
  } catch (error) {
    console.error('Error in Strava token exchange:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
