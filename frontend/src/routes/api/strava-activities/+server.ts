import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, request }) => {
  try {
    const authorization = request.headers.get('authorization');
    if (!authorization) {
      return json({ error: 'Missing authorization header' }, { status: 401 });
    }

    // Get query parameters
    const page = url.searchParams.get('page') || '1';
    const perPage = url.searchParams.get('per_page') || '200';

    // Make request to Strava API
    const response = await fetch(
      `https://www.strava.com/api/v3/athlete/activities?page=${page}&per_page=${perPage}`,
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
      console.error('Strava activities fetch failed:', errorText);
      return json({ error: 'Failed to fetch activities' }, { status: response.status });
    }

    const data = await response.json();
    return json(data);
  } catch (error) {
    console.error('Error in Strava activities proxy:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
