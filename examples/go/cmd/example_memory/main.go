package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"alejandria-go-examples/pkg/client"
)

func main() {
	// Load environment variables
	serverPath := os.Getenv("ALEJANDRIA_BIN")
	if serverPath == "" {
		serverPath = "./target/release/alejandria"
	}
	dbPath := os.Getenv("ALEJANDRIA_DB")

	// Create context with cancellation support
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Set up signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
	go func() {
		<-sigChan
		fmt.Println("\nReceived interrupt signal, shutting down...")
		cancel()
	}()

	// Initialize Alejandria client
	fmt.Println("Initializing Alejandria MCP client...")
	c, err := client.NewAlejandriaClient(ctx, serverPath, dbPath)
	if err != nil {
		log.Fatalf("Failed to initialize client: %v", err)
	}
	defer func() {
		fmt.Println("\nClosing client...")
		if err := c.Close(); err != nil {
			log.Printf("Error closing client: %v", err)
		}
	}()

	fmt.Println("✓ Client initialized successfully\n")

	// Example 1: Store memories
	fmt.Println("=== Storing Memories ===")

	memory1ID, err := c.MemStore(ctx, client.MemStoreParams{
		Content:    "Learned about Go's context package for managing request lifecycles",
		Summary:    "Go context package",
		Importance: "high",
		Topic:      "golang",
		TopicKey:   "go/context-learning",
		Source:     "go-example",
	})
	if err != nil {
		log.Fatalf("Failed to store memory 1: %v", err)
	}
	fmt.Printf("✓ Stored memory 1 with ID: %s\n", memory1ID)

	memory2ID, err := c.MemStore(ctx, client.MemStoreParams{
		Content:    "Go's defer statement ensures cleanup code runs even on panic",
		Summary:    "Go defer for resource cleanup",
		Importance: "medium",
		Topic:      "golang",
		Source:     "go-example",
	})
	if err != nil {
		log.Fatalf("Failed to store memory 2: %v", err)
	}
	fmt.Printf("✓ Stored memory 2 with ID: %s\n", memory2ID)

	memory3ID, err := c.MemStore(ctx, client.MemStoreParams{
		Content:    "Goroutines are lightweight threads managed by Go runtime",
		Summary:    "Go concurrency with goroutines",
		Importance: "high",
		Topic:      "golang",
		Source:     "go-example",
	})
	if err != nil {
		log.Fatalf("Failed to store memory 3: %v", err)
	}
	fmt.Printf("✓ Stored memory 3 with ID: %s\n\n", memory3ID)

	// Example 2: Recall memories
	fmt.Println("=== Recalling Memories ===")

	// Note: mem_recall has a known FTS5 bug that may cause errors
	// This is documented and will be fixed in the server
	memories, err := c.MemRecall(ctx, client.MemRecallParams{
		Query: "goroutines context",
		Limit: 5,
	})
	if err != nil {
		// Gracefully handle the known FTS5 error
		fmt.Printf("⚠ Recall failed (known FTS5 bug): %v\n", err)
		fmt.Println("  This is a server-side issue and does not affect storage operations\n")
	} else {
		fmt.Printf("Found %d memories:\n", len(memories))
		for i, memory := range memories {
			fmt.Printf("  %d. [%s] %s (similarity: %.2f)\n",
				i+1, memory.ID, memory.Content[:50]+"...", memory.Similarity)
		}
		fmt.Println()
	}

	// Example 3: List topics
	fmt.Println("=== Listing Topics ===")

	topics, err := c.MemListTopics(ctx)
	if err != nil {
		log.Fatalf("Failed to list topics: %v", err)
	}
	fmt.Printf("Found %d topics:\n", len(topics))
	for _, topic := range topics {
		fmt.Printf("  - %s: %d memories\n", topic.Name, topic.Count)
	}
	fmt.Println()

	fmt.Println("✓ Memory operations completed successfully!")
}
