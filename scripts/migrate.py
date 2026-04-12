#!/usr/bin/env python3
"""
Alejandría Migration Tool
Migrates memories from Engram to Alejandría and syncs between local/remote instances
"""

import sqlite3
import json
import argparse
import sys
from datetime import datetime
from pathlib import Path
import subprocess
import hashlib


class Color:
    BLUE = "\033[0;34m"
    GREEN = "\033[0;32m"
    YELLOW = "\033[1;33m"
    RED = "\033[0;31m"
    NC = "\033[0m"


def print_banner():
    print(f"{Color.BLUE}")
    print("""
    _    _           _                _      _       
   / \  | | ___     | | __ _ _ __   __| |_ __(_) __ _ 
  / _ \ | |/ _ \ _  | |/ _` | '_ \ / _` | '__| |/ _` |
 / ___ \| |  __/ |_|| | (_| | | | | (_| | |  | | (_| |
/_/   \_\_|\___|\___/ \__,_|_| |_|\__,_|_|  |_|\__,_|
                                                      
          Migration & Sync Tool
    """)
    print(f"{Color.NC}")


def get_engram_db():
    """Get path to Engram database"""
    engram_db = Path.home() / ".engram" / "engram.db"
    if not engram_db.exists():
        print(f"{Color.RED}Error: Engram database not found at {engram_db}{Color.NC}")
        sys.exit(1)
    return engram_db


def get_alejandria_db():
    """Get path to Alejandría database"""
    alejandria_db = Path.home() / ".local" / "share" / "alejandria" / "alejandria.db"
    if not alejandria_db.exists():
        print(
            f"{Color.RED}Error: Alejandría database not found at {alejandria_db}{Color.NC}"
        )
        sys.exit(1)
    return alejandria_db


