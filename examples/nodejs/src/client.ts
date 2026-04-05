import { spawn, ChildProcess } from 'child_process';
import { config } from 'dotenv';
import * as readline from 'readline';

// Load environment variables
config();

/**
 * JSON-RPC 2.0 Request structure
 */
interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number | string;
  method: string;
  params?: Record<string, unknown>;
}

/**
 * JSON-RPC 2.0 Response structure
 */
interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id: number | string;
  result?: T;
  error?: JsonRpcError;
}

/**
 * JSON-RPC 2.0 Error structure
 */
interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

/**
 * MCP Tool Response structure
 */
interface McpToolResponse {
  content: Array<{
    type: 'text' | 'image' | 'resource';
    text?: string;
    data?: string;
    mimeType?: string;
  }>;
  isError?: boolean;
}

/**
 * Parameters for mem_store tool
 */
export interface MemStoreParams {
  content: string;
  summary?: string;
  importance?: 'critical' | 'high' | 'medium' | 'low';
  topic?: string;
  topic_key?: string;
  source?: string;
  related_ids?: string[];
}

/**
 * Parameters for mem_recall tool
 */
export interface MemRecallParams {
  query: string;
  limit?: number;
  min_score?: number;
  topic?: string;
}

/**
 * Memory store response
 */
interface MemStoreResponse {
  id: string;
  action: 'created' | 'updated';
}

/**
 * Memoir object returned by memoir_create
 */
interface Memoir {
  id: string;
  name: string;
  description: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

/**
 * Concept object returned by memoir_add_concept
 */
interface Concept {
  id: string;
  memoir_id: string;
  name: string;
  definition: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

/**
 * Memory structure returned by mem_recall
 */
interface Memory {
  id: string;
  content: string;
  topic?: string;
  importance?: string;
  created_at?: string;
}

/**
 * Topic structure with count
 */
interface Topic {
  topic: string;
  count: number;
}

/**
 * Topic object returned by mem_list_topics
 */
export interface Topic {
  name: string;
  count: number;
}

/**
 * Parameters for memoir_create tool
 */
export interface MemoirCreateParams {
  name: string;
  description?: string;
}

/**
 * Parameters for memoir_add_concept tool
 */
export interface MemoirAddConceptParams {
  memoir: string;  // Memoir name (not ID)
  name: string;    // Concept name
  definition?: string;  // Concept definition
  labels?: string[];    // Optional labels
}

/**
 * Parameters for memoir_link tool
 */
export interface MemoirLinkParams {
  memoir: string;   // Memoir name (not ID)
  source: string;   // Source concept name
  target: string;   // Target concept name
  relation: string; // Relationship type (IsA, HasProperty, RelatedTo, etc.)
  weight?: number;  // Optional weight (default: 1.0)
}

/**
 * Custom error class for MCP tool errors
 */
export class MCPToolError extends Error {
  constructor(
    public toolName: string,
    public code: number,
    message: string,
    public data?: unknown
  ) {
    super(`MCP tool '${toolName}' failed (code ${code}): ${message}`);
    this.name = 'MCPToolError';
  }
}

/**
 * Custom error class for MCP connection errors
 */
export class MCPConnectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'MCPConnectionError';
  }
}

/**
 * Alejandria MCP Client
 *
 * TypeScript client for communicating with the Alejandria MCP server via stdio transport.
 * Implements JSON-RPC 2.0 protocol for tool invocation.
 */
export class AlejandriaClient {
  private serverProcess?: ChildProcess;
  private nextId = 1;
  private pendingRequests = new Map<number, {
    resolve: (result: unknown) => void;
    reject: (error: Error) => void;
  }>();
  private closed = false;

  constructor(
    private serverPath?: string,
    private dbPath?: string
  ) {
    this.serverPath = serverPath || process.env.ALEJANDRIA_BIN;
    this.dbPath = dbPath || process.env.ALEJANDRIA_DB;

    if (!this.serverPath) {
      throw new Error(
        'Alejandria server path not configured. Set ALEJANDRIA_BIN environment variable or pass serverPath parameter.'
      );
    }
  }

