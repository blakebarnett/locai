import * as d3 from 'd3';

const ZOOM_CACHE_KEY = 'locai-graph-zoom-transform';
const CACHE_VERSION = '1.0';

interface CachedZoomTransform {
  version: string;
  timestamp: number;
  layoutType: string;
  dataSource: string;
  transform: {
    x: number;
    y: number;
    k: number;
  };
}

export const saveZoomTransform = (
  transform: d3.ZoomTransform, 
  layoutType: string, 
  dataSource: string
): void => {
  try {
    const cacheData: CachedZoomTransform = {
      version: CACHE_VERSION,
      timestamp: Date.now(),
      layoutType,
      dataSource,
      transform: {
        x: transform.x,
        y: transform.y,
        k: transform.k,
      },
    };

    localStorage.setItem(ZOOM_CACHE_KEY, JSON.stringify(cacheData));
    console.log('Saved zoom transform to cache:', cacheData.transform);
  } catch (error) {
    console.warn('Failed to save zoom transform to localStorage:', error);
  }
};

export const loadZoomTransform = (
  layoutType: string, 
  dataSource: string
): d3.ZoomTransform | null => {
  try {
    const cached = localStorage.getItem(ZOOM_CACHE_KEY);
    if (!cached) {
      console.log('No cached zoom transform found');
      return null;
    }

    const cacheData: CachedZoomTransform = JSON.parse(cached);
    
    // Check cache version and age (expire after 1 day)
    const maxAge = 24 * 60 * 60 * 1000; // 1 day in milliseconds
    const isExpired = Date.now() - cacheData.timestamp > maxAge;
    
    if (cacheData.version !== CACHE_VERSION || isExpired) {
      console.log('Cached zoom transform expired or version mismatch');
      localStorage.removeItem(ZOOM_CACHE_KEY);
      return null;
    }

    // Only apply cached transform if layout type and data source match
    if (cacheData.layoutType !== layoutType || cacheData.dataSource !== dataSource) {
      console.log(`Zoom cache mismatch: cached=${cacheData.layoutType}/${cacheData.dataSource}, current=${layoutType}/${dataSource}`);
      return null;
    }

    const transform = d3.zoomIdentity
      .translate(cacheData.transform.x, cacheData.transform.y)
      .scale(cacheData.transform.k);
    
    console.log('Loaded zoom transform from cache:', cacheData.transform);
    return transform;
  } catch (error) {
    console.warn('Failed to load zoom transform from localStorage:', error);
    return null;
  }
};

export const clearZoomCache = (): void => {
  try {
    localStorage.removeItem(ZOOM_CACHE_KEY);
    console.log('Cleared zoom transform cache');
  } catch (error) {
    console.warn('Failed to clear zoom transform cache:', error);
  }
};

export const hasZoomCache = (layoutType: string, dataSource: string): boolean => {
  return loadZoomTransform(layoutType, dataSource) !== null;
}; 