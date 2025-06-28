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
    
    console.log('Track fetch - Token from query:', tokenFromQuery ? 'present' : 'missing');
    
    if (!accessToken) {
      console.log('Missing access token for track fetch');
      return json({ error: 'Missing access token' }, { status: 401 });
    }

    const trackId = url.searchParams.get('id');
    
    if (!trackId) {
      return json({ error: 'Missing track ID' }, { status: 400 });
    }

    console.log('Fetching track data for ID:', trackId);

    // Try different potential track endpoints
    const endpoints = [
      `https://ridewithgps.com/tracks/${trackId}.json`,
      `https://ridewithgps.com/api/v1/tracks/${trackId}.json`,
      `https://ridewithgps.com/trips/${trackId}/track.json`
    ];

    for (let i = 0; i < endpoints.length; i++) {
      const apiUrl = endpoints[i];
      console.log(`Trying track endpoint ${i + 1}:`, apiUrl);

      const response = await fetch(apiUrl, {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
          'Accept': 'application/json',
          'User-Agent': 'ActivityHeatmap/1.0'
        }
      });

      if (response.ok) {
        console.log(`Success with track endpoint ${i + 1}`);
        const data = await response.json();
        return json(data);
      } else {
        const errorText = await response.text();
        console.log(`Track endpoint ${i + 1} failed:`, {
          status: response.status,
          statusText: response.statusText,
          errorText: errorText
        });
        
        // If this is the last endpoint, return the error
        if (i === endpoints.length - 1) {
          return json({ 
            error: 'Failed to fetch track from all endpoints', 
            details: {
              status: response.status,
              statusText: response.statusText,
              message: errorText,
              lastEndpoint: apiUrl
            }
          }, { status: response.status });
        }
      }
    }
    
    return json({ error: 'All track endpoints failed' }, { status: 500 });
  } catch (error) {
    console.error('Error in RideWithGPS track proxy:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