def migrate_engram_to_alejandria(
    dry_run=False, project_filter=None, skip_duplicates=True
):
    """Migrate observations from Engram to Alejandría"""

    print(f"{Color.BLUE}[1/4] Connecting to databases...{Color.NC}")
    engram_db = get_engram_db()
    alejandria_db = get_alejandria_db()

    engram_conn = sqlite3.connect(engram_db)
    engram_conn.row_factory = sqlite3.Row

    alejandria_conn = sqlite3.connect(alejandria_db)
    alejandria_conn.row_factory = sqlite3.Row

    print(f"{Color.GREEN}✓ Connected{Color.NC}")

    # Query Engram observations
    print(f"{Color.BLUE}[2/4] Fetching Engram observations...{Color.NC}")
    query = """
        SELECT id, session_id, type, title, content, project, scope, 
               topic_key, created_at, updated_at
        FROM observations
        WHERE deleted_at IS NULL
    """

    params = []
    if project_filter:
        query += " AND project = ?"
        params.append(project_filter)

    engram_cursor = engram_conn.execute(query, params)
    observations = engram_cursor.fetchall()

    print(f"{Color.GREEN}✓ Found {len(observations)} observations{Color.NC}")

    # Prepare migration
    print(f"{Color.BLUE}[3/4] Analyzing observations...{Color.NC}")

    # Get existing Alejandría topic_keys (to detect duplicates)
    existing_topic_keys = set()
    if skip_duplicates:
        alejandria_cursor = alejandria_conn.execute(
            "SELECT topic_key FROM memories WHERE topic_key IS NOT NULL AND deleted_at IS NULL"
        )
        existing_topic_keys = {row["topic_key"] for row in alejandria_cursor.fetchall()}
        print(
            f"{Color.YELLOW}  Found {len(existing_topic_keys)} existing topic_keys in Alejandría{Color.NC}"
        )

    # Map importance
    importance_map = {
        "manual": "medium",
        "decision": "high",
        "architecture": "high",
        "bugfix": "high",
        "pattern": "medium",
        "config": "medium",
        "discovery": "medium",
        "learning": "low",
        "tool_use": "low",
        "file_change": "medium",
        "command": "low",
        "file_read": "low",
        "search": "low",
    }

    # Migrate
    print(f"{Color.BLUE}[4/4] Migrating observations...{Color.NC}")
    migrated = 0
    skipped = 0
    errors = 0

    for obs in observations:
        try:
            # Check if already exists
            if (
                skip_duplicates
                and obs["topic_key"]
                and obs["topic_key"] in existing_topic_keys
            ):
                skipped += 1
                continue

            # Map fields
            content = obs["content"] or ""
            summary = obs["title"] or content[:200]
            topic = obs["project"] or obs["scope"] or "general"
            importance = importance_map.get(obs["type"], "medium")

            # Extract keywords from title and content
            keywords_set = set()
            if obs["title"]:
                keywords_set.update(obs["title"].lower().split())
            if obs["content"]:
                # Take first 500 chars for keywords
                keywords_set.update(obs["content"][:500].lower().split())
            keywords = " ".join(sorted(keywords_set)[:50])  # Max 50 keywords

            # Timestamps
            created_at = obs["created_at"]
            if created_at:
                try:
                    dt = datetime.fromisoformat(created_at.replace("Z", "+00:00"))
                    created_timestamp = int(dt.timestamp())
                except:
                    created_timestamp = int(datetime.now().timestamp())
            else:
                created_timestamp = int(datetime.now().timestamp())

            # Generate ID (ULID-like)
            id_hash = (
                hashlib.sha256(f"{obs['id']}-{created_timestamp}".encode())
                .hexdigest()[:26]
                .upper()
            )
            memory_id = f"01{id_hash}"

            if dry_run:
                print(f"  [DRY RUN] Would migrate: {summary[:60]}... (topic: {topic})")
                migrated += 1
            else:
                # Insert into Alejandría
                alejandria_conn.execute(
                    """
                    INSERT INTO memories (
                        id, created_at, updated_at, last_accessed, access_count,
                        weight, topic, summary, raw_excerpt, keywords, importance,
                        source, topic_key, revision_count, last_seen_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                    (
                        memory_id,
                        created_timestamp,
                        created_timestamp,
                        created_timestamp,
                        0,  # access_count
                        1.0,  # weight
                        topic,
                        summary,
                        content,
                        keywords,
                        importance,
                        "engram_migration",
                        obs["topic_key"],
                        1,  # revision_count
                        created_timestamp,
                    ),
                )

                migrated += 1

                if migrated % 100 == 0:
                    print(
                        f"  {Color.YELLOW}Progress: {migrated}/{len(observations)}{Color.NC}"
                    )

        except Exception as e:
            errors += 1
            print(
                f"  {Color.RED}Error migrating observation {obs['id']}: {e}{Color.NC}"
            )

    if not dry_run:
        alejandria_conn.commit()

    engram_conn.close()
    alejandria_conn.close()

    # Summary
    print(f"\n{Color.GREEN}{'═' * 60}{Color.NC}")
    print(f"{Color.GREEN}Migration {'Preview' if dry_run else 'Complete'}!{Color.NC}")
    print(f"{Color.GREEN}{'═' * 60}{Color.NC}\n")
    print(f"  {Color.BLUE}Total observations:{Color.NC} {len(observations)}")
    print(f"  {Color.GREEN}Migrated:{Color.NC} {migrated}")
    print(f"  {Color.YELLOW}Skipped (duplicates):{Color.NC} {skipped}")
    print(f"  {Color.RED}Errors:{Color.NC} {errors}")

    if dry_run:
        print(
            f"\n{Color.YELLOW}This was a dry run. Use --execute to actually migrate.{Color.NC}"
        )


def export_alejandria(output_file, project_filter=None):
    """Export Alejandría memories to JSON"""

    print(f"{Color.BLUE}[1/2] Exporting from Alejandría...{Color.NC}")
    alejandria_db = get_alejandria_db()

    conn = sqlite3.connect(alejandria_db)
    conn.row_factory = sqlite3.Row

    query = "SELECT * FROM memories WHERE deleted_at IS NULL"
    params = []

    if project_filter:
        query += " AND topic = ?"
        params.append(project_filter)

    cursor = conn.execute(query, params)
    memories = cursor.fetchall()

    # Convert to dict
    export_data = {
        "exported_at": datetime.now().isoformat(),
        "source": "alejandria",
        "count": len(memories),
        "memories": [],
    }

    for mem in memories:
        # Convert Unix timestamps (milliseconds) to RFC 3339
        created_at = (
            datetime.fromtimestamp(mem["created_at"] / 1000).isoformat() + "Z"
            if mem["created_at"]
            else None
        )
        updated_at = (
            datetime.fromtimestamp(mem["updated_at"] / 1000).isoformat() + "Z"
            if mem["updated_at"]
            else None
        )

        export_data["memories"].append(
            {
                "id": mem["id"],
                "created_at": created_at,
                "updated_at": updated_at,
                "topic": mem["topic"],
                "summary": mem["summary"],
                "content": mem["raw_excerpt"],
                "importance": mem["importance"],
                "topic_key": mem["topic_key"],
                "keywords": mem["keywords"],
                "weight": mem["weight"],
                "access_count": mem["access_count"],
            }
        )

    conn.close()

    print(f"{Color.BLUE}[2/2] Writing to {output_file}...{Color.NC}")
    with open(output_file, "w") as f:
        json.dump(export_data, f, indent=2)

    print(
        f"{Color.GREEN}✓ Exported {len(memories)} memories to {output_file}{Color.NC}"
    )


def import_to_remote(json_file, remote_url, api_key, dry_run=False):
    """Import JSON memories to remote Alejandría via HTTP API"""

    print(f"{Color.BLUE}[1/3] Loading JSON file...{Color.NC}")
    with open(json_file, "r") as f:
        data = json.load(f)

    memories = data.get("memories", [])
    print(f"{Color.GREEN}✓ Loaded {len(memories)} memories{Color.NC}")

    print(f"{Color.BLUE}[2/3] Connecting to remote server...{Color.NC}")

    # Test connection
    import requests

    try:
        response = requests.get(
            f"{remote_url}/health", headers={"X-API-Key": api_key}, timeout=5
        )
        if response.status_code == 200:
            print(f"{Color.GREEN}✓ Connected to {remote_url}{Color.NC}")
        else:
            print(f"{Color.RED}Error: Server returned {response.status_code}{Color.NC}")
            return
    except Exception as e:
        print(f"{Color.RED}Error connecting to server: {e}{Color.NC}")
        return

    print(f"{Color.BLUE}[3/3] Importing memories...{Color.NC}")

    # Note: This requires a batch import endpoint on the server
    # For now, we'll use the CLI approach via SSH

    print(f"{Color.YELLOW}⚠ Remote import via HTTP not yet implemented.{Color.NC}")
    print(f"{Color.YELLOW}Alternative: Use CLI import on the server:{Color.NC}")
    print(f"\n  1. Copy file to server:")
    print(f"     scp {json_file} user@server:/tmp/import.json")
    print(f"  2. SSH to server and run:")
    print(f"     alejandria import --input /tmp/import.json")


def sync_local_to_remote(remote_url, api_key, project_filter=None):
    """Sync local Alejandría to remote (via export + server import)"""

    import tempfile

    print(f"{Color.BLUE}Syncing local → remote...{Color.NC}\n")

    # Export local
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as tmp:
        tmp_file = tmp.name

    export_alejandria(tmp_file, project_filter)

    # Instructions for manual import (since HTTP import not implemented yet)
    print(f"\n{Color.YELLOW}Next steps to complete sync:{Color.NC}")
    print(
        f"  1. Copy to server: scp {tmp_file} mroldan@ar-appsec-01.veritran.net:/tmp/local-sync.json"
    )
    print(
        f"  2. SSH and import: ssh mroldan@ar-appsec-01.veritran.net 'alejandria import --input /tmp/local-sync.json'"
    )
    print(f"\n{Color.BLUE}Or run this command:{Color.NC}")
    print(
        f"  scp {tmp_file} mroldan@ar-appsec-01.veritran.net:/tmp/local-sync.json && \\"
    )
    print(
        f"  ssh mroldan@ar-appsec-01.veritran.net 'alejandria import --input /tmp/local-sync.json'"
    )


def main():
    parser = argparse.ArgumentParser(
        description="Migrate memories from Engram to Alejandría and sync local/remote",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Preview Engram migration
  %(prog)s migrate --dry-run
  
  # Migrate Engram to Alejandría
  %(prog)s migrate --execute
  
  # Migrate only specific project
  %(prog)s migrate --execute --project Alejandria
  
  # Export local Alejandría to JSON
  %(prog)s export --output backup.json
  
  # Sync local to remote
  %(prog)s sync --remote http://ar-appsec-01.veritran.net:8080 --api-key YOUR_KEY
        """,
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # Migrate command
    migrate_parser = subparsers.add_parser(
        "migrate", help="Migrate from Engram to Alejandría"
    )
    migrate_parser.add_argument(
        "--dry-run", action="store_true", help="Preview without migrating"
    )
    migrate_parser.add_argument(
        "--execute", action="store_true", help="Actually perform migration"
    )
    migrate_parser.add_argument("--project", help="Filter by project name")
    migrate_parser.add_argument(
        "--allow-duplicates", action="store_true", help="Allow duplicate topic_keys"
    )

    # Export command
    export_parser = subparsers.add_parser("export", help="Export Alejandría to JSON")
    export_parser.add_argument("--output", required=True, help="Output JSON file")
    export_parser.add_argument("--project", help="Filter by project/topic")

    # Sync command
    sync_parser = subparsers.add_parser("sync", help="Sync local to remote")
    sync_parser.add_argument("--remote", required=True, help="Remote server URL")
    sync_parser.add_argument(
        "--api-key", required=True, help="API key for remote server"
    )
    sync_parser.add_argument("--project", help="Filter by project/topic")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    print_banner()

    if args.command == "migrate":
        if not args.execute and not args.dry_run:
            print(
                f"{Color.YELLOW}Use --dry-run to preview or --execute to migrate{Color.NC}"
            )
            sys.exit(1)

        migrate_engram_to_alejandria(
            dry_run=args.dry_run,
            project_filter=args.project,
            skip_duplicates=not args.allow_duplicates,
        )

    elif args.command == "export":
        export_alejandria(args.output, args.project)

    elif args.command == "sync":
        sync_local_to_remote(args.remote, args.api_key, args.project)


if __name__ == "__main__":
    main()
