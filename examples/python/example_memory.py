#!/usr/bin/env python3
"""
Example demonstrating Alejandria memory operations via MCP.

This script shows how to:
1. Store memories with different topics and tags
2. Recall memories using hybrid search (semantic + keyword)
3. List all available topics
4. Filter recalls by topic and tags

Prerequisites:
- Alejandria server binary built (cargo build --release)
- Python dependencies installed (pip install -r requirements.txt)
- .env file configured with ALEJANDRIA_BIN and ALEJANDRIA_DB paths
"""

import sys
import os
from pathlib import Path

# Add parent directory to path for client import
sys.path.insert(0, str(Path(__file__).parent))

from client import AlejandriaClient, MCPToolError, MCPConnectionError


def print_section(title: str):
    """Print a formatted section header."""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}\n")


def main():
    """Run memory operations demo."""

    print_section("Alejandria Memory Operations Demo")

    try:
        # Initialize client (spawns server subprocess)
        with AlejandriaClient() as client:
            print("✓ Connected to Alejandria MCP server\n")

            # 1. Store memories with different topics
            print_section("1. Storing Memories")

            memories = [
                {
                    "content": "The Rust borrow checker prevents data races at compile time by enforcing ownership rules.",
                    "topic": "rust-concepts",
                    "importance": "high",
                    "description": "Store Rust memory safety concept",
                },
                {
                    "content": "MCP (Model Context Protocol) uses JSON-RPC 2.0 over stdio for client-server communication.",
                    "topic": "mcp-protocol",
                    "importance": "high",
                    "description": "Store MCP protocol detail",
                },
                {
                    "content": "Semantic search combines vector embeddings with keyword matching for better retrieval.",
                    "topic": "search-techniques",
                    "importance": "medium",
                    "description": "Store search technique",
                },
                {
                    "content": "The actor model in Rust uses message passing between isolated actors for concurrency.",
                    "topic": "rust-concepts",
                    "importance": "medium",
                    "description": "Store Rust concurrency pattern",
                },
            ]

            memory_ids = []
            for mem in memories:
                print(f"Storing: {mem['description']}")
                print(f"  Topic: {mem['topic']}")
                print(f"  Importance: {mem['importance']}")

                result = client.mem_store(
                    content=mem["content"],
                    topic=mem["topic"],
                    importance=mem["importance"],
                )

                memory_id = result
                memory_ids.append(memory_id)
                print(f"  ✓ Stored with ID: {memory_id}\n")

            # 2. List all topics
            print_section("2. Listing All Topics")

            topics_result = client.mem_list_topics()
            topics = (
                topics_result.get("topics", [])
                if isinstance(topics_result, dict)
                else []
            )

            print(f"Found {len(topics)} topics:\n")
            for topic_info in topics:
                if isinstance(topic_info, dict):
                    topic_name = topic_info.get("topic", "unknown")
                    count = topic_info.get("count", 0)
                    print(f"  • {topic_name}: {count} memories")
                else:
                    print(f"  • {topic_info}")

            # 3. Recall memories - semantic search
            print_section("3. Semantic Search - General Query")

            query = "How does Rust handle memory safety?"
            print(f'Query: "{query}"\n')

            recall_result = client.mem_recall(query=query, limit=2)

            memories_found = recall_result
            print(f"Found {len(memories_found)} relevant memories:\n")

            for i, mem in enumerate(memories_found, 1):
                print(f"{i}. Score: {mem.get('score', 0.0):.3f}")
                print(f"   Topic: {mem.get('topic', 'none')}")
                print(f"   Content: {mem.get('content', '')[:100]}...\n")

            # 4. Recall with topic filter
            print_section("4. Filtered Search - By Topic")

            query = "concurrency"
            topic = "rust-concepts"
            print(f'Query: "{query}"')
            print(f"Topic filter: {topic}\n")

            recall_result = client.mem_recall(query=query, topic=topic, limit=5)

            memories_found = recall_result
            print(f"Found {len(memories_found)} memories in '{topic}' topic:\n")

            for i, mem in enumerate(memories_found, 1):
                print(f"{i}. {mem.get('content', '')[:80]}...\n")

            # 5. Keyword search
            print_section("5. Keyword Search - Exact Matching")

            query = "JSON-RPC"
            print(f'Query: "{query}"\n')

            recall_result = client.mem_recall(query=query, limit=3)

            memories_found = recall_result
            print(f"Found {len(memories_found)} memories matching keyword:\n")

            for i, mem in enumerate(memories_found, 1):
                print(f"{i}. {mem.get('content', '')}\n")

            print_section("Demo Complete")
            print("✓ All memory operations executed successfully")
            print("✓ Server will shut down gracefully\n")

    except MCPConnectionError as e:
        print(f"\n❌ Connection Error: {e}", file=sys.stderr)
        print("\nTroubleshooting:")
        print("1. Ensure Alejandria is built: cargo build --release")
        print("2. Check .env file has correct ALEJANDRIA_BIN path")
        print("3. Verify ALEJANDRIA_DB path is writable")
        sys.exit(1)

    except MCPToolError as e:
        print(f"\n❌ Tool Error: {e}", file=sys.stderr)
        sys.exit(1)

    except KeyboardInterrupt:
        print("\n\n⚠ Interrupted by user")
        sys.exit(130)

    except Exception as e:
        print(f"\n❌ Unexpected Error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
