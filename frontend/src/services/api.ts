import type {
  Memory,
  Entity,
  Relationship,
  CreateMemoryRequest,
  UpdateMemoryRequest,
  CreateEntityRequest,
  UpdateEntityRequest,
  CreateRelationshipRequest,
  GraphResponse,
  PathResponse,
  GraphQueryRequest,
  GraphQueryResponse,
  GraphMetrics,
  SearchResult,
  PaginatedResponse,
  ApiError,
} from '../types/api';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3000';

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const config: RequestInit = {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    };

    try {
      const response = await fetch(url, config);
      
      if (!response.ok) {
        const errorData: ApiError = await response.json().catch(() => ({
          error: 'Unknown error',
          message: `HTTP ${response.status}: ${response.statusText}`,
          status: response.status,
        }));
        throw new Error(errorData.message || `HTTP ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error('Network error occurred');
    }
  }

  // Memory endpoints
  async getMemories(page = 0, size = 1000): Promise<Memory[]> {
    const timestamp = Date.now();
    return this.request(`/api/memories?page=${page}&size=${size}&_bust=${timestamp}`);
  }

  async getMemory(id: string): Promise<Memory> {
    return this.request(`/api/memories/${id}`);
  }

  async createMemory(memory: CreateMemoryRequest): Promise<Memory> {
    return this.request('/api/memories', {
      method: 'POST',
      body: JSON.stringify(memory),
    });
  }

  async updateMemory(id: string, memory: UpdateMemoryRequest): Promise<Memory> {
    return this.request(`/api/memories/${id}`, {
      method: 'PUT',
      body: JSON.stringify(memory),
    });
  }

  async deleteMemory(id: string): Promise<void> {
    return this.request(`/api/memories/${id}`, {
      method: 'DELETE',
    });
  }

  async searchMemories(query: string, limit = 10): Promise<SearchResult[]> {
    return this.request(`/api/memories/search?q=${encodeURIComponent(query)}&limit=${limit}`);
  }

  // Entity endpoints
  async getEntities(page = 0, size = 1000): Promise<Entity[]> {
    const timestamp = Date.now();
    return this.request(`/api/entities?page=${page}&size=${size}&_bust=${timestamp}`);
  }

  async getEntity(id: string): Promise<Entity> {
    return this.request(`/api/entities/${id}`);
  }

  async createEntity(entity: CreateEntityRequest): Promise<Entity> {
    return this.request('/api/entities', {
      method: 'POST',
      body: JSON.stringify(entity),
    });
  }

  async updateEntity(id: string, entity: UpdateEntityRequest): Promise<Entity> {
    return this.request(`/api/entities/${id}`, {
      method: 'PUT',
      body: JSON.stringify(entity),
    });
  }

  async deleteEntity(id: string): Promise<void> {
    return this.request(`/api/entities/${id}`, {
      method: 'DELETE',
    });
  }

  async getRelatedEntities(id: string, relationshipType?: string): Promise<Entity[]> {
    const params = relationshipType ? `?relationship_type=${encodeURIComponent(relationshipType)}` : '';
    return this.request(`/api/entities/${id}/related_entities${params}`);
  }

  async getEntityMemories(id: string): Promise<Memory[]> {
    return this.request(`/api/entities/${id}/memories`);
  }

  async getCentralEntities(limit = 10): Promise<Entity[]> {
    return this.request(`/api/entities/central?limit=${limit}`);
  }

  // Relationship endpoints  
  async getRelationships(page = 0, size = 1000): Promise<Relationship[]> {
    // Use proper pagination to ensure we get all relationships
    const timestamp = Date.now();
    return this.request(`/api/relationships?page=${page}&size=${size}&_bust=${timestamp}`);
  }

  async getRelationship(id: string): Promise<Relationship> {
    return this.request(`/api/relationships/${id}`);
  }

  async createRelationship(relationship: CreateRelationshipRequest): Promise<Relationship> {
    return this.request('/api/relationships', {
      method: 'POST',
      body: JSON.stringify(relationship),
    });
  }

  async deleteRelationship(id: string): Promise<void> {
    return this.request(`/api/relationships/${id}`, {
      method: 'DELETE',
    });
  }

  // Graph endpoints
  async getMemoryGraph(id: string, depth = 2): Promise<GraphResponse> {
    return this.request(`/api/memories/${id}/graph?depth=${depth}`);
  }

  async getEntityGraph(id: string, depth = 2): Promise<GraphResponse> {
    return this.request(`/api/entities/${id}/graph?depth=${depth}`);
  }

  async findPaths(fromId: string, toId: string, maxDepth = 5): Promise<PathResponse> {
    return this.request(`/api/graph/paths?from=${fromId}&to=${toId}&max_depth=${maxDepth}`);
  }

  async queryGraph(query: GraphQueryRequest): Promise<GraphQueryResponse> {
    return this.request('/api/graph/query', {
      method: 'POST',
      body: JSON.stringify(query),
    });
  }

  async getSimilarStructures(patternId: string, limit = 10): Promise<GraphResponse[]> {
    return this.request(`/api/graph/similar_structures?pattern=${patternId}&limit=${limit}`);
  }

  async getGraphMetrics(): Promise<GraphMetrics> {
    return this.request('/api/graph/metrics');
  }

  // WebSocket connection
  createWebSocketConnection(): WebSocket {
    const wsUrl = this.baseUrl.replace(/^http/, 'ws') + '/api/ws';
    return new WebSocket(wsUrl);
  }
}

export const apiClient = new ApiClient();
export default apiClient;