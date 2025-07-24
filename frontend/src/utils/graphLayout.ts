import { forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide } from 'd3-force';
import type { GraphNode, GraphEdge } from '../stores/graphStore';

export interface LayoutNode {
  id: string;
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

export interface LayoutLink {
  source: string | LayoutNode;
  target: string | LayoutNode;
}

// Calculate network density to adjust layout parameters
const calculateNetworkDensity = (nodeCount: number, edgeCount: number): number => {
  if (nodeCount <= 1) return 0;
  const maxPossibleEdges = (nodeCount * (nodeCount - 1)) / 2;
  return edgeCount / maxPossibleEdges;
};

// Get optimal layout parameters based on network size and density
const getLayoutParameters = (nodeCount: number, edgeCount: number, width: number, height: number) => {
  const density = calculateNetworkDensity(nodeCount, edgeCount);
  
  console.log(`🔧 Layout parameter calculation:`, {
    nodeCount,
    edgeCount,
    density: density.toFixed(3),
    dimensions: `${width}x${height}`
  });
  
  // Cap nodeSpacing for reasonable values with small networks
  const area = width * height;
  const rawNodeSpacing = Math.sqrt(area / nodeCount);
  const nodeSpacing = Math.min(rawNodeSpacing, 150); // Cap at 150px for sanity
  
  // Handle very small networks (< 10 nodes) with fixed reasonable parameters
  if (nodeCount < 10) {
    console.log(`🔧 Small network detected, using fixed parameters`);
    return {
      linkDistance: 80,
      linkStrength: 0.6,
      chargeStrength: -200,
      collisionRadius: 35,
      centerStrength: 0.3,
      alphaDecay: 0.05, // Faster convergence for small networks
      velocityDecay: 0.5, // Higher damping for quicker settling
      maxIterations: 150 // Fewer iterations needed
    };
  }
  
  // For very dense networks (high density OR many edges)
  if (density > 0.2 || edgeCount > 500) {
    console.log(`🔧 Dense network detected, using spread-out parameters`);
    return {
      linkDistance: Math.max(80, Math.min(nodeSpacing * 1.5, 200)), // Cap max distance
      linkStrength: 0.3, // Weaker links to spread nodes out more
      chargeStrength: Math.min(-800, -nodeCount * 8), // Strong repulsion
      collisionRadius: Math.max(35, Math.min(nodeSpacing * 0.3, 80)), // Cap max collision
      centerStrength: 0.3,
      alphaDecay: 0.01, // Slower cooling for better convergence
      velocityDecay: 0.4,
      maxIterations: 500
    };
  }
  
  // For medium density networks
  if (density > 0.1 || edgeCount > 200) {
    console.log(`🔧 Medium density network, using balanced parameters`);
    return {
      linkDistance: Math.max(60, Math.min(nodeSpacing * 1.2, 150)),
      linkStrength: 0.4,
      chargeStrength: Math.min(-500, -nodeCount * 6),
      collisionRadius: Math.max(30, Math.min(nodeSpacing * 0.25, 60)),
      centerStrength: 0.4,
      alphaDecay: 0.015,
      velocityDecay: 0.35,
      maxIterations: 400
    };
  }
  
  // For sparse networks (original parameters)
  console.log(`🔧 Sparse network, using compact parameters`);
  return {
    linkDistance: 80,
    linkStrength: 0.5,
    chargeStrength: -300,
    collisionRadius: 30,
    centerStrength: 0.5,
    alphaDecay: 0.02,
    velocityDecay: 0.3,
    maxIterations: 300
  };
};

export const calculateForceLayout = (
  nodes: GraphNode[],
  edges: GraphEdge[],
  width: number = 1200, // Increased default width for dense graphs
  height: number = 800   // Increased default height for dense graphs
): Promise<GraphNode[]> => {
  return new Promise((resolve) => {
    const params = getLayoutParameters(nodes.length, edges.length, width, height);
    
    console.log(`Layout parameters for ${nodes.length} nodes, ${edges.length} edges:`, params);
    
    // Convert to D3 format
    const d3Nodes: LayoutNode[] = nodes.map(node => ({
      id: node.id,
      x: node.position.x || Math.random() * width,
      y: node.position.y || Math.random() * height,
    }));

    const d3Links: LayoutLink[] = edges.map(edge => ({
      source: edge.source,
      target: edge.target,
    }));

    // Create force simulation with density-aware parameters
    const simulation = forceSimulation(d3Nodes)
      .force('link', forceLink(d3Links)
        .id((d: any) => d.id)
        .distance(params.linkDistance)
        .strength(params.linkStrength))
      .force('charge', forceManyBody()
        .strength(params.chargeStrength)
        .distanceMax(Math.min(width, height) * 0.8)) // Limit repulsion range
      .force('center', forceCenter(width * 0.7, height / 2) // Account for left panel
        .strength(params.centerStrength))
      .force('collision', forceCollide()
        .radius(params.collisionRadius)
        .strength(0.8))
      .alphaDecay(params.alphaDecay)
      .velocityDecay(params.velocityDecay);

    // Track iterations to prevent infinite loops
    let iterations = 0;
    const maxIterations = params.maxIterations;

    simulation.on('tick', () => {
      iterations++;
      if (iterations >= maxIterations) {
        simulation.stop();
      }
      
      // Log progress based on network size
      const logFrequency = nodes.length < 10 ? 10 : 50; // More frequent for small networks
      if (iterations % logFrequency === 0) {
        console.log(`🔧 Layout iteration ${iterations}/${maxIterations}, alpha: ${simulation.alpha().toFixed(3)}`);
      }
    });

    simulation.on('end', () => {
      const finalAlpha = simulation.alpha();
      const converged = finalAlpha <= simulation.alphaMin();
      console.log(`🔧 Layout ${converged ? 'converged naturally' : 'stopped by timeout'} after ${iterations} iterations (alpha: ${finalAlpha.toFixed(4)})`);
      
      // Apply calculated positions back to React Flow nodes
      const updatedNodes = nodes.map(node => {
        const d3Node = d3Nodes.find(d => d.id === node.id);
        return {
          ...node,
          position: {
            x: Math.max(0, Math.min(width - 100, d3Node?.x || node.position.x)),
            y: Math.max(0, Math.min(height - 100, d3Node?.y || node.position.y)),
          },
        };
      });

      resolve(updatedNodes);
    });

    // Adjust timeout based on network complexity
    let timeout: number;
    if (nodes.length < 10) {
      timeout = 1500; // Small networks converge quickly but give a bit more time
    } else if (edges.length > 500) {
      timeout = 5000; // Dense networks need more time
    } else {
      timeout = 2000; // Default for medium networks
    }
    
    console.log(`🔧 Setting ${timeout}ms timeout for ${nodes.length} nodes, ${edges.length} edges`);
    
    setTimeout(() => {
      if (simulation.alpha() > simulation.alphaMin()) {
        console.log(`Force layout timeout after ${timeout}ms, stopping simulation`);
        simulation.stop();
      }
    }, timeout);
  });
};

export const calculateHierarchicalLayout = (
  nodes: GraphNode[],
  edges: GraphEdge[],
  width: number = 1200,
  height: number = 800
): GraphNode[] => {
  // Enhanced hierarchical layout with better spacing for dense networks
  const memoryNodes = nodes.filter(n => n.data.type === 'memory');
  const entityNodes = nodes.filter(n => n.data.type === 'entity');

  const updatedNodes: GraphNode[] = [];
  
  // Calculate optimal grid size based on available space (account for left panel)
  const visibleWidth = width * 0.6; // Only use right 60% of screen
  const visibleStartX = width * 0.4; // Start after left panel
  const memoryArea = height * 0.6; // 60% of height for memories
  const entityArea = height * 0.3; // 30% of height for entities
  
  // Place memory nodes in a grid in the upper portion
  const memoryCols = Math.ceil(Math.sqrt(memoryNodes.length * (visibleWidth / memoryArea)));
  const memorySpacingX = visibleWidth / (memoryCols + 1);
  const memorySpacingY = memoryArea / (Math.ceil(memoryNodes.length / memoryCols) + 1);

  memoryNodes.forEach((node, index) => {
    const row = Math.floor(index / memoryCols);
    const col = index % memoryCols;
    
    updatedNodes.push({
      ...node,
      position: {
        x: visibleStartX + memorySpacingX * (col + 1),
        y: memorySpacingY * (row + 1),
      },
    });
  });

  // Place entity nodes in a grid in the lower portion
  const entityCols = Math.ceil(Math.sqrt(entityNodes.length * (visibleWidth / entityArea)));
  const entitySpacingX = visibleWidth / (entityCols + 1);
  const entitySpacingY = entityArea / (Math.ceil(entityNodes.length / entityCols) + 1);

  entityNodes.forEach((node, index) => {
    const row = Math.floor(index / entityCols);
    const col = index % entityCols;
    
    updatedNodes.push({
      ...node,
      position: {
        x: visibleStartX + entitySpacingX * (col + 1),
        y: height * 0.65 + entitySpacingY * (row + 1),
      },
    });
  });

  return updatedNodes;
};

export const calculateCircularLayout = (
  nodes: GraphNode[],
  edges: GraphEdge[],
  width: number = 1200,
  height: number = 800
): GraphNode[] => {
  // Enhanced circular layout with clustering for dense networks
  const memoryNodes = nodes.filter(n => n.data.type === 'memory');
  const entityNodes = nodes.filter(n => n.data.type === 'entity');
  
  // Center in visible area (account for left panel)
  const centerX = width * 0.7; // Center of visible area
  const centerY = height / 2;
  const maxRadius = Math.min(width * 0.6, height) * 0.4; // Use visible width
  
  const updatedNodes: GraphNode[] = [];
  
  // Place memory nodes in outer circle
  const memoryRadius = maxRadius;
  memoryNodes.forEach((node, index) => {
    const angle = (2 * Math.PI * index) / memoryNodes.length;
    updatedNodes.push({
      ...node,
      position: {
        x: centerX + memoryRadius * Math.cos(angle),
        y: centerY + memoryRadius * Math.sin(angle),
      },
    });
  });
  
  // Place entity nodes in inner circle
  const entityRadius = maxRadius * 0.4;
  entityNodes.forEach((node, index) => {
    const angle = (2 * Math.PI * index) / entityNodes.length;
    updatedNodes.push({
      ...node,
      position: {
        x: centerX + entityRadius * Math.cos(angle),
        y: centerY + entityRadius * Math.sin(angle),
      },
    });
  });

  return updatedNodes;
}; 