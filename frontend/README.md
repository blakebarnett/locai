# Locai Graph Visualization Frontend

This is the frontend implementation for **Task 17: Graph Visualization** of the Locai memory system. It provides an interactive web-based visualization of the graph memory structure with real-time updates.

## Features Implemented

### ✅ Core Visualization
- **React Flow Integration**: Interactive graph visualization with zoom, pan, and selection
- **Custom Node Types**: Specialized components for Memory and Entity nodes
- **Real-time Updates**: WebSocket integration for live graph changes
- **Responsive Design**: Modern UI with Tailwind CSS

### ✅ Graph Components
- **Memory Nodes**: Visual representation of different memory types (Fact, Episodic, Semantic)
- **Entity Nodes**: Customizable entity visualization with type-based icons
- **Interactive Edges**: Relationship visualization with labels and selection
- **Graph Controls**: Search, filtering, and layout options
- **Analytics Panel**: Real-time metrics and centrality analysis

### ✅ Technical Stack
- **React 18** with TypeScript
- **React Flow** for graph visualization
- **Zustand** for state management
- **TanStack Query** for API state management
- **Tailwind CSS** for styling
- **WebSocket** for real-time updates

### ✅ API Integration
- **Comprehensive REST Client**: Full TypeScript support for all Locai API endpoints
- **WebSocket Manager**: Robust connection handling with reconnection logic
- **Graph Operations**: Memory/Entity graph loading, path finding, pattern queries
- **Metrics Integration**: Real-time graph analytics and centrality scores

## Project Structure

```
frontend/
├── src/
│   ├── components/
│   │   ├── nodes/
│   │   │   ├── MemoryNode.tsx      # Custom memory node component
│   │   │   └── EntityNode.tsx      # Custom entity node component
│   │   ├── GraphVisualization.tsx  # Main graph component
│   │   ├── GraphControls.tsx       # Control panel
│   │   ├── AnalyticsPanel.tsx      # Metrics display
│   │   └── DemoLoader.tsx          # Sample data loader
│   ├── stores/
│   │   └── graphStore.ts           # Zustand state management
│   ├── services/
│   │   ├── api.ts                  # API client
│   │   └── websocket.ts            # WebSocket manager
│   ├── types/
│   │   └── api.ts                  # TypeScript type definitions
│   ├── App.tsx                     # Main application
│   └── index.css                   # Tailwind styles
├── package.json
└── README.md
```

## Getting Started

### Prerequisites
- Node.js 18+ 
- npm or yarn
- Locai server running on `http://localhost:3000`

### Installation

1. **Install dependencies:**
   ```bash
   npm install
   ```

2. **Start the development server:**
   ```bash
   npm run dev
   ```

3. **Open your browser:**
   Navigate to `http://localhost:5173`

### Configuration

The application expects the Locai server to be running on `http://localhost:3000`. To change this:

1. Create a `.env` file in the frontend directory:
   ```
   VITE_API_BASE_URL=http://your-server:port
   ```

## Usage

### Demo Mode
The application includes sample data for testing the visualization without a running backend. The demo loads automatically and shows:
- 3 sample memories (different types)
- 3 sample entities (Person, Technology, Concept)
- 4 relationships connecting them

### Graph Interaction
- **Click nodes/edges** to select them
- **Drag nodes** to reposition
- **Zoom and pan** to navigate large graphs
- **Use controls** to filter and search
- **Expand nodes** to load connected subgraphs

### Real-time Features
- **Live updates** when memories/entities are created/updated/deleted
- **Connection status** indicator in top-left
- **Automatic metrics refresh** every 30 seconds

## API Integration

The frontend integrates with all Locai API endpoints:

### Graph Endpoints
- `GET /api/memories/{id}/graph` - Load memory subgraph
- `