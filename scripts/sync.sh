#!/bin/bash
#
# Alejandría Sync Helper
# Quick commands for common migration/sync tasks
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATE_SCRIPT="$SCRIPT_DIR/migrate.py"
REMOTE_SERVER="ar-appsec-01.veritran.net"
REMOTE_URL="http://${REMOTE_SERVER}:8080"
API_KEY="alejandria-prod-initial-key-2026"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

show_help() {
    cat << EOF
${BLUE}Alejandría Sync Helper${NC}

Quick commands for migrating and syncing memories:

${GREEN}Migration Commands:${NC}
  $0 engram-preview     Preview Engram → Alejandría migration
  $0 engram-migrate     Migrate ALL Engram observations to Alejandría
  $0 engram-project X   Migrate only project X from Engram

${GREEN}Backup Commands:${NC}
  $0 backup-local       Export local Alejandría to JSON
  $0 backup-remote      Export remote Alejandría to JSON (via SSH)

${GREEN}Sync Commands:${NC}
  $0 push               Push local memories to remote server
  $0 pull               Pull remote memories to local
  $0 stats              Show stats for local and remote

${GREEN}Examples:${NC}
  # Preview what would be migrated from Engram
  $0 engram-preview

  # Actually migrate from Engram
  $0 engram-migrate

  # Backup local to file
  $0 backup-local

  # Push local memories to remote server
  $0 push

EOF
}

case "$1" in
    engram-preview)
        echo -e "${BLUE}Previewing Engram → Alejandría migration...${NC}"
        python3 "$MIGRATE_SCRIPT" migrate --dry-run
        ;;
    
    engram-migrate)
        echo -e "${YELLOW}⚠ This will migrate ALL Engram observations to Alejandría.${NC}"
        echo -e "${YELLOW}Duplicates (by topic_key) will be skipped.${NC}"
        read -p "Continue? (yes/no): " confirm
        if [[ "$confirm" == "yes" ]]; then
            python3 "$MIGRATE_SCRIPT" migrate --execute
        else
            echo -e "${RED}Cancelled.${NC}"
        fi
        ;;
    
    engram-project)
        if [[ -z "$2" ]]; then
            echo -e "${RED}Error: Project name required${NC}"
            echo "Usage: $0 engram-project <project-name>"
            exit 1
        fi
        echo -e "${BLUE}Migrating project '$2' from Engram...${NC}"
        python3 "$MIGRATE_SCRIPT" migrate --execute --project "$2"
        ;;
    
    backup-local)
        BACKUP_FILE="${HOME}/alejandria-local-backup-$(date +%Y%m%d-%H%M%S).json"
        echo -e "${BLUE}Backing up local Alejandría to ${BACKUP_FILE}...${NC}"
        python3 "$MIGRATE_SCRIPT" export --output "$BACKUP_FILE"
        echo -e "${GREEN}✓ Backup saved to: ${BACKUP_FILE}${NC}"
        ;;
    
    backup-remote)
        BACKUP_FILE="${HOME}/alejandria-remote-backup-$(date +%Y%m%d-%H%M%S).json"
        echo -e "${BLUE}Backing up remote Alejandría...${NC}"
        
        # Export on server
        ssh mroldan@${REMOTE_SERVER} "alejandria export --output /tmp/remote-backup.json"
        
        # Download
        scp mroldan@${REMOTE_SERVER}:/tmp/remote-backup.json "$BACKUP_FILE"
        
        # Cleanup
        ssh mroldan@${REMOTE_SERVER} "rm /tmp/remote-backup.json"
        
        echo -e "${GREEN}✓ Remote backup saved to: ${BACKUP_FILE}${NC}"
        ;;
    
    push)
        echo -e "${BLUE}Pushing local → remote...${NC}"
        
        # Export local using CLI (correct format)
        TEMP_FILE="/tmp/alejandria-local-export-$(date +%s).json"
        alejandria export --output "$TEMP_FILE"
        
        echo -e "${BLUE}Uploading to server...${NC}"
        scp "$TEMP_FILE" mroldan@${REMOTE_SERVER}:/tmp/alejandria-import.json
        
        echo -e "${BLUE}Importing on server...${NC}"
        ssh mroldan@${REMOTE_SERVER} "alejandria import --input /tmp/alejandria-import.json"
        
        # Cleanup
        rm "$TEMP_FILE"
        ssh mroldan@${REMOTE_SERVER} "rm /tmp/alejandria-import.json"
        
        echo -e "${GREEN}✓ Push complete${NC}"
        ;;
    
    pull)
        echo -e "${BLUE}Pulling remote → local...${NC}"
        
        # Export on server
        echo -e "${BLUE}Exporting from server...${NC}"
        ssh mroldan@${REMOTE_SERVER} "alejandria export --output /tmp/alejandria-export.json"
        
        # Download
        TEMP_FILE="/tmp/alejandria-remote-export-$(date +%s).json"
        scp mroldan@${REMOTE_SERVER}:/tmp/alejandria-export.json "$TEMP_FILE"
        
        # Import locally
        echo -e "${BLUE}Importing to local...${NC}"
        alejandria import --input "$TEMP_FILE"
        
        # Cleanup
        rm "$TEMP_FILE"
        ssh mroldan@${REMOTE_SERVER} "rm /tmp/alejandria-export.json"
        
        echo -e "${GREEN}✓ Pull complete${NC}"
        ;;
    
    stats)
        echo -e "${BLUE}Local Alejandría stats:${NC}"
        alejandria stats
        
        echo ""
        echo -e "${BLUE}Remote Alejandría stats:${NC}"
        ssh mroldan@${REMOTE_SERVER} "alejandria stats"
        
        echo ""
        echo -e "${BLUE}Engram stats:${NC}"
        sqlite3 ~/.engram/engram.db "SELECT COUNT(*) as total, scope FROM observations WHERE deleted_at IS NULL GROUP BY scope;"
        ;;
    
    help|--help|-h|"")
        show_help
        ;;
    
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo ""
        show_help
        exit 1
        ;;
esac
