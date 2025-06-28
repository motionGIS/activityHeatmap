import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, request }) => {
  try {
    const authorization = request.headers.get('authorization');
    if (!authorization) {
      return json({ error: 'Missing authorization header' }, { status: 401 });
    }

    // Get query parameters
    const offset = url.searchParams.get('offset') || '0';
    const limit = url.searchParams.get('limit') || '100';
    const privacy = url.searchParams.get('privacy') || '0';

    // Make request to RideWithGPS API
    const response = await fetch(
      `https://ridewithgps.com/users/current/trips.json?offset=${offset}&limit=${limit}&privacy=${privacy}`,
      {
        headers: {
          'Authorization': authorization,
          'Accept': 'application/json',
          'User-Agent': 'ActivityHeatmap/1.0'
        }
      }
    );

    if (!response.ok) {
      const errorText = await response.text();
      console.error('RideWithGPS trips fetch failed:', errorText);
      return json({ error: 'Failed to fetch trips' }, { status: response.status });
    }

    const data = await response.json();
    return json(data);
  } catch (error) {
    console.error('Error in RideWithGPS trips proxy:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
