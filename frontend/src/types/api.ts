// Memory types
export interface Memory {
  id: string;
  content: string;
  memory_type: 'Fact' | 'Episodic' | 'Semantic';
  priority: 'Low' | 'Medium' | 'High';
  created_at: string;
  updated_at: string;
  metadata?: Record<string, any>;
}

export interface CreateMemoryRequest {
  content: string;
  memory_type: 'Fact' | 'Episodic' | 'Semantic';
  priority: 'Low' | 'Medium' | 'High';
  metadata?: Record<string, any>;
}

export interface UpdateMemoryRequest {
  content?: string;
  memory_type?: 'Fact' | 'Episodic' | 'Semantic';
  priority?: 'Low' | 'Medium' | 'High';
  metadata?: Record<string, any>;
}

// Entity types
export interface Entity {
  id: string;
  name: string;
  entity_type: string;
  properties: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface CreateEntityRequest {
  name: string;
  entity_type: string;
  properties?: Record<string, any>;
}

export interface UpdateEntityRequest {
  name?: string;
  entity_type?: string;
  properties?: Record<string, any>;
}

// Relationship types
export interface Relationship {
  id: string;
  from_id?: string;
  to_id?: string;
  source_id?: string; // New field name from server  
  target_id?: string; // New field name from server
  relationship_id?: string; // Alternative ID field
  relationship_type: string;
  properties: Record<string, any>;
  created_at?: string;
  node_id?: string; // Nullable node_id from server
}

export interface CreateRelationshipRequest {
  from_id: string;
  to_id: string;
  relationship_type: string;
  properties?: Record<string, any>;
}

// Graph types
export interface GraphNode {
  id: string;
  type: 'memory' | 'entity';
  data: Memory | Entity;
  position?: { x: number; y: number };
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  type: string;
  data: Relationship;
}

export interface GraphResponse {
  center_id: string;
  memories: Memory[];
  entities: Entity[];
  relationships: Relationship[];
  metadata: {
    depth: number;
    total_nodes: number;
    total_edges: number;
  };
}

export interface PathResponse {
  paths: Array<{
    nodes: string[];
    relationships: Relationship[];
    length: number;
  }>;
  metadata: {
    total_paths: number;
    max_depth_reached: number;
  };
}

// Query types
export interface GraphQueryRequest {
  query_type: 'connected' | 'isolated' | 'semantic';
  parameters: Record<string, any>;
  limit?: number;
}

export interface GraphQueryResponse {
  results: Array<{
    nodes: GraphNode[];
    edges: GraphEdge[];
    score?: number;
  }>;
  metadata: {
    query_type: string;
    total_results: number;
    execution_time_ms: number;
  };
}

// Metrics types
export interface GraphMetrics {
  memory_count: number;
  entity_count: number;
  relationship_count: number;
  average_degree: number;
  density: number;
  connected_components: number;
  central_memories: Array<{
    id: string;
    content: string;
    centrality_score: number;
  }>;
  central_entities: Array<{
    id: string;
    name: string;
    centrality_score: number;
  }>;
}

// Search types
export interface SearchResult {
  id: string;
  content: string;
  memory_type: string;
  score: number;
  metadata?: Record<string, any>;
}

// WebSocket types
export interface WebSocketMessage {
  type: 'MemoryCreated' | 'MemoryUpdated' | 'MemoryDeleted' | 
        'EntityCreated' | 'EntityUpdated' | 'EntityDeleted' |
        'RelationshipCreated' | 'RelationshipDeleted';
  data: any;
  timestamp: string;
}

// API Response types
export interface ApiResponse<T> {
  data: T;
  success: boolean;
  message?: string;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// Error types
export interface ApiError {
  error: string;
  message: string;
  status: number;
} 