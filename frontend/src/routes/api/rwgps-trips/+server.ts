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
    console.log('Final access token:', accessToken); // Debug log
    
    if (!accessToken) {
      console.log('Missing access token'); // Debug log
      return json({ error: 'Missing access token' }, { status: 401 });
    }

    // Get query parameters
    const offset = url.searchParams.get('offset') || '0';
    const limit = url.searchParams.get('limit') || '100';
    
    // Try different API endpoints that might work for RideWithGPS
    const endpoints = [
      `https://ridewithgps.com/trips.json?offset=${offset}&limit=${limit}`,
      `https://ridewithgps.com/api/v1/trips.json?offset=${offset}&limit=${limit}`,
      `https://ridewithgps.com/users/current/trips.json?offset=${offset}&limit=${limit}`
    ];
    
    console.log('Testing access token:', accessToken ? 'present' : 'missing');

    // Try each endpoint until one works
    for (let i = 0; i < endpoints.length; i++) {
      const apiUrl = endpoints[i];
      console.log(`Trying endpoint ${i + 1}:`, apiUrl);

      const response = await fetch(apiUrl, {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
          'Accept': 'application/json',
          'User-Agent': 'ActivityHeatmap/1.0'
        }
      });

      if (response.ok) {
        console.log(`Success with endpoint ${i + 1}`);
        const data = await response.json();
        return json(data);
      } else {
        const errorText = await response.text();
        console.log(`Endpoint ${i + 1} failed:`, {
          status: response.status,
          statusText: response.statusText,
          errorText: errorText
        });
        
        // If this is the last endpoint, return the error
        if (i === endpoints.length - 1) {
          return json({ 
            error: 'Failed to fetch trips from all endpoints', 
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
    
    // This should never be reached, but just in case
    return json({ error: 'All endpoints failed' }, { status: 500 });
  } catch (error) {
    console.error('Error in RideWithGPS trips proxy:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
