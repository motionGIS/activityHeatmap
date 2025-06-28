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
    
    // Build URL without privacy parameter first to test
    const apiUrl = `https://ridewithgps.com/users/current/trips.json?offset=${offset}&limit=${limit}`;
    
    console.log('Making request to:', apiUrl);
    console.log('With access token:', accessToken ? 'present' : 'missing');

    // Make request to RideWithGPS API
    const response = await fetch(apiUrl, {
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Accept': 'application/json',
        'User-Agent': 'ActivityHeatmap/1.0'
      }
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('RideWithGPS trips fetch failed:', {
        status: response.status,
        statusText: response.statusText,
        errorText: errorText,
        url: apiUrl
      });
      return json({ 
        error: 'Failed to fetch trips', 
        details: {
          status: response.status,
          statusText: response.statusText,
          message: errorText
        }
      }, { status: response.status });
    }

    const data = await response.json();
    return json(data);
  } catch (error) {
    console.error('Error in RideWithGPS trips proxy:', error);
    return json({ error: 'Internal server error' }, { status: 500 });
  }
};
