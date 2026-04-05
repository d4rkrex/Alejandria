"""
Alejandria MCP Client for Python

This module provides a Python client for communicating with the Alejandria MCP server
using JSON-RPC 2.0 over stdio transport.

Usage:
    from client import AlejandriaClient

    with AlejandriaClient() as client:
        memory_id = client.mem_store(content="Hello world")
        memories = client.mem_recall(query="hello", limit=5)
"""

import json
import os
import signal
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()


class MCPToolError(Exception):
    """Exception raised when an MCP tool invocation fails."""

    def __init__(
        self, code: int, message: str, tool_name: str, data: Optional[Any] = None
    ):
        self.code = code
        self.message = message
        self.tool_name = tool_name
        self.data = data
        super().__init__(f"MCP tool '{tool_name}' failed (code {code}): {message}")


class MCPConnectionError(Exception):
    """Exception raised when connection to MCP server is lost."""

    pass


class AlejandriaClient:
    """
    Client for communicating with Alejandria MCP server.

    The client spawns the MCP server as a subprocess and communicates via stdin/stdout
    using line-delimited JSON-RPC 2.0 messages.

    Attributes:
        server_path: Path to the Alejandria server binary
        db_path: Path to the SQLite database

    Example:
        with AlejandriaClient() as client:
            # Store a memory
            memory_id = client.mem_store(
                content="Learned about MCP protocol",
                topic="learning",
                importance="high"
            )

            # Recall similar memories
            results = client.mem_recall(query="MCP", limit=5)
            for memory in results:
                print(f"- {memory['title']}")
    """

    def __init__(
        self, server_path: Optional[str] = None, db_path: Optional[str] = None
    ):
        """
        Initialize the Alejandria client.

        Args:
            server_path: Path to Alejandria binary. If None, reads from ALEJANDRIA_BIN env var.
            db_path: Path to database. If None, reads from ALEJANDRIA_DB env var or uses default.

        Raises:
            FileNotFoundError: If server binary not found at the specified path.
        """
        self.server_path = server_path or os.getenv(
            "ALEJANDRIA_BIN", "./target/release/alejandria"
        )
        self.db_path = db_path or os.getenv("ALEJANDRIA_DB")

        # Check if server binary exists
        if not Path(self.server_path).exists():
            raise FileNotFoundError(
                f"MCP server binary not found at: {self.server_path}\n"
                f"Please set ALEJANDRIA_BIN environment variable or build the server:\n"
                f"  cargo build --release --bin alejandria"
            )

        self.process: Optional[subprocess.Popen] = None
        self._request_id = 0
        self._setup_signal_handlers()

    def _setup_signal_handlers(self):
        """Register signal handlers for graceful shutdown."""

        def signal_handler(signum, frame):
            self.close()
            sys.exit(0)

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

    def _spawn_server(self):
        """
        Spawn the MCP server subprocess and establish stdio communication.

        Raises:
            MCPConnectionError: If server process fails to start.
        """
        # Build command args
        cmd = [self.server_path, "serve"]

        # Set environment variables for the subprocess
        env = os.environ.copy()
        if self.db_path:
            env["ALEJANDRIA_DB"] = self.db_path

        try:
            self.process = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env=env,
                text=True,
                bufsize=1,  # Line buffered
            )
        except Exception as e:
            raise MCPConnectionError(f"Failed to spawn MCP server: {e}") from e

        # Give server a moment to start
        import time

        time.sleep(0.5)

        # Check if process is still running
        if self.process.poll() is not None:
            stderr = self.process.stderr.read() if self.process.stderr else ""
            raise MCPConnectionError(
                f"MCP server exited unexpectedly with code {self.process.returncode}\n"
                f"stderr: {stderr}"
            )

    def _send_request(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """
        Send a JSON-RPC request to the MCP server.

        Args:
            method: The JSON-RPC method name (e.g., "tools/call")
            params: Method parameters

        Returns:
            The 'result' field from the JSON-RPC response

        Raises:
            MCPToolError: If the server returns an error
            MCPConnectionError: If communication with server fails
        """
        if not self.process or self.process.poll() is not None:
            raise MCPConnectionError("MCP server process not running")

        # Construct JSON-RPC 2.0 request
        self._request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self._request_id,
            "method": method,
            "params": params,
        }

        try:
            # Send request (line-delimited JSON)
            request_json = json.dumps(request) + "\n"
            self.process.stdin.write(request_json)
            self.process.stdin.flush()

            # Read response (line-delimited JSON)
            response_line = self.process.stdout.readline()
            if not response_line:
                raise MCPConnectionError("Server closed stdout (EOF)")

            response = json.loads(response_line)

            # Validate JSON-RPC 2.0 response
            if response.get("jsonrpc") != "2.0":
                raise MCPConnectionError(f"Invalid JSON-RPC response: {response}")

            # Check for errors
            if "error" in response:
                error = response["error"]
                tool_name = (
                    params.get("name", "unknown") if method == "tools/call" else method
                )
                raise MCPToolError(
                    code=error.get("code", -1),
                    message=error.get("message", "Unknown error"),
                    tool_name=tool_name,
                    data=error.get("data"),
                )

            # Return result
            return response.get("result", {})

        except json.JSONDecodeError as e:
            raise MCPConnectionError(f"Failed to parse server response: {e}") from e
        except BrokenPipeError as e:
            raise MCPConnectionError("Server connection broken (pipe closed)") from e
        except Exception as e:
            if isinstance(e, (MCPToolError, MCPConnectionError)):
                raise
            raise MCPConnectionError(f"Unexpected error during request: {e}") from e

    def _call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """
        Call an MCP tool by name with the given arguments.

        Args:
            tool_name: Name of the MCP tool (e.g., "mem_store")
            arguments: Tool-specific arguments

        Returns:
            The tool's result (structure depends on the tool)
        """
        result = self._send_request(
            "tools/call", {"name": tool_name, "arguments": arguments}
        )
        return result

    # Memory Operations

    def mem_store(
        self,
        content: str,
        summary: Optional[str] = None,
        importance: Optional[str] = None,
        topic: Optional[str] = None,
        topic_key: Optional[str] = None,
        source: Optional[str] = None,
        related_ids: Optional[List[str]] = None,
    ) -> str:
        """
        Store a memory in Alejandria.

        Args:
            content: Memory content (required)
            summary: Brief summary of the memory
            importance: Importance level ("critical", "high", "medium", "low")
            topic: Topic for organization
            topic_key: Unique key for upsert workflow (updates existing memory if key matches)
            source: Source of the memory
            related_ids: List of related memory IDs

        Returns:
            The memory ID as a string

        Example:
            memory_id = client.mem_store(
                content="Learned how to use MCP protocol",
                topic="learning/mcp",
                importance="high"
            )
        """
        args = {"content": content}
        if summary:
            args["summary"] = summary
        if importance:
            args["importance"] = importance
        if topic:
            args["topic"] = topic
        if topic_key:
            args["topic_key"] = topic_key
        if source:
            args["source"] = source
        if related_ids:
            args["related_ids"] = related_ids

        result = self._call_tool("mem_store", args)
        return result.get("id", "")

    def mem_recall(
        self,
        query: str,
        limit: int = 10,
        min_score: Optional[float] = None,
        topic: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """
        Search and recall memories using hybrid search (BM25 + vector similarity).

        Args:
            query: Search query (required)
            limit: Maximum number of results (default: 10)
            min_score: Minimum similarity score (0.0-1.0)
            topic: Filter by topic

        Returns:
            List of memory dictionaries with keys: id, title, content, similarity, etc.

        Example:
            memories = client.mem_recall(query="MCP protocol", limit=5, min_score=0.7)
            for memory in memories:
                print(f"[{memory['id']}] {memory['title']} (score: {memory['similarity']})")
        """
        args = {"query": query, "limit": limit}
        if min_score is not None:
            args["min_score"] = min_score
        if topic:
            args["topic"] = topic

        result = self._call_tool("mem_recall", args)
        return result.get("memories", [])

    def mem_list_topics(self) -> List[Dict[str, Any]]:
        """
        List all topics with memory counts.

        Returns:
            List of topic dictionaries with keys: topic, count

        Example:
            topics = client.mem_list_topics()
            for topic in topics:
                print(f"{topic['topic']}: {topic['count']} memories")
        """
        result = self._call_tool("mem_list_topics", {})
        return result.get("topics", [])

    # Memoir Operations

    def memoir_create(self, name: str, description: Optional[str] = None) -> str:
        """
        Create a new memoir (knowledge graph).

        Args:
            name: Name of the memoir (required)
            description: Description of the memoir

        Returns:
            The memoir ID as a string

        Example:
            memoir_id = client.memoir_create(
                name="Machine Learning Concepts",
                description="A knowledge graph of ML terminology"
            )
        """
        args = {"name": name}
        if description:
            args["description"] = description

        result = self._call_tool("memoir_create", args)
        return result.get("id", "")

    def memoir_add_concept(
        self, memoir_id: str, concept: str, description: Optional[str] = None
    ) -> str:
        """
        Add a concept to a memoir.

        Args:
            memoir_id: The memoir ID
            concept: Concept name (required)
            description: Concept description

        Returns:
            The concept ID as a string

        Example:
            concept_id = client.memoir_add_concept(
                memoir_id=memoir_id,
                concept="Neural Networks",
                description="Computational models inspired by biological neural networks"
            )
        """
        args = {"memoir_id": memoir_id, "concept": concept}
        if description:
            args["description"] = description

        result = self._call_tool("memoir_add_concept", args)
        return result.get("id", "")

    def memoir_link(
        self, memoir_id: str, from_concept: str, to_concept: str, relationship: str
    ):
        """
        Create a relationship link between two concepts in a memoir.

        Args:
            memoir_id: The memoir ID
            from_concept: Source concept ID
            to_concept: Target concept ID
            relationship: Relationship type (e.g., "includes", "enables", "relates_to")

        Example:
            client.memoir_link(
                memoir_id=memoir_id,
                from_concept=ml_concept_id,
                to_concept=nn_concept_id,
                relationship="includes"
            )
        """
        args = {
            "memoir_id": memoir_id,
            "from_concept": from_concept,
            "to_concept": to_concept,
            "relationship": relationship,
        }

        self._call_tool("memoir_link", args)

    def close(self):
        """
        Gracefully terminate the MCP server process.

        Sends SIGTERM and waits up to 5 seconds for graceful shutdown.
        If the process doesn't terminate, sends SIGKILL.
        """
        if self.process and self.process.poll() is None:
            try:
                # Send SIGTERM for graceful shutdown
                self.process.terminate()

                # Wait up to 5 seconds
                try:
                    self.process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    # Force kill if still running
                    self.process.kill()
                    self.process.wait()
            except Exception as e:
                print(f"Warning: Error during server shutdown: {e}", file=sys.stderr)
            finally:
                self.process = None

    # Context manager support

    def __enter__(self):
        """Context manager entry: spawn server and return client."""
        self._spawn_server()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit: ensure server is terminated."""
        self.close()
        return False  # Don't suppress exceptions
