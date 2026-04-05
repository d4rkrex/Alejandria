package client

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"sync"
	"time"
)

// JSON-RPC 2.0 Request structure
type JsonRpcRequest struct {
	JsonRpc string      `json:"jsonrpc"`
	ID      int         `json:"id"`
	Method  string      `json:"method"`
	Params  interface{} `json:"params,omitempty"`
}

// JSON-RPC 2.0 Response structure
type JsonRpcResponse struct {
	JsonRpc string          `json:"jsonrpc"`
	ID      int             `json:"id"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *RpcError       `json:"error,omitempty"`
}

// JSON-RPC Error structure
type RpcError struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    interface{} `json:"data,omitempty"`
}

// Implement error interface for RpcError
func (e *RpcError) Error() string {
	return fmt.Sprintf("RPC error %d: %s", e.Code, e.Message)
}

// MCP Tool Call Parameters
type ToolCallParams struct {
	Name      string                 `json:"name"`
	Arguments map[string]interface{} `json:"arguments"`
}

// MCP Tool Response structure
type McpToolResponse struct {
	Content []struct {
		Type     string `json:"type"`
		Text     string `json:"text,omitempty"`
		Data     string `json:"data,omitempty"`
		MimeType string `json:"mimeType,omitempty"`
	} `json:"content"`
	IsError bool `json:"isError,omitempty"`
}

// Memory Store Parameters
type MemStoreParams struct {
	Content    string   `json:"content"`
	Summary    string   `json:"summary,omitempty"`
	Importance string   `json:"importance,omitempty"`
	Topic      string   `json:"topic,omitempty"`
	TopicKey   string   `json:"topic_key,omitempty"`
	Source     string   `json:"source,omitempty"`
	RelatedIds []string `json:"related_ids,omitempty"`
}

// Memory Recall Parameters
type MemRecallParams struct {
	Query    string  `json:"query"`
	Limit    int     `json:"limit,omitempty"`
	MinScore float64 `json:"min_score,omitempty"`
	Topic    string  `json:"topic,omitempty"`
}

// Memory structure
type Memory struct {
	ID         string                 `json:"id"`
	Title      string                 `json:"title"`
	Content    string                 `json:"content"`
	Summary    string                 `json:"summary"`
	Importance string                 `json:"importance"`
	Topic      string                 `json:"topic"`
	TopicKey   string                 `json:"topic_key"`
	Similarity float64                `json:"similarity,omitempty"`
	Metadata   map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt  string                 `json:"created_at"`
	UpdatedAt  string                 `json:"updated_at"`
}

// Topic structure
type Topic struct {
	Name  string `json:"name"`
	Count int    `json:"count"`
}

// Memoir Create Parameters
type MemoirCreateParams struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
}

// Memoir Add Concept Parameters
type MemoirAddConceptParams struct {
	Memoir     string `json:"memoir"`
	Concept    string `json:"concept"`
	Definition string `json:"definition,omitempty"`
}

// Memoir Link Parameters
type MemoirLinkParams struct {
	Memoir       string `json:"memoir"`
	FromConcept  string `json:"from_concept"`
	ToConcept    string `json:"to_concept"`
	Relationship string `json:"relationship"`
}

// Memoir structure
type Memoir struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Metadata    map[string]interface{} `json:"metadata"`
	CreatedAt   string                 `json:"created_at"`
	UpdatedAt   string                 `json:"updated_at"`
}

// Concept structure
type Concept struct {
	ID         string `json:"id"`
	MemoirID   string `json:"memoir_id"`
	Name       string `json:"name"`
	Definition string `json:"definition"`
	CreatedAt  string `json:"created_at"`
	UpdatedAt  string `json:"updated_at"`
}

// AlejandriaClient manages communication with the Alejandria MCP server
type AlejandriaClient struct {
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	stdout *bufio.Scanner
	ctx    context.Context
	nextID int
	mu     sync.Mutex // Protects nextID for thread-safe request ID generation
}

// NewAlejandriaClient creates a new client and spawns the MCP server
func NewAlejandriaClient(ctx context.Context, serverPath, dbPath string) (*AlejandriaClient, error) {
	// Verify server binary exists
	if _, err := os.Stat(serverPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("MCP server binary not found at: %s\nPlease set ALEJANDRIA_BIN environment variable or build the server:\n  cargo build --release --bin alejandria", serverPath)
	}

	// Create command with context
	cmd := exec.CommandContext(ctx, serverPath, "serve")

	// Set database path if provided
	if dbPath != "" {
		cmd.Env = append(os.Environ(), fmt.Sprintf("ALEJANDRIA_DB=%s", dbPath))
	}

	// Create pipes for stdin/stdout
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return nil, fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		stdin.Close()
		return nil, fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	// Start the server process
	if err := cmd.Start(); err != nil {
		stdin.Close()
		return nil, fmt.Errorf("failed to start MCP server: %w", err)
	}

	client := &AlejandriaClient{
		cmd:    cmd,
		stdin:  stdin,
		stdout: bufio.NewScanner(stdoutPipe),
		ctx:    ctx,
		nextID: 1,
	}

	// Wait a moment for server to initialize
	time.Sleep(100 * time.Millisecond)

	return client, nil
}

// sendRequest sends a JSON-RPC request and returns the result
func (c *AlejandriaClient) sendRequest(method string, params interface{}) (json.RawMessage, error) {
	c.mu.Lock()
	requestID := c.nextID
	c.nextID++
	c.mu.Unlock()

	// Construct JSON-RPC request
	request := JsonRpcRequest{
		JsonRpc: "2.0",
		ID:      requestID,
		Method:  method,
		Params:  params,
	}

	// Serialize request to JSON
	requestData, err := json.Marshal(request)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	// Send request to server (with newline delimiter)
	if _, err := c.stdin.Write(append(requestData, '\n')); err != nil {
		return nil, fmt.Errorf("failed to write request: %w", err)
	}

	// Read response from server
	if !c.stdout.Scan() {
		if err := c.stdout.Err(); err != nil {
			return nil, fmt.Errorf("failed to read response: %w", err)
		}
		return nil, fmt.Errorf("server closed connection")
	}

	// Parse JSON-RPC response
	var response JsonRpcResponse
	if err := json.Unmarshal(c.stdout.Bytes(), &response); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	// Check for JSON-RPC error
	if response.Error != nil {
		return nil, response.Error
	}

	return response.Result, nil
}

// callTool invokes an MCP tool and handles dual-format responses
func (c *AlejandriaClient) callTool(name string, arguments map[string]interface{}) (json.RawMessage, error) {
	params := ToolCallParams{
		Name:      name,
		Arguments: arguments,
	}

	result, err := c.sendRequest("tools/call", params)
	if err != nil {
		return nil, fmt.Errorf("tool %s failed: %w", name, err)
	}

	// Try to parse as MCP-wrapped response (Memory tools format)
	var mcpResponse McpToolResponse
	if err := json.Unmarshal(result, &mcpResponse); err == nil && len(mcpResponse.Content) > 0 {
		// Extract text content from MCP wrapper
		if mcpResponse.IsError {
			return nil, fmt.Errorf("tool returned error: %s", mcpResponse.Content[0].Text)
		}
		if mcpResponse.Content[0].Type == "text" && mcpResponse.Content[0].Text != "" {
			// Parse the text content as JSON (for Memory tools)
			return json.RawMessage(mcpResponse.Content[0].Text), nil
		}
	}

	// Otherwise, return direct JSON (Memoir tools format)
	return result, nil
}

// MemStore stores a memory in Alejandria
func (c *AlejandriaClient) MemStore(ctx context.Context, params MemStoreParams) (string, error) {
	// Convert params struct to map for JSON serialization
	args := map[string]interface{}{
		"content": params.Content,
	}
	if params.Summary != "" {
		args["summary"] = params.Summary
	}
	if params.Importance != "" {
		args["importance"] = params.Importance
	}
	if params.Topic != "" {
		args["topic"] = params.Topic
	}
	if params.TopicKey != "" {
		args["topic_key"] = params.TopicKey
	}
	if params.Source != "" {
		args["source"] = params.Source
	}
	if len(params.RelatedIds) > 0 {
		args["related_ids"] = params.RelatedIds
	}

	result, err := c.callTool("mem_store", args)
	if err != nil {
		return "", err
	}

	// Parse response to extract memory ID
	var response struct {
		ID     string `json:"id"`
		Action string `json:"action"`
	}
	if err := json.Unmarshal(result, &response); err != nil {
		return "", fmt.Errorf("failed to parse mem_store response: %w", err)
	}

	return response.ID, nil
}

// MemRecall recalls memories matching the query
func (c *AlejandriaClient) MemRecall(ctx context.Context, params MemRecallParams) ([]Memory, error) {
	args := map[string]interface{}{
		"query": params.Query,
	}
	if params.Limit > 0 {
		args["limit"] = params.Limit
	}
	if params.MinScore > 0 {
		args["min_score"] = params.MinScore
	}
	if params.Topic != "" {
		args["topic"] = params.Topic
	}

	result, err := c.callTool("mem_recall", args)
	if err != nil {
		return nil, err
	}

	var memories []Memory
	if err := json.Unmarshal(result, &memories); err != nil {
		return nil, fmt.Errorf("failed to parse mem_recall response: %w", err)
	}

	return memories, nil
}

// MemListTopics lists all memory topics with counts
func (c *AlejandriaClient) MemListTopics(ctx context.Context) ([]Topic, error) {
	result, err := c.callTool("mem_list_topics", map[string]interface{}{})
	if err != nil {
		return nil, err
	}

	var topics []Topic
	if err := json.Unmarshal(result, &topics); err != nil {
		return nil, fmt.Errorf("failed to parse mem_list_topics response: %w", err)
	}

	return topics, nil
}

// MemoirCreate creates a new memoir knowledge graph
func (c *AlejandriaClient) MemoirCreate(ctx context.Context, params MemoirCreateParams) (string, error) {
	args := map[string]interface{}{
		"name": params.Name,
	}
	if params.Description != "" {
		args["description"] = params.Description
	}

	result, err := c.callTool("memoir_create", args)
	if err != nil {
		return "", err
	}

	// Memoir tools return direct JSON objects
	var memoir Memoir
	if err := json.Unmarshal(result, &memoir); err != nil {
		return "", fmt.Errorf("failed to parse memoir_create response: %w", err)
	}

	return memoir.ID, nil
}

// MemoirAddConcept adds a concept to a memoir
func (c *AlejandriaClient) MemoirAddConcept(ctx context.Context, params MemoirAddConceptParams) (string, error) {
	args := map[string]interface{}{
		"memoir":  params.Memoir,
		"concept": params.Concept,
	}
	if params.Definition != "" {
		args["definition"] = params.Definition
	}

	result, err := c.callTool("memoir_add_concept", args)
	if err != nil {
		return "", err
	}

	var concept Concept
	if err := json.Unmarshal(result, &concept); err != nil {
		return "", fmt.Errorf("failed to parse memoir_add_concept response: %w", err)
	}

	return concept.ID, nil
}

// MemoirLink creates a relationship between two concepts
func (c *AlejandriaClient) MemoirLink(ctx context.Context, params MemoirLinkParams) error {
	args := map[string]interface{}{
		"memoir":       params.Memoir,
		"from_concept": params.FromConcept,
		"to_concept":   params.ToConcept,
		"relationship": params.Relationship,
	}

	_, err := c.callTool("memoir_link", args)
	return err
}

// Close terminates the MCP server process and cleans up resources
func (c *AlejandriaClient) Close() error {
	// Close stdin to signal server to shut down
	if c.stdin != nil {
		c.stdin.Close()
	}

	// Wait for process to exit (with timeout)
	done := make(chan error, 1)
	go func() {
		done <- c.cmd.Wait()
	}()

	select {
	case <-time.After(5 * time.Second):
		// Force kill if graceful shutdown takes too long
		if c.cmd.Process != nil {
			c.cmd.Process.Kill()
		}
		return fmt.Errorf("server did not shut down gracefully, forced kill")
	case err := <-done:
		return err
	}
}
