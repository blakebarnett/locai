import * as d3 from 'd3';

export interface D3Node extends d3.SimulationNodeDatum {
  id: string;
  type: 'memoryNode' | 'entityNode';
  data: {
    id: string;
    type: 'memory' | 'entity';
    content: any;
    centrality?: number;
  };
  selected?: boolean;
  radius?: number;
  color?: string;
  cluster?: string;
}

export interface D3Link extends d3.SimulationLinkDatum<D3Node> {
  id: string;
  source: D3Node | string;
  target: D3Node | string;
  type: string;
  label?: string;
  data: any;
  selected?: boolean;
  strength?: number;
}

export interface LayoutConfig {
  width: number;
  height: number;
  nodeSize: 'uniform' | 'centrality' | 'degree';
  clustering: boolean;
  animation: boolean;
  forces: {
    charge: number;
    link: number;
    collision: number;
    center: number;
  };
}

export class D3GraphLayoutEngine {
  private simulation: d3.Simulation<D3Node, D3Link> | null = null;
  private config: LayoutConfig;

  constructor(config: LayoutConfig) {
    this.config = config;
  }

  /**
   * Create and configure a force simulation for the graph
   */
  createSimulation(nodes: D3Node[], links: D3Link[]): d3.Simulation<D3Node, D3Link> {
    const { width, height, forces } = this.config;

    this.simulation = d3.forceSimulation<D3Node, D3Link>(nodes)
      .force('link', d3.forceLink<D3Node, D3Link>(links)
        .id(d => d.id)
        .distance(d => this.calculateLinkDistance(d))
        .strength(forces.link))
      .force('charge', d3.forceManyBody()
        .strength(forces.charge)
        .distanceMax(width * 0.8))
      .force('center', d3.forceCenter(width / 2, height / 2)
        .strength(forces.center))
      .force('collision', d3.forceCollide<D3Node>()
        .radius(d => this.calculateNodeRadius(d) + 5)
        .strength(forces.collision))
      .force('boundary', this.createBoundaryForce(width, height));

    // Add clustering force if enabled
    if (this.config.clustering) {
      this.simulation.force('cluster', this.createClusteringForce());
    }

    return this.simulation;
  }

  /**
   * Apply hierarchical layout with memory nodes at top, entities at bottom
   */
  applyHierarchicalLayout(nodes: D3Node[], links: D3Link[]): void {
    const memoryNodes = nodes.filter(n => n.type === 'memoryNode');
    const entityNodes = nodes.filter(n => n.type === 'entityNode');

    // Position memory nodes in upper region
    const memoryCols = Math.ceil(Math.sqrt(memoryNodes.length));
    memoryNodes.forEach((node, i) => {
      const row = Math.floor(i / memoryCols);
      const col = i % memoryCols;
      node.x = (this.config.width / (memoryCols + 1)) * (col + 1);
      node.y = this.config.height * 0.25 + row * 60;
      node.fx = node.x; // Fix position initially
      node.fy = node.y;
    });

    // Position entity nodes in lower region
    const entityCols = Math.ceil(Math.sqrt(entityNodes.length));
    entityNodes.forEach((node, i) => {
      const row = Math.floor(i / entityCols);
      const col = i % entityCols;
      node.x = (this.config.width / (entityCols + 1)) * (col + 1);
      node.y = this.config.height * 0.75 + row * 60;
      node.fx = node.x;
      node.fy = node.y;
    });

    // Release fixed positions after layout stabilizes
    setTimeout(() => {
      nodes.forEach(node => {
        node.fx = null;
        node.fy = null;
      });
    }, 1000);
  }

  /**
   * Apply circular layout with clusters
   */
  applyCircularLayout(nodes: D3Node[], links: D3Link[]): void {
    // Group nodes by type or cluster
    const groups = this.groupNodesByCluster(nodes);
    const centerX = this.config.width / 2;
    const centerY = this.config.height / 2;
    const mainRadius = Math.min(this.config.width, this.config.height) * 0.3;

    Object.entries(groups).forEach(([cluster, clusterNodes], groupIndex) => {
      const groupAngle = (2 * Math.PI * groupIndex) / Object.keys(groups).length;
      const groupCenterX = centerX + mainRadius * Math.cos(groupAngle);
      const groupCenterY = centerY + mainRadius * Math.sin(groupAngle);
      const clusterRadius = Math.min(100, clusterNodes.length * 15);

      clusterNodes.forEach((node, i) => {
        const nodeAngle = (2 * Math.PI * i) / clusterNodes.length;
        node.x = groupCenterX + clusterRadius * Math.cos(nodeAngle);
        node.y = groupCenterY + clusterRadius * Math.sin(nodeAngle);
      });
    });
  }

  /**
   * Apply force-directed layout with enhanced physics
   */
  applyForceLayout(nodes: D3Node[], links: D3Link[]): d3.Simulation<D3Node, D3Link> {
    const simulation = this.createSimulation(nodes, links);
    
    // Add heat-diffusion for better distribution
    simulation.force('heat', this.createHeatDiffusionForce());
    
    return simulation;
  }

  /**
   * Calculate optimal node radius based on configuration
   */
  private calculateNodeRadius(node: D3Node): number {
    const baseRadius = 20;
    
    switch (this.config.nodeSize) {
      case 'centrality':
        return baseRadius + (node.data.centrality || 0) * 15;
      case 'degree':
        // Would need degree calculation
        return baseRadius + 5;
      case 'uniform':
      default:
        return baseRadius;
    }
  }

