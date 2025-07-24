import { create } from 'zustand';
import type { Node, Edge } from 'reactflow';
import type { 
  Memory, 
  Entity, 
  Relationship, 
  GraphResponse, 
  GraphMetrics,
  WebSocketMessage
} from '../types/api';
import { apiClient } from '../services/api';
import { webSocketManager } from '../services/websocket';
import { calculateForceLayout, calculateHierarchicalLayout, calculateCircularLayout } from '../utils/graphLayout';
import { saveNodePositions, loadNodePositions, hasLayoutCache } from '../utils/layoutCache';
import type { ToastProps } from '../components/Toast';

const DATA_SOURCE_CACHE_KEY = 'locai-graph-data-source';

// Helper functions for data source caching
const saveDataSource = (dataSource: 'demo' | 'server' | null): void => {
  try {
    if (dataSource) {
      localStorage.setItem(DATA_SOURCE_CACHE_KEY, dataSource);
    } else {
      localStorage.removeItem(DATA_SOURCE_CACHE_KEY);
    }
  } catch (error) {
    console.warn('Failed to save data source to localStorage:', error);
  }
};

const loadDataSource = (): 'demo' | 'server' | null => {
  try {
    const cached = localStorage.getItem(DATA_SOURCE_CACHE_KEY);
    return cached as 'demo' | 'server' | null;
  } catch (error) {
    console.warn('Failed to load data source from localStorage:', error);
    return null;
  }
};

export interface GraphNode {
  id: string;
  type: 'memoryNode' | 'entityNode';
  position: { x: number; y: number };
  data: {
    id: string;
    type: 'memory' | 'entity';
    content: Memory | Entity;
    centrality?: number;
    degree: number;
  };
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  type: string;
  label?: string;
  data: Relationship;
}

interface GraphState {
  // Graph data
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNodes: string[];
  selectedEdges: string[];
  
  // UI state
  isLoading: boolean;
  error: string | null;
  connectionState: string;
  dataSource: 'demo' | 'server' | null;
  
  // Graph metrics
  metrics: GraphMetrics | null;
  
  // Search and filters
  searchQuery: string;
  activeFilters: {
    memoryTypes: string[];
    entityTypes: string[];
    relationshipTypes: string[];
  };
  showTemporalRelationships: boolean;
  
  // Graph layout
  layoutType: 'force' | 'hierarchical' | 'circular';
  showLabels: boolean;
  nodeSize: 'uniform' | 'centrality' | 'degree';
  
  // Dense network management
  maxVisibleEdges: number;
  edgeVisibilityThreshold: number;
  autoHideEdges: boolean;
  
  // Toast notifications
  toasts: ToastProps[];
  
  // Simple real-time event tracking
  realtimeFeed: Array<{
    id: string;
    type: string;
    timestamp: Date;
    description: string;
    data: any;
  }>;
  
  // Actions
  setNodes: (nodes: GraphNode[]) => void;
  setEdges: (edges: GraphEdge[]) => void;
  addNode: (node: GraphNode) => void;
  addEdge: (edge: GraphEdge) => void;
  removeNode: (nodeId: string) => void;
  removeEdge: (edgeId: string) => void;
  updateNode: (nodeId: string, updates: Partial<GraphNode>) => void;
  
  // Selection
  selectNode: (nodeId: string) => void;
  selectEdge: (edgeId: string) => void;
  clearSelection: () => void;
  
  // Graph operations
  loadMemoryGraph: (memoryId: string, depth?: number) => Promise<void>;
  loadEntityGraph: (entityId: string, depth?: number) => Promise<void>;
  expandNode: (nodeId: string, depth?: number) => Promise<void>;
  
  // Real-time updates
  initializeWebSocket: () => void;
  handleWebSocketMessage: (message: WebSocketMessage) => void;
  
  // Metrics
  loadMetrics: () => Promise<void>;
  
  // Search and filters
  setSearchQuery: (query: string) => void;
  setFilters: (filters: Partial<GraphState['activeFilters']>) => void;
  setShowTemporalRelationships: (show: boolean) => void;
  
  // Layout
  setLayoutType: (layout: GraphState['layoutType']) => void;
  setShowLabels: (show: boolean) => void;
  setNodeSize: (size: GraphState['nodeSize']) => void;
  applyLayout: (width?: number, height?: number) => Promise<void>;
  
  // Toast notifications
  addToast: (toast: Omit<ToastProps, 'onClose'>) => void;
  removeToast: (id: string) => void;
  
  // Error handling
  setError: (error: string | null) => void;
  setLoading: (loading: boolean) => void;
  
