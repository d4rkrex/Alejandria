#!/usr/bin/env python3
"""
Example demonstrating Alejandria memoir (knowledge graph) operations via MCP.

This script shows how to:
1. Create a new memoir (knowledge graph)
2. Add concepts to the memoir
3. Link concepts with typed relationships
4. Build a simple knowledge graph structure

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
    """Run memoir operations demo."""

    print_section("Alejandria Memoir (Knowledge Graph) Demo")

    try:
        # Initialize client (spawns server subprocess)
        with AlejandriaClient() as client:
            print("✓ Connected to Alejandria MCP server\n")

            # 1. Create a memoir
            print_section("1. Creating Memoir")

            memoir_name = "rust-concurrency-patterns"
            memoir_desc = (
                "Knowledge graph of Rust concurrency patterns and their relationships"
            )

            print(f"Creating memoir: {memoir_name}")
            print(f"Description: {memoir_desc}\n")

            memoir_id = client.memoir_create(name=memoir_name, description=memoir_desc)

            print(f"✓ Created memoir with ID: {memoir_id}\n")

            # 2. Add concepts to the memoir
            print_section("2. Adding Concepts")

            concepts = [
                {
                    "name": "Ownership",
                    "concept_type": "core-principle",
                    "description": "Rust's ownership system ensures memory safety without garbage collection",
                },
                {
                    "name": "Borrowing",
                    "concept_type": "core-principle",
                    "description": "References that allow temporary access to data without taking ownership",
                },
                {
                    "name": "Channels",
                    "concept_type": "concurrency-primitive",
                    "description": "Message-passing primitive for communication between threads",
                },
                {
                    "name": "Arc",
                    "concept_type": "smart-pointer",
                    "description": "Atomic reference counting for shared ownership across threads",
                },
                {
                    "name": "Mutex",
                    "concept_type": "synchronization-primitive",
                    "description": "Mutual exclusion lock for protecting shared data",
                },
                {
                    "name": "Actor Model",
                    "concept_type": "design-pattern",
                    "description": "Concurrency pattern using isolated actors communicating via messages",
                },
                {
                    "name": "Async/Await",
                    "concept_type": "language-feature",
                    "description": "Asynchronous programming primitives for non-blocking I/O",
                },
            ]

            concept_ids = {}
            for concept in concepts:
                print(f"Adding concept: {concept['name']}")
                print(f"  Type: {concept['concept_type']}")
                print(f"  Description: {concept['description'][:60]}...\n")

                concept_id = client.memoir_add_concept(
                    memoir_id=memoir_id,
                    name=concept["name"],
                    concept_type=concept["concept_type"],
                    description=concept["description"],
                )

                concept_ids[concept["name"]] = concept_id
                print(f"  ✓ Added with ID: {concept_id}\n")

            # 3. Link concepts with typed relationships
            print_section("3. Creating Relationships")

            relationships = [
                {
                    "from": "Borrowing",
                    "to": "Ownership",
                    "rel_type": "builds-on",
                    "description": "Explain borrowing relationship",
                },
                {
                    "from": "Mutex",
                    "to": "Ownership",
                    "rel_type": "enforces",
                    "description": "Explain how Mutex enforces ownership",
                },
                {
                    "from": "Arc",
                    "to": "Ownership",
                    "rel_type": "enables",
                    "description": "Explain Arc and shared ownership",
                },
                {
                    "from": "Channels",
                    "to": "Ownership",
                    "rel_type": "leverages",
                    "description": "Explain ownership transfer in channels",
                },
                {
                    "from": "Actor Model",
                    "to": "Channels",
                    "rel_type": "uses",
                    "description": "Explain actor-channel relationship",
                },
                {
                    "from": "Actor Model",
                    "to": "Ownership",
                    "rel_type": "relies-on",
                    "description": "Explain actor isolation via ownership",
                },
                {
                    "from": "Async/Await",
                    "to": "Ownership",
                    "rel_type": "compatible-with",
                    "description": "Explain async and ownership interaction",
                },
                {
                    "from": "Mutex",
                    "to": "Arc",
                    "rel_type": "commonly-paired-with",
                    "description": "Explain Arc<Mutex<T>> pattern",
                },
            ]

            for rel in relationships:
                from_id = concept_ids.get(rel["from"])
                to_id = concept_ids.get(rel["to"])

                if not from_id or not to_id:
                    print(
                        f"⚠ Skipping relationship: {rel['from']} -> {rel['to']} (concept not found)\n"
                    )
                    continue

                print(f"Linking: {rel['from']} --[{rel['rel_type']}]--> {rel['to']}")
                print(f"  Context: {rel['description']}\n")

                client.memoir_link(
                    memoir_id=memoir_id,
                    from_concept_id=from_id,
                    to_concept_id=to_id,
                    relationship_type=rel["rel_type"],
                )

                print(f"  ✓ Created link successfully\n")

            # 4. Summary
            print_section("Knowledge Graph Summary")

            print(f"Memoir: {memoir_name}")
            print(f"ID: {memoir_id}\n")
            print(f"Concepts added: {len(concepts)}")
            print(f"Relationships created: {len(relationships)}\n")

            print("Graph structure:")
            print("  • Core principles: Ownership, Borrowing")
            print("  • Concurrency primitives: Channels, Mutex, Arc")
            print("  • Design patterns: Actor Model")
            print("  • Language features: Async/Await\n")

            print("Key relationships:")
            print("  • Borrowing builds on Ownership")
            print("  • Mutex and Arc enforce/enable Ownership")
            print("  • Channels leverage Ownership for safe message passing")
            print("  • Actor Model uses Channels and relies on Ownership")
            print("  • Arc<Mutex<T>> is a common pattern for shared state\n")

            print_section("Demo Complete")
            print("✓ Knowledge graph created successfully")
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