  /**
   * Connect to the Alejandria MCP server by spawning the subprocess
   */
  async connect(): Promise<void> {
    if (this.serverProcess) {
      throw new MCPConnectionError('Client already connected');
    }

    const args = ['serve'];
    const env = { ...process.env };
    
    if (this.dbPath) {
      env.ALEJANDRIA_DB = this.dbPath;
    }

    try {
      this.serverProcess = spawn(this.serverPath!, args, {
        stdio: ['pipe', 'pipe', 'pipe'],
        env,
      });
    } catch (error) {
      throw new MCPConnectionError(
        `Failed to spawn server at ${this.serverPath}: ${error}`
      );
    }

    // Set up error handlers
    this.serverProcess.on('error', (error) => {
      this.handleServerError(error);
    });

    this.serverProcess.on('exit', (code, signal) => {
      if (!this.closed) {
        this.handleServerExit(code, signal);
      }
    });

    // Set up stdout reader for responses
    if (this.serverProcess.stdout) {
      const rl = readline.createInterface({
        input: this.serverProcess.stdout,
        crlfDelay: Infinity,
      });

      rl.on('line', (line) => {
        this.handleResponse(line);
      });
    }

    // Give server a moment to initialize
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Check if process is still running
    if (this.serverProcess.exitCode !== null) {
      throw new MCPConnectionError(
        `Server exited immediately with code ${this.serverProcess.exitCode}`
      );
    }
  }

