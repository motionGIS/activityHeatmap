import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ request }) => {
  try {
    const authorization = request.headers.get('authorization');
    if (!authorization) {
      return json({ error: 'Missing authorization header' }, { status: 401 });
    }

    // Make request to RideWithGPS API
    const response = await fetch('https://ridewithgps.com/users/current.json', {
      headers: {
        'Authorization': authorization,
        'Accept': 'application/json',
        'User-Agent': 'ActivityHeatmap/1.0'
      }
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('RideWithGPS user fetch failed:', errorText);
      return json({ error: 'Failed to fetch user info' }, { status: response.status });
    }

    const data = await response.json();
    return json(data);
  } catch (error) {
    console.error('Error in RideWithGPS user proxy:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