  /**
   * Calculate optimal link distance based on node types and relationships
   */
  private calculateLinkDistance(link: D3Link): number {
    const source = link.source as D3Node;
    const target = link.target as D3Node;
    
    // Shorter distance for same-type nodes
    if (source.type === target.type) {
      return 60;
    }
    
    // Longer distance for cross-type relationships
    return 100;
  }

  /**
   * Create boundary force to keep nodes within viewport
   */
  private createBoundaryForce(width: number, height: number) {
    const padding = 50;
    
    return (alpha: number) => {
      if (!this.simulation) return;
      
      this.simulation.nodes().forEach(node => {
        if (node.x! < padding) node.vx! += (padding - node.x!) * alpha;
        if (node.x! > width - padding) node.vx! += (width - padding - node.x!) * alpha;
        if (node.y! < padding) node.vy! += (padding - node.y!) * alpha;
        if (node.y! > height - padding) node.vy! += (height - padding - node.y!) * alpha;
      });
    };
  }

  /**
   * Create clustering force to group related nodes
   */
  private createClusteringForce() {
    return (alpha: number) => {
      if (!this.simulation) return;
      
      const nodes = this.simulation.nodes();
      const clusters = this.groupNodesByCluster(nodes);
      
      Object.values(clusters).forEach(clusterNodes => {
        if (clusterNodes.length < 2) return;
        
        // Calculate cluster centroid
        const centroidX = d3.mean(clusterNodes, d => d.x!) || 0;
        const centroidY = d3.mean(clusterNodes, d => d.y!) || 0;
        
        // Pull nodes toward cluster center
        clusterNodes.forEach(node => {
          const dx = centroidX - node.x!;
          const dy = centroidY - node.y!;
          node.vx! += dx * alpha * 0.1;
          node.vy! += dy * alpha * 0.1;
        });
      });
    };
  }

  /**
   * Create heat diffusion force for better node distribution
   */
  private createHeatDiffusionForce() {
    return (alpha: number) => {
      if (!this.simulation) return;
      
      const nodes = this.simulation.nodes();
      const quadtree = d3.quadtree<D3Node>()
        .x(d => d.x!)
        .y(d => d.y!)
        .addAll(nodes);
      
      nodes.forEach(node => {
        let totalHeat = 0;
        let totalWeight = 0;
        
        quadtree.visit((quad, x1, y1, x2, y2) => {
          const quadLeaf = quad as d3.QuadtreeLeaf<D3Node>;
          if (!quadLeaf.data) return false;
          
          const other = quadLeaf.data;
          if (other === node) return false;
          
          const dx = other.x! - node.x!;
          const dy = other.y! - node.y!;
          const distance = Math.sqrt(dx * dx + dy * dy);
          
          if (distance < 100) {
            const weight = 1 / (distance + 1);
            totalHeat += weight;
            totalWeight += weight;
          }
          
          return false;
        });
        
        if (totalWeight > 0) {
          const avgHeat = totalHeat / totalWeight;
          const coolForce = (avgHeat - 0.5) * alpha * 0.05;
          
          // Apply cooling force (spread out from hot areas)
          node.vx! += Math.random() * coolForce - coolForce / 2;
          node.vy! += Math.random() * coolForce - coolForce / 2;
        }
      });
    };
  }

  /**
   * Group nodes by cluster for layout purposes
   */
  private groupNodesByCluster(nodes: D3Node[]): Record<string, D3Node[]> {
    const groups: Record<string, D3Node[]> = {};
    
    nodes.forEach(node => {
      const cluster = node.cluster || node.type;
      if (!groups[cluster]) {
        groups[cluster] = [];
      }
      groups[cluster].push(node);
    });
    
    return groups;
  }

  /**
   * Update configuration and restart simulation
   */
  updateConfig(newConfig: Partial<LayoutConfig>): void {
    this.config = { ...this.config, ...newConfig };
    
    if (this.simulation) {
      // Update forces with new configuration
      const { forces } = this.config;
      
      this.simulation
        .force('charge', d3.forceManyBody().strength(forces.charge))
        .force('link', (this.simulation.force('link') as d3.ForceLink<D3Node, D3Link>)?.strength(forces.link))
        .force('collision', d3.forceCollide<D3Node>().strength(forces.collision))
        .force('center', d3.forceCenter(this.config.width / 2, this.config.height / 2).strength(forces.center));
      
      this.simulation.alpha(0.3).restart();
    }
  }

  /**
   * Stop the simulation
   */
  stop(): void {
    if (this.simulation) {
      this.simulation.stop();
    }
  }
}

/**
 * Default layout configuration
 */
export const defaultLayoutConfig: LayoutConfig = {
  width: 800,
  height: 600,
  nodeSize: 'uniform',
  clustering: true,
  animation: true,
  forces: {
    charge: -300,
    link: 0.5,
    collision: 0.8,
    center: 0.1,
  },
};

/**
 * Performance-optimized configuration for large graphs
 */
export const performanceLayoutConfig: LayoutConfig = {
  width: 800,
  height: 600,
  nodeSize: 'uniform',
  clustering: false,
  animation: false,
  forces: {
    charge: -150,
    link: 0.3,
    collision: 0.5,
    center: 0.05,
  },
}; 