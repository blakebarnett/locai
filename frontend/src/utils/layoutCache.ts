import type { GraphNode } from '../stores/graphStore';

const LAYOUT_CACHE_KEY = 'locai-graph-layout';
const CACHE_VERSION = '1.0';

interface CachedLayout {
  version: string;
  timestamp: number;
  positions: Record<string, { x: number; y: number }>;
  layoutType: string;
}

export const saveNodePositions = (nodes: GraphNode[], layoutType: string): void => {
  try {
    const positions: Record<string, { x: number; y: number }> = {};
    
    nodes.forEach(node => {
      positions[node.id] = {
        x: node.position.x,
        y: node.position.y,
      };
    });

    const cacheData: CachedLayout = {
      version: CACHE_VERSION,
      timestamp: Date.now(),
      positions,
      layoutType,
    };

    localStorage.setItem(LAYOUT_CACHE_KEY, JSON.stringify(cacheData));
    console.log(`Saved ${nodes.length} node positions for layout type: ${layoutType}`);
  } catch (error) {
    console.warn('Failed to save layout to localStorage:', error);
  }
};

export const loadNodePositions = (nodes: GraphNode[], currentLayoutType: string): GraphNode[] => {
  try {
    const cached = localStorage.getItem(LAYOUT_CACHE_KEY);
    if (!cached) {
      console.log('No cached layout found');
      return nodes;
    }

    const cacheData: CachedLayout = JSON.parse(cached);
    
    // Check cache version and age (expire after 7 days)
    const maxAge = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds
    const isExpired = Date.now() - cacheData.timestamp > maxAge;
    
    if (cacheData.version !== CACHE_VERSION || isExpired) {
      console.log('Cached layout expired or version mismatch');
      localStorage.removeItem(LAYOUT_CACHE_KEY);
      return nodes;
    }

    // Only apply cached positions if layout type matches
    if (cacheData.layoutType !== currentLayoutType) {
      console.log(`Layout type mismatch: cached=${cacheData.layoutType}, current=${currentLayoutType}`);
      return nodes;
    }

    // Apply cached positions to nodes that exist in cache
    let appliedCount = 0;
    const updatedNodes = nodes.map(node => {
      const cachedPosition = cacheData.positions[node.id];
      if (cachedPosition) {
        appliedCount++;
        return {
          ...node,
          position: {
            x: cachedPosition.x,
            y: cachedPosition.y,
          },
        };
      }
      return node;
    });
    
    console.log(`Applied cached positions to ${appliedCount}/${nodes.length} nodes for layout type: ${currentLayoutType}`);
    return updatedNodes;
  } catch (error) {
    console.warn('Failed to load layout from localStorage:', error);
    return nodes;
  }
};

export const clearLayoutCache = (): void => {
  try {
    localStorage.removeItem(LAYOUT_CACHE_KEY);
  } catch (error) {
    console.warn('Failed to clear layout cache:', error);
  }
};

export const hasLayoutCache = (layoutType: string): boolean => {
  try {
    const cached = localStorage.getItem(LAYOUT_CACHE_KEY);
    if (!cached) return false;

    const cacheData: CachedLayout = JSON.parse(cached);
    return cacheData.layoutType === layoutType && Object.keys(cacheData.positions).length > 0;
  } catch (error) {
    return false;
  }
}; 