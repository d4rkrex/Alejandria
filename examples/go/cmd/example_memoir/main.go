package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"sync"
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

	// Example 1: Create memoir knowledge graph
	fmt.Println("=== Creating Memoir ===")

	memoirID, err := c.MemoirCreate(ctx, client.MemoirCreateParams{
		Name:        "Programming Paradigms",
		Description: "Knowledge graph of different programming paradigms and their relationships",
	})
	if err != nil {
		log.Fatalf("Failed to create memoir: %v", err)
	}
	fmt.Printf("✓ Created memoir with ID: %s\n\n", memoirID)

	// Example 2: Add concepts concurrently using goroutines
	fmt.Println("=== Adding Concepts (concurrent) ===")

	concepts := []struct {
		name       string
		definition string
	}{
		{"Functional Programming", "Programming paradigm that treats computation as evaluation of mathematical functions"},
		{"Object-Oriented Programming", "Programming paradigm based on the concept of objects containing data and code"},
		{"Procedural Programming", "Programming paradigm based on procedure calls and sequential execution"},
		{"Declarative Programming", "Programming paradigm that expresses logic without describing control flow"},
		{"Imperative Programming", "Programming paradigm using statements that change program state"},
	}

	// Use WaitGroup to synchronize goroutines
	var wg sync.WaitGroup
	conceptIDs := make([]string, len(concepts))
	errors := make([]error, len(concepts))

	for i, concept := range concepts {
		wg.Add(1)
		go func(idx int, conc struct {
			name       string
			definition string
		}) {
			defer wg.Done()

			conceptID, err := c.MemoirAddConcept(ctx, client.MemoirAddConceptParams{
				Memoir:     memoirID,
				Concept:    conc.name,
				Definition: conc.definition,
			})
			if err != nil {
				errors[idx] = err
				return
			}
			conceptIDs[idx] = conceptID
			fmt.Printf("✓ Added concept: %s (ID: %s)\n", conc.name, conceptID)
		}(i, concept)
	}

	// Wait for all goroutines to complete
	wg.Wait()

	// Check for errors
	for i, err := range errors {
		if err != nil {
			log.Fatalf("Failed to add concept %s: %v", concepts[i].name, err)
		}
	}
	fmt.Println()

	// Example 3: Link concepts sequentially
	fmt.Println("=== Linking Concepts ===")

	// Functional Programming is_a Declarative Programming
	err = c.MemoirLink(ctx, client.MemoirLinkParams{
		Memoir:       memoirID,
		FromConcept:  "Functional Programming",
		ToConcept:    "Declarative Programming",
		Relationship: "is_a",
	})
	if err != nil {
		log.Fatalf("Failed to link concepts: %v", err)
	}
	fmt.Println("✓ Linked: Functional Programming -> is_a -> Declarative Programming")

	// Object-Oriented Programming is_a Imperative Programming
	err = c.MemoirLink(ctx, client.MemoirLinkParams{
		Memoir:       memoirID,
		FromConcept:  "Object-Oriented Programming",
		ToConcept:    "Imperative Programming",
		Relationship: "is_a",
	})
	if err != nil {
		log.Fatalf("Failed to link concepts: %v", err)
	}
	fmt.Println("✓ Linked: Object-Oriented Programming -> is_a -> Imperative Programming")

	// Procedural Programming is_a Imperative Programming
	err = c.MemoirLink(ctx, client.MemoirLinkParams{
		Memoir:       memoirID,
		FromConcept:  "Procedural Programming",
		ToConcept:    "Imperative Programming",
		Relationship: "is_a",
	})
	if err != nil {
		log.Fatalf("Failed to link concepts: %v", err)
	}
	fmt.Println("✓ Linked: Procedural Programming -> is_a -> Imperative Programming")

	// Functional Programming related_to Object-Oriented Programming
	err = c.MemoirLink(ctx, client.MemoirLinkParams{
		Memoir:       memoirID,
		FromConcept:  "Functional Programming",
		ToConcept:    "Object-Oriented Programming",
		Relationship: "related_to",
	})
	if err != nil {
		log.Fatalf("Failed to link concepts: %v", err)
	}
	fmt.Println("✓ Linked: Functional Programming -> related_to -> Object-Oriented Programming")

	fmt.Println()
	fmt.Println("✓ Memoir operations completed successfully!")
	fmt.Printf("  Created memoir '%s' with %d concepts and 4 relationships\n", memoirID, len(concepts))
}