  // Load all data from locai-server for the main memory explorer
  loadAllData: () => Promise<void>;
  
  // Load demo or server data explicitly
  loadDemoData: () => void;
  loadServerData: () => Promise<void>;
  
  // Feed management
  addRealtimeEvent: (event: { type: string; description: string; data: any }) => void;
  clearRealtimeFeed: () => void;
}

// Enhanced convertToGraphNodes with connectivity calculation
const convertToGraphNodes = (memories: Memory[], entities: Entity[], relationships: Relationship[] = [], layoutType: string = 'force'): GraphNode[] => {
  const nodes: GraphNode[] = [];
  
  // Calculate connectivity (degree) for each node
  const connectivity = new Map<string, number>();
  
  // Count connections from relationships
  relationships.forEach(rel => {
    const sourceId = rel.source_id || rel.from_id || '';
    const targetId = rel.target_id || rel.to_id || '';
    
    if (sourceId) {
      connectivity.set(sourceId, (connectivity.get(sourceId) || 0) + 1);
    }
    if (targetId) {
      connectivity.set(targetId, (connectivity.get(targetId) || 0) + 1);
    }
  });
  
  // Get viewport dimensions from window or use defaults
  const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 1200;
  const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800;
  
  // Account for left panel (approximate) - position nodes in visible area
  const visibleStartX = viewportWidth * 0.4; // Start after left panel
  const visibleWidth = viewportWidth * 0.6;  // Use right 60% of screen
  const visibleCenterX = visibleStartX + visibleWidth / 2;
  const visibleCenterY = viewportHeight / 2;

  memories.forEach((memory) => {
    const degree = connectivity.get(memory.id) || 0;
    nodes.push({
      id: memory.id,
      type: 'memoryNode',
      position: { 
        x: visibleCenterX + (Math.random() - 0.5) * 300, // Random around visible center
        y: visibleCenterY + (Math.random() - 0.5) * 400 
      },
      data: {
        id: memory.id,
        type: 'memory',
        content: memory,
        degree,
      },
    });
  });
  
  entities.forEach((entity) => {
    const degree = connectivity.get(entity.id) || 0;
    nodes.push({
      id: entity.id,
      type: 'entityNode',
      position: { 
        x: visibleCenterX + (Math.random() - 0.5) * 300, // Random around visible center
        y: visibleCenterY + (Math.random() - 0.5) * 400 
      },
      data: {
        id: entity.id,
        type: 'entity',
        content: entity,
        degree,
      },
    });
  });
  
  // Try to load cached positions
  return loadNodePositions(nodes, layoutType);
};

const convertToGraphEdges = (relationships: Relationship[], showTemporalRelationships: boolean = false): GraphEdge[] => {
  console.log('🔍 Converting relationships to graph edges:', {
    totalRelationships: relationships.length,
    relationshipTypes: relationships.map(r => r.relationship_type),
    sampleRelationship: relationships[0],
    showTemporalRelationships
  });
  
  // Define temporal relationship types that can be toggled off  
  // These are sequence-based relationships that show chronological order
  const temporalRelationshipTypes = ['follows', 'precedes', 'before', 'after', 'during', 'temporal_sequence'];
  
  // Filter out temporal relationships if the toggle is off
  const filteredRelationships = showTemporalRelationships 
    ? relationships 
    : relationships.filter(rel => !temporalRelationshipTypes.includes(rel.relationship_type.toLowerCase()));
  
  console.log('🔍 Relationship filtering:', {
    originalCount: relationships.length,
    filteredCount: filteredRelationships.length,
    removedCount: relationships.length - filteredRelationships.length,
    showTemporalRelationships,
    temporalTypesRemoved: relationships
      .filter(rel => temporalRelationshipTypes.includes(rel.relationship_type.toLowerCase()))
      .map(rel => rel.relationship_type),
    allRelationshipTypes: [...new Set(relationships.map(rel => rel.relationship_type))]
  });
  
  const edges = filteredRelationships.map(rel => {
    const sourceId = rel.source_id || rel.from_id || '';
    const targetId = rel.target_id || rel.to_id || '';
    
    const edge = {
      id: rel.id,
      source: sourceId,
      target: targetId,
      type: 'smoothstep',
      label: rel.relationship_type,
      data: rel,
    };
    
    // Log mentions relationships specifically
    if (rel.relationship_type === 'mentions') {
      console.log('🎯 MENTIONS relationship found:', {
        id: rel.id,
        sourceId,
        targetId,
        edge
      });
    }
    
    return edge;
  }).filter(edge => {
    const hasSourceAndTarget = edge.source && edge.target;
    if (!hasSourceAndTarget) {
      console.warn('⚠️ Filtering out edge with missing source/target:', edge);
    }
    return hasSourceAndTarget;
  });
  
  const mentionsEdges = edges.filter(e => e.label === 'mentions');
  console.log('🎯 FINAL mentions edges after filtering:', mentionsEdges.length, mentionsEdges);
  
  return edges;
};