  /**
   * Send a JSON-RPC request to the server
   */
  private async sendRequest<T>(method: string, params: Record<string, unknown>): Promise<T> {
    if (!this.serverProcess || !this.serverProcess.stdin) {
      throw new MCPConnectionError('Not connected to server');
    }

    const id = this.nextId++;
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      // Store pending request
      this.pendingRequests.set(id, { resolve: resolve as (result: unknown) => void, reject });

      // Set timeout
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new MCPToolError(method, -1, 'Request timeout after 30 seconds'));
      }, 30000);

      // Send request
      const requestStr = JSON.stringify(request) + '\n';
      this.serverProcess!.stdin!.write(requestStr, (error) => {
        if (error) {
          clearTimeout(timeout);
          this.pendingRequests.delete(id);
          reject(new MCPConnectionError(`Failed to write request: ${error}`));
        }
      });
    });
  }

  /**
   * Handle JSON-RPC response from server
   */
  private handleResponse(line: string): void {
    try {
      const response: JsonRpcResponse = JSON.parse(line);
      const pending = this.pendingRequests.get(response.id as number);

      if (!pending) {
        console.warn(`Received response for unknown request ID: ${response.id}`);
        return;
      }

      this.pendingRequests.delete(response.id as number);

      if (response.error) {
        pending.reject(
          new MCPToolError(
            'unknown',
            response.error.code,
            response.error.message,
            response.error.data
          )
        );
      } else {
        pending.resolve(response.result);
      }
    } catch (error) {
      console.error('Failed to parse response:', error);
    }
  }

  /**
   * Handle server process errors
   */
  private handleServerError(error: Error): void {
    console.error('Server process error:', error);
    
    // Reject all pending requests
    for (const [id, pending] of this.pendingRequests.entries()) {
      pending.reject(new MCPConnectionError(`Server error: ${error.message}`));
      this.pendingRequests.delete(id);
    }
  }

  /**
   * Handle server process exit
   */
  private handleServerExit(code: number | null, signal: string | null): void {
    const message = signal
      ? `Server terminated by signal ${signal}`
      : `Server exited with code ${code}`;

    // Reject all pending requests
    for (const [id, pending] of this.pendingRequests.entries()) {
      pending.reject(new MCPConnectionError(message));
      this.pendingRequests.delete(id);
    }
  }

  /**
   * Call an MCP tool and extract result from formatted response
   */
  private async callTool<T>(name: string, args: Record<string, unknown>): Promise<T> {
    const response = await this.sendRequest<any>('tools/call', { name, arguments: args });
    
    // Handle two response formats from the server:
    // 1. Direct objects (memoir tools): { id: "...", name: "...", ... }
    // 2. MCP-wrapped (memory tools): { content: [{ type: "text", text: "..." }] }
    
    // Check if it's an MCP-wrapped response
    if (response && typeof response === 'object' && 'content' in response) {
      const mcpResponse = response as McpToolResponse;
      
      // Check for MCP-level errors
      if (mcpResponse.isError) {
        const errorText = mcpResponse.content[0]?.text || 'Unknown error';
        throw new MCPToolError(name, -1, errorText);
      }
      
      // Extract the text content
      if (!mcpResponse.content || mcpResponse.content.length === 0) {
        throw new MCPToolError(name, -1, 'No content in MCP response');
      }
      
      const firstContent = mcpResponse.content[0];
      if (firstContent.type !== 'text') {
        throw new MCPToolError(name, -1, `Unexpected content type: ${firstContent.type}`);
      }
      
      const text = firstContent.text;
      if (!text) {
        throw new MCPToolError(name, -1, 'Empty response text');
      }
      
      // Server returns formatted text with JSON embedded:
      // "Memory stored:\n{...json...}"
      // "Found N topics:\n[...json...]"
      
      try {
        // Try to parse the entire text as JSON first
        return JSON.parse(text) as T;
      } catch {
        // If that fails, try to extract JSON from formatted text
        const jsonMatch = text.match(/(\{[\s\S]*\}|\[[\s\S]*\])/);
        if (jsonMatch) {
          try {
            return JSON.parse(jsonMatch[1]) as T;
          } catch (e) {
            throw new MCPToolError(name, -1, `Failed to parse JSON response: ${e}`);
          }
        }
        
        // If no JSON found, return the text as-is (for simple string responses)
        return text as T;
      }
    }
    
    // Direct object response (memoir tools)
    return response as T;
  }

  /**
   * Store a memory using mem_store tool
   */
  async memStore(params: MemStoreParams): Promise<MemStoreResponse> {
    return this.callTool<MemStoreResponse>('mem_store', params);
  }

  /**
   * Recall memories using mem_recall tool
   */
  async memRecall(params: MemRecallParams): Promise<Memory[]> {
    const result = await this.callTool<Memory[]>('mem_recall', params);
    return result;
  }

  /**
   * List all topics using mem_list_topics tool
   */
  async memListTopics(): Promise<Topic[]> {
    const result = await this.callTool<Topic[]>('mem_list_topics', {});
    return result;
  }

  /**
   * Create a new memoir using memoir_create tool
   */
  async memoirCreate(params: MemoirCreateParams): Promise<Memoir> {
    return this.callTool<Memoir>('memoir_create', params);
  }

  /**
   * Add a concept to a memoir using memoir_add_concept tool
   */
  async memoirAddConcept(params: MemoirAddConceptParams): Promise<Concept> {
    return this.callTool<Concept>('memoir_add_concept', params);
  }

  /**
   * Link two concepts in a memoir using memoir_link tool
   */
  async memoirLink(params: MemoirLinkParams): Promise<void> {
    await this.callTool<void>('memoir_link', params);
  }

  /**
   * Close the connection and terminate the server process
   */
  async close(): Promise<void> {
    if (this.closed || !this.serverProcess) {
      return;
    }

    this.closed = true;

    // Reject all pending requests
    for (const [id, pending] of this.pendingRequests.entries()) {
      pending.reject(new MCPConnectionError('Client closed'));
      this.pendingRequests.delete(id);
    }

    // Send SIGTERM
    this.serverProcess.kill('SIGTERM');

    // Wait for graceful shutdown (max 5 seconds)
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => {
        if (this.serverProcess && this.serverProcess.exitCode === null) {
          this.serverProcess.kill('SIGKILL');
        }
        resolve();
      }, 5000);

      this.serverProcess!.once('exit', () => {
        clearTimeout(timeout);
        resolve();
      });
    });
  }
}

// Handle process termination signals
process.on('SIGINT', async () => {
  console.log('\nReceived SIGINT, shutting down gracefully...');
  process.exit(0);
});

process.on('SIGTERM', async () => {
  console.log('\nReceived SIGTERM, shutting down gracefully...');
  process.exit(0);
});
