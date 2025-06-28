import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, request }) => {
  try {
    // Try to get token from query parameter first, then from authorization header
    const tokenFromQuery = url.searchParams.get('token');
    const authHeader = request.headers.get('authorization') || request.headers.get('Authorization');
    
    let accessToken = tokenFromQuery;
    if (!accessToken && authHeader) {
      accessToken = authHeader.replace('Bearer ', '');
    }
    
    console.log('Token from query:', tokenFromQuery); // Debug log
    console.log('Auth header:', authHeader); // Debug log
    
    if (!accessToken) {
      console.log('Missing access token'); // Debug log
      return json({ error: 'Missing access token' }, { status: 401 });
    }

    // Make request to RideWithGPS API
    const response = await fetch('https://ridewithgps.com/users/current.json', {
      headers: {
        'Authorization': `Bearer ${accessToken}`,
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