// Create sample connections for demonstration when no real relationships exist
const createSampleConnections = (nodes: GraphNode[]): GraphEdge[] => {
  const edges: GraphEdge[] = [];
  const memoryNodes = nodes.filter(n => n.type === 'memoryNode');
  const entityNodes = nodes.filter(n => n.type === 'entityNode');
  
  // Connect some memories to entities based on content similarity
  memoryNodes.forEach((memory, index) => {
    if (entityNodes.length > 0) {
      // Connect each memory to 1-2 entities
      const entityIndex = index % entityNodes.length;
      const entity = entityNodes[entityIndex];
      
      edges.push({
        id: `sample-edge-${memory.id}-${entity.id}`,
        source: memory.id,
        target: entity.id,
        type: 'relationship',
        label: 'relates_to',
        data: {
          id: `sample-rel-${memory.id}-${entity.id}`,
          from_id: memory.id,
          to_id: entity.id,
          relationship_type: 'relates_to',
          properties: { auto_generated: true },
          created_at: new Date().toISOString()
        }
      });
      
      // Add some cross-memory connections
      if (index > 0) {
        const prevMemory = memoryNodes[index - 1];
        edges.push({
          id: `sample-edge-${prevMemory.id}-${memory.id}`,
          source: prevMemory.id,
          target: memory.id,
          type: 'relationship',
          label: 'follows',
          data: {
            id: `sample-rel-${prevMemory.id}-${memory.id}`,
            from_id: prevMemory.id,
            to_id: memory.id,
            relationship_type: 'follows',
            properties: { auto_generated: true },
            created_at: new Date().toISOString()
          }
        });
      }
    }
  });
  
  // Connect entities to each other in small clusters
  entityNodes.forEach((entity, index) => {
    if (index < entityNodes.length - 1) {
      const nextEntity = entityNodes[index + 1];
      edges.push({
        id: `sample-edge-${entity.id}-${nextEntity.id}`,
        source: entity.id,
        target: nextEntity.id,
        type: 'relationship',
        label: 'connected_to',
        data: {
          id: `sample-rel-${entity.id}-${nextEntity.id}`,
          from_id: entity.id,
          to_id: nextEntity.id,
          relationship_type: 'connected_to',
          properties: { auto_generated: true },
          created_at: new Date().toISOString()
        }
      });
    }
  });
  
  console.log(`Created ${edges.length} sample connections`);
  return edges;
};

export const useGraphStore = create<GraphState>((set, get) => ({
  // Initial state
  nodes: [],
  edges: [],
  selectedNodes: [],
  selectedEdges: [],
  isLoading: false,
  error: null,
  connectionState: 'disconnected',
  dataSource: loadDataSource() || 'demo', // Load cached data source or default to demo
  metrics: null,
  searchQuery: '',
  activeFilters: {
    memoryTypes: [],
    entityTypes: [],
    relationshipTypes: [],
  },
  // Temporal relationship control - off by default to reduce visual clutter
  showTemporalRelationships: false,
  layoutType: 'circular',
  showLabels: true,
  nodeSize: 'uniform',
  
  // Dense network management defaults
  maxVisibleEdges: 300,
  edgeVisibilityThreshold: 0.5,
  autoHideEdges: true,
  
  toasts: [],
  
  // Simple real-time event tracking
  realtimeFeed: [],

  // Basic setters
  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),
  setError: (error) => set({ error }),
  setLoading: (loading) => set({ isLoading: loading }),

  // Node/Edge manipulation
  addNode: (node) => set((state) => ({
    nodes: [...state.nodes, node]
  })),

  addEdge: (edge) => set((state) => ({
    edges: [...state.edges, edge]
  })),

  removeNode: (nodeId) => set((state) => ({
    nodes: state.nodes.filter(n => n.id !== nodeId),
    edges: state.edges.filter(e => e.source !== nodeId && e.target !== nodeId),
    selectedNodes: state.selectedNodes.filter(id => id !== nodeId),
  })),

  removeEdge: (edgeId) => set((state) => ({
    edges: state.edges.filter(e => e.id !== edgeId),
    selectedEdges: state.selectedEdges.filter(id => id !== edgeId),
  })),

  updateNode: (nodeId, updates) => set((state) => {
    const updatedNodes = state.nodes.map(node => 
      node.id === nodeId ? { ...node, ...updates } : node
    );
    
    // Save positions to cache when nodes are updated
    if (updates.position) {
      saveNodePositions(updatedNodes, state.layoutType);
    }
    
    return { nodes: updatedNodes };
  }),

  // Selection
  selectNode: (nodeId) => set((state) => ({
    selectedNodes: state.selectedNodes.includes(nodeId) 
      ? state.selectedNodes.filter(id => id !== nodeId)
      : [...state.selectedNodes, nodeId]
  })),

  selectEdge: (edgeId) => set((state) => ({
    selectedEdges: state.selectedEdges.includes(edgeId)
      ? state.selectedEdges.filter(id => id !== edgeId)
      : [...state.selectedEdges, edgeId]
  })),

  clearSelection: () => set({ selectedNodes: [], selectedEdges: [] }),

  // Graph loading
  loadMemoryGraph: async (memoryId, depth = 2) => {
    set({ isLoading: true, error: null });
    try {
      const { layoutType, showTemporalRelationships } = get();
      const response = await apiClient.getMemoryGraph(memoryId, depth);
      const nodes = convertToGraphNodes(response.memories, response.entities, response.relationships, layoutType);
      const edges = convertToGraphEdges(response.relationships, showTemporalRelationships);
      set({ nodes, edges, isLoading: false });
    } catch (error) {
      set({ 
        error: error instanceof Error ? error.message : 'Failed to load memory graph',
        isLoading: false 
      });
    }
  },

  loadEntityGraph: async (entityId, depth = 2) => {
    set({ isLoading: true, error: null });
    try {
      const { layoutType, showTemporalRelationships } = get();
      const response = await apiClient.getEntityGraph(entityId, depth);
      const nodes = convertToGraphNodes(response.memories, response.entities, response.relationships, layoutType);
      const edges = convertToGraphEdges(response.relationships, showTemporalRelationships);
      set({ nodes, edges, isLoading: false });
    } catch (error) {
      set({ 
        error: error instanceof Error ? error.message : 'Failed to load entity graph',
        isLoading: false 
      });
    }
  },

  // Load all data from locai-server for the main memory explorer
  loadAllData: async () => {
    set({ isLoading: true, error: null, dataSource: 'server' });
    try {
      const { layoutType, showTemporalRelationships } = get();
      
      // Fetch memories, entities, and relationships from locai-server
      // Note: locai-server returns arrays directly, not wrapped in pagination objects
      // The API client ignores pagination parameters and fetches ALL data
      const [memories, entities, relationships] = await Promise.all([
        apiClient.getMemories(), // Get ALL memories
        apiClient.getEntities(), // Get ALL entities
        apiClient.getRelationships() // Get ALL relationships
      ]);
      
      const nodes = convertToGraphNodes(memories, entities, relationships, layoutType);
      const edges = convertToGraphEdges(relationships, showTemporalRelationships);
      
      console.log('Loaded data:', { 
        memoriesCount: memories.length, 
        entitiesCount: entities.length, 
        relationshipsCount: relationships.length,
        nodesCount: nodes.length,
        edgesCount: edges.length 
      });
      
      // Debug: Log some sample data to understand the structure
      if (memories.length > 0) console.log('Sample memory:', memories[0]);
      if (entities.length > 0) console.log('Sample entity:', entities[0]);
      if (relationships.length > 0) console.log('Sample relationship:', relationships[0]);
      if (relationships.length === 0) console.log('No relationships found in data');
      
      // Debug: Check specifically for mentions relationships
      const mentionsRels = relationships.filter(r => r.relationship_type === 'mentions');
      console.log('🎯 MENTIONS relationships in API data:', {
        count: mentionsRels.length,
        relationships: mentionsRels.map(r => ({
          id: r.id,
          source_id: r.source_id,
          target_id: r.target_id,
          type: r.relationship_type
        }))
      });
      
      // Debug: Check for ID mismatches
      if (relationships.length > 0) {
        const memoryIds = new Set(memories.map(m => m.id));
        const entityIds = new Set(entities.map(e => e.id));
        const allNodeIds = new Set([...memoryIds, ...entityIds]);
        
        const invalidRels = relationships.filter(rel => {
          const sourceId = rel.source_id || rel.from_id;
          const targetId = rel.target_id || rel.to_id;
          return !sourceId || !targetId || !allNodeIds.has(sourceId) || !allNodeIds.has(targetId);
        });
        
        console.log('Node ID sets:', {
          memoryIds: Array.from(memoryIds).slice(0, 5),
          entityIds: Array.from(entityIds).slice(0, 5),
          totalNodes: allNodeIds.size
        });
        
        console.log(`Found ${invalidRels.length} relationships with missing nodes out of ${relationships.length} total`);
        
        // Specific debug for mentions relationships
        const invalidMentionsRels = mentionsRels.filter(rel => {
          const sourceId = rel.source_id || rel.from_id;
          const targetId = rel.target_id || rel.to_id;
          return !sourceId || !targetId || !allNodeIds.has(sourceId) || !allNodeIds.has(targetId);
        });
        
        console.log('🎯 Invalid MENTIONS relationships:', {
          count: invalidMentionsRels.length,
          invalidRels: invalidMentionsRels.map(rel => ({
            id: rel.id,
            sourceId: rel.source_id,
            targetId: rel.target_id,
            sourceExists: allNodeIds.has(rel.source_id || ''),
            targetExists: allNodeIds.has(rel.target_id || ''),
            type: rel.relationship_type
          }))
        });
        
        if (invalidRels.length > 0) {
          const sampleRel = invalidRels[0];
          const sampleSourceId = sampleRel.source_id || sampleRel.from_id;
          const sampleTargetId = sampleRel.target_id || sampleRel.to_id;
          
          console.log('Sample invalid relationship:', sampleRel);
          console.log('Missing source?', !sampleSourceId || !allNodeIds.has(sampleSourceId));
          console.log('Missing target?', !sampleTargetId || !allNodeIds.has(sampleTargetId));
        }
      }
      
      set({ 
        nodes, 
        edges, 
        isLoading: false,
        connectionState: 'connected'
      });
      
      // Show success toast
      get().addToast({
        id: `data-loaded-${Date.now()}`,
        type: 'success',
        title: 'Data Loaded from Locai',
        message: `Loaded ${memories.length} memories, ${entities.length} entities, ${relationships.length} relationships`,
        duration: 3000,
      });
      
      // Auto-apply layout if no cached layout exists
      if (!hasLayoutCache(layoutType)) {
        // For dense networks, use larger canvas and automatically switch to hierarchical
        const isDenseNetwork = relationships.length > 500;
        const layoutWidth = isDenseNetwork ? 1600 : 1200;
        const layoutHeight = isDenseNetwork ? 1000 : 800;
        
        if (isDenseNetwork) {
          console.log(`Dense network detected (${relationships.length} relationships), using larger canvas and hierarchical layout`);
        }
        
        setTimeout(() => {
          get().applyLayout(layoutWidth, layoutHeight);
        }, 100);
      }
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to load data from locai-server';
      set({ 
        error: errorMessage,
        isLoading: false,
        connectionState: 'error'
      });
      
      // Show error toast
      get().addToast({
        id: `data-error-${Date.now()}`,
        type: 'error',
        title: 'Failed to Load Data',
        message: 'Could not connect to locai-server. Make sure it\'s running on http://localhost:3000 with --no-auth',
        duration: 5000,
      });
    }
  },

  // Load demo data explicitly
  loadDemoData: async () => {
    set({ 
      isLoading: true, 
      error: null, 
      connectionState: 'demo',
      dataSource: 'demo' 
    });
    
    // Save data source selection
    saveDataSource('demo');
    
    // Sample data for demonstration
    const sampleMemories: Memory[] = [
      {
        id: 'mem-1',
        content: 'The quarterly review meeting highlighted strong performance in customer satisfaction metrics.',
        memory_type: 'Episodic',
        priority: 'High',
        created_at: '2024-01-15T10:00:00Z',
        updated_at: '2024-01-15T10:00:00Z',
      },
      {
        id: 'mem-2',
        content: 'Machine learning models can improve prediction accuracy through ensemble methods.',
        memory_type: 'Fact',
        priority: 'Medium',
        created_at: '2024-01-14T15:30:00Z',
        updated_at: '2024-01-14T15:30:00Z',
      },
      {
        id: 'mem-3',
        content: 'Knowledge management systems facilitate organizational learning and information sharing.',
        memory_type: 'Semantic',
        priority: 'Medium',
        created_at: '2024-01-13T09:15:00Z',
        updated_at: '2024-01-13T09:15:00Z',
      },
      {
        id: 'mem-4',
        content: 'The product launch was successful with initial sales exceeding projections by 20%.',
        memory_type: 'Episodic',
        priority: 'High',
        created_at: '2024-01-12T14:00:00Z',
        updated_at: '2024-01-12T14:00:00Z',
      },
    ];

    const sampleEntities: Entity[] = [
      {
        id: 'ent-1',
        name: 'Customer Satisfaction',
        entity_type: 'concept',
        properties: { domain: 'business metrics' },
        created_at: '2024-01-15T10:00:00Z',
        updated_at: '2024-01-15T10:00:00Z',
      },
      {
        id: 'ent-2',
        name: 'Machine Learning',
        entity_type: 'technology',
        properties: { domain: 'artificial intelligence' },
        created_at: '2024-01-14T15:30:00Z',
        updated_at: '2024-01-14T15:30:00Z',
      },
      {
        id: 'ent-3',
        name: 'Knowledge Management',
        entity_type: 'system',
        properties: { domain: 'information systems' },
        created_at: '2024-01-13T09:15:00Z',
        updated_at: '2024-01-13T09:15:00Z',
      },
      {
        id: 'ent-4',
        name: 'Product Launch',
        entity_type: 'event',
        properties: { domain: 'business operations' },
        created_at: '2024-01-12T14:00:00Z',
        updated_at: '2024-01-12T14:00:00Z',
      },
    ];

    const sampleRelationships: Relationship[] = [
      {
        id: 'rel-1',
        from_id: 'mem-1',
        to_id: 'ent-1',
        source_id: 'mem-1',
        target_id: 'ent-1',
        relationship_type: 'references',
        properties: { context: 'performance review' },
        created_at: '2024-01-15T10:00:00Z',
      },
      {
        id: 'rel-2',
        from_id: 'mem-2',
        to_id: 'ent-2',
        source_id: 'mem-2',
        target_id: 'ent-2',
        relationship_type: 'describes',
        properties: { aspect: 'methodology' },
        created_at: '2024-01-14T15:30:00Z',
      },
      // Additional relationships for connected network...
      {
        id: 'rel-5',
        from_id: 'mem-1',
        to_id: 'ent-4',
        source_id: 'mem-1',
        target_id: 'ent-4',
        relationship_type: 'references',
        properties: { context: 'product impact on satisfaction' },
        created_at: '2024-01-15T10:00:00Z',
      },
    ];

    const { layoutType, showTemporalRelationships } = get();
    const nodes = convertToGraphNodes(sampleMemories, sampleEntities, sampleRelationships, layoutType);
    const edges = convertToGraphEdges(sampleRelationships, showTemporalRelationships);

    set({ 
      nodes, 
      edges, 
      isLoading: false,
      connectionState: 'demo'
    });

    // Show success toast
    get().addToast({
      id: `demo-loaded-${Date.now()}`,
      type: 'info',
      title: 'Demo Data Loaded',
      message: `Loaded ${sampleMemories.length} memories, ${sampleEntities.length} entities, ${sampleRelationships.length} relationships`,
      duration: 3000,
    });
  },

  // Load server data explicitly  
  loadServerData: async () => {
    set({ 
      isLoading: true, 
      error: null,
      dataSource: 'server' 
    });
    
    // Save data source selection
    saveDataSource('server');
    
    // Just call loadAllData since it already sets dataSource to 'server'
    get().loadAllData();
  },

  expandNode: async (nodeId, depth = 1) => {
    const { nodes } = get();
    const node = nodes.find(n => n.id === nodeId);
    if (!node) return;

    set({ isLoading: true });
    try {
      let response: GraphResponse;
      if (node.data.type === 'memory') {
        response = await apiClient.getMemoryGraph(nodeId, depth);
      } else {
        response = await apiClient.getEntityGraph(nodeId, depth);
      }

      const { layoutType, showTemporalRelationships } = get();
      const newNodes = convertToGraphNodes(response.memories, response.entities, response.relationships, layoutType);
      const newEdges = convertToGraphEdges(response.relationships, showTemporalRelationships);

      // Merge with existing nodes/edges, avoiding duplicates
      const existingNodeIds = new Set(nodes.map(n => n.id));
      const existingEdgeIds = new Set(get().edges.map(e => e.id));

      const nodesToAdd = newNodes.filter(n => !existingNodeIds.has(n.id));
      const edgesToAdd = newEdges.filter(e => !existingEdgeIds.has(e.id));

      set((state) => ({
        nodes: [...state.nodes, ...nodesToAdd],
        edges: [...state.edges, ...edgesToAdd],
        isLoading: false,
      }));
    } catch (error) {
      set({ 
        error: error instanceof Error ? error.message : 'Failed to expand node',
        isLoading: false 
      });
    }
  },

  // WebSocket
  initializeWebSocket: () => {
    webSocketManager.on('connected', () => {
      set({ connectionState: 'connected' });
    });

    webSocketManager.on('disconnected', () => {
      set({ connectionState: 'disconnected' });
    });

    webSocketManager.on('message', (message: WebSocketMessage) => {
      get().handleWebSocketMessage(message);
    });

    webSocketManager.connect().catch(error => {
      console.error('Failed to connect WebSocket:', error);
      set({ connectionState: 'error' });
    });
  },

  handleWebSocketMessage: (message) => {
    const { addToast, nodes, addNode, updateNode } = get();
    
    try {
      console.log('Handling WebSocket message:', message);

      switch (message.type) {
        case 'MemoryCreated':
          if (message.data) {
            const memory: Memory = message.data;
            
            // Check if this node already exists to avoid duplicates
            const existingNode = nodes.find(n => n.id === memory.id);
            if (!existingNode) {
              // Position new nodes in visible area (account for left panel)
              const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 1200;
              const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800;
              const visibleCenterX = viewportWidth * 0.7; // Center of visible area
              const visibleCenterY = viewportHeight * 0.5;
              
              const newNode: GraphNode = {
                id: memory.id,
                type: 'memoryNode',
                position: { 
                  x: visibleCenterX + (Math.random() - 0.5) * 200, 
                  y: visibleCenterY + (Math.random() - 0.5) * 200 
                },
                data: {
                  id: memory.id,
                  type: 'memory',
                  content: memory,
                  degree: 0,
                },
              };
              addNode(newNode);
              
              addToast({
                id: `memory-created-${Date.now()}`,
                type: 'info',
                title: 'New Memory Created',
                message: memory.content.substring(0, 50) + '...',
                duration: 3000,
              });
            }
          }
          break;
          
        case 'EntityCreated':
          if (message.data) {
            const entity: Entity = message.data;
            
            // Check if this node already exists to avoid duplicates
            const existingNode = nodes.find(n => n.id === entity.id);
            if (!existingNode) {
              // Position new nodes in visible area (account for left panel)
              const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 1200;
              const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800;
              const visibleCenterX = viewportWidth * 0.7; // Center of visible area
              const visibleCenterY = viewportHeight * 0.5;
              
              const newNode: GraphNode = {
                id: entity.id,
                type: 'entityNode',
                position: { 
                  x: visibleCenterX + (Math.random() - 0.5) * 200, 
                  y: visibleCenterY + (Math.random() - 0.5) * 200 
                },
                data: {
                  id: entity.id,
                  type: 'entity',
                  content: entity,
                  degree: 0,
                },
              };
              addNode(newNode);
              
              addToast({
                id: `entity-created-${Date.now()}`,
                type: 'info',
                title: 'New Entity Created',
                message: `${entity.name} (${entity.entity_type})`,
                duration: 3000,
              });
            }
          }
          break;

        case 'RelationshipCreated':
          if (message.data) {
            const relationshipData = message.data;
            
            // Map the WebSocket data to our Relationship interface
            const relationship: Relationship = {
              id: relationshipData.relationship_id || relationshipData.id,
              from_id: relationshipData.source_id,
              to_id: relationshipData.target_id,
              source_id: relationshipData.source_id,
              target_id: relationshipData.target_id,
              relationship_type: relationshipData.relationship_type,
              properties: relationshipData.properties || {},
              node_id: relationshipData.node_id,
            };
            
            // Check if this edge already exists to avoid duplicates
            const existingEdge = get().edges.find(e => e.id === relationship.id);
            if (!existingEdge) {
              const sourceId = relationship.source_id || relationship.from_id;
              const targetId = relationship.target_id || relationship.to_id;
              
              console.log('Creating relationship edge:', {
                id: relationship.id,
                sourceId,
                targetId,
                type: relationship.relationship_type
              });
              
              if (sourceId && targetId) {
                const newEdge: GraphEdge = {
                  id: relationship.id,
                  source: sourceId,
                  target: targetId,
                  type: 'relationship',
                  label: relationship.relationship_type,
                  data: relationship,
                };
                get().addEdge(newEdge);
                
                addToast({
                  id: `relationship-created-${Date.now()}`,
                  type: 'success',
                  title: 'New Relationship Formed',
                  message: `${relationship.relationship_type}`,
                  duration: 3000,
                });
              } else {
                console.warn('Failed to create edge - missing source or target:', {
                  sourceId,
                  targetId,
                  relationshipData
                });
              }
            }
          }
          break;

        case 'MemoryUpdated':
          if (message.data) {
            const memory: Memory = message.data;
            updateNode(memory.id, {
              data: {
                id: memory.id,
                type: 'memory',
                content: memory,
                degree: 0,
              },
            });
          }
          break;
          
        case 'EntityUpdated':
          if (message.data) {
            const entity: Entity = message.data;
            updateNode(entity.id, {
              data: {
                id: entity.id,
                type: 'entity',
                content: entity,
                degree: 0,
              },
            });
          }
          break;

        case 'MemoryDeleted':
          if (message.data?.id) {
            get().removeNode(message.data.id);
          }
          break;
          
        case 'EntityDeleted':
          if (message.data?.id) {
            get().removeNode(message.data.id);
          }
          break;

        default:
          console.log('Unknown WebSocket message type:', message.type);
      }
    } catch (error) {
      console.error('Error handling WebSocket message:', error);
    }
  },

  // Metrics
  loadMetrics: async () => {
    try {
      const metrics = await apiClient.getGraphMetrics();
      set({ metrics });
    } catch (error) {
      console.error('Failed to load metrics:', error);
    }
  },

  // Search and filters
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setFilters: (filters) => set((state) => ({
    activeFilters: { ...state.activeFilters, ...filters }
  })),
  setShowTemporalRelationships: (showTemporalRelationships) => {
    console.log(`🔄 Temporal relationships toggle changed to: ${showTemporalRelationships}`);
    set({ showTemporalRelationships });
    // Reload the current data source with the new filter setting
    const { dataSource, loadDemoData, loadServerData } = get();
    console.log(`🔄 Reloading ${dataSource} data with temporal filter: ${showTemporalRelationships}`);
    if (dataSource === 'demo') {
      loadDemoData();
    } else if (dataSource === 'server') {
      loadServerData();
    }
  },

  // Layout
  setLayoutType: (layoutType) => {
    console.log(`🔄 Layout type changed to: ${layoutType}`);
    set({ layoutType });
  },
  setShowLabels: (showLabels) => set({ showLabels }),
  setNodeSize: (nodeSize) => {
    console.log(`🔄 Node size strategy changed to: ${nodeSize}`);
    set({ nodeSize });
  },
  
  applyLayout: async (width = 1400, height = 900) => {
    const { nodes, edges, layoutType } = get();
    if (nodes.length === 0) return;

    set({ isLoading: true });
    
    try {
      let updatedNodes: GraphNode[];
      
      // For dense networks, automatically use hierarchical layout as default
      const density = edges.length / (nodes.length || 1);
      const effectiveLayoutType = density > 10 && layoutType === 'force' ? 'hierarchical' : layoutType;
      
      if (effectiveLayoutType !== layoutType) {
        console.log(`Auto-switching from ${layoutType} to ${effectiveLayoutType} layout for dense network (${edges.length} edges, ${nodes.length} nodes)`);
      }
      
      switch (effectiveLayoutType) {
        case 'force':
          updatedNodes = await calculateForceLayout(nodes, edges, width, height);
          break;
        case 'hierarchical':
          updatedNodes = calculateHierarchicalLayout(nodes, edges, width, height);
          break;
        case 'circular':
          updatedNodes = calculateCircularLayout(nodes, edges, width, height);
          break;
        default:
          updatedNodes = nodes;
      }
      
      // Save the new positions to localStorage
      saveNodePositions(updatedNodes, layoutType);
      
      // Show success toast with density info
      const densityInfo = edges.length > 500 ? ` (dense network: ${edges.length} relationships)` : '';
      get().addToast({
        id: `layout-applied-${Date.now()}`,
        type: 'success',
        title: `${effectiveLayoutType.charAt(0).toUpperCase() + effectiveLayoutType.slice(1)} Layout Applied`,
        message: `Positioned ${updatedNodes.length} nodes${densityInfo}`,
        duration: 3000,
      });
      
      set({ nodes: updatedNodes, isLoading: false });
    } catch (error) {
      console.error('Layout calculation failed:', error);
      set({ isLoading: false });
      
      // Show error toast for dense networks
      get().addToast({
        id: `layout-error-${Date.now()}`,
        type: 'error',
        title: 'Layout Failed',
        message: 'Try switching to hierarchical layout for dense networks',
        duration: 5000,
      });
    }
  },

  // Toast notifications
  addToast: (toast) => set((state) => ({
    toasts: [...state.toasts, { ...toast, onClose: () => {} }]
  })),

  removeToast: (id) => set((state) => ({
    toasts: state.toasts.filter(toast => toast.id !== id)
  })),

  // Feed management
  addRealtimeEvent: (event) => {
    set((state) => ({
      realtimeFeed: [{
        id: `event-${Date.now()}`,
        type: event.type,
        timestamp: new Date(),
        description: event.description,
        data: event.data
      }, ...state.realtimeFeed].slice(0, 100) // Keep only last 100 events
    }));
  },

  clearRealtimeFeed: () => {
    set((state) => ({
      realtimeFeed: []
    }));
  },
})); 