//! MemoirStore implementation for knowledge graph operations.
//!
//! This module provides the concrete implementation of the MemoirStore trait
//! for SQLite-backed memoir (knowledge graph) storage. It handles:
//!
//! - Memoir CRUD operations with UNIQUE name constraint
//! - Concept CRUD operations with UNIQUE(memoir_id, name) constraint
//! - FTS5 search across concepts (within memoir or all memoirs)
//! - Typed concept links with relation validation
//! - BFS graph traversal for neighborhood queries
//!
//! # Examples
//!
//! ```no_run
//! use alejandria_storage::SqliteStore;
//! use alejandria_core::{MemoirStore, NewMemoir, NewConcept, NewConceptLink, RelationType};
//!
//! # fn example() -> alejandria_core::error::IcmResult<()> {
//! let store = SqliteStore::open("alejandria.db")?;
//!
//! // Create a memoir
//! let memoir = store.create_memoir(NewMemoir {
//!     name: "rust-patterns".to_string(),
//!     description: "Common Rust design patterns".to_string(),
//! })?;
//!
//! // Add concepts
//! let builder = store.add_concept(NewConcept {
//!     memoir_name: "rust-patterns".to_string(),
//!     name: "Builder Pattern".to_string(),
//!     definition: "A creational pattern".to_string(),
//!     labels: vec!["design-pattern".to_string()],
//! })?;
//!
//! // Link concepts
//! store.link_concepts(NewConceptLink {
//!     memoir_name: "rust-patterns".to_string(),
//!     source_name: "Builder Pattern".to_string(),
//!     target_name: "Creational Pattern".to_string(),
//!     relation: RelationType::IsA,
//!     weight: 1.0,
//! })?;
//!
//! // Inspect neighborhood
//! let neighborhood = store.inspect_concept("rust-patterns", "Builder Pattern", 1)?;
//! # Ok(())
//! # }
//! ```

use alejandria_core::{
    error::{IcmError, IcmResult},
    memoir::{Concept, ConceptLink, Memoir, RelationType},
    memoir_store::{
        ConceptMatch, ConceptNeighborhood, ConceptUpdate, LinkDirection, LinkInfo, MemoirDetail,
        MemoirStore, MemoirSummary, NeighborInfo, NewConcept, NewConceptLink, NewMemoir,
    },
};
use chrono::{TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use std::str::FromStr;

use crate::store::{RusqliteResultExt, SqliteStore};

impl MemoirStore for SqliteStore {
    fn create_memoir(&self, memoir: NewMemoir) -> IcmResult<Memoir> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Generate ULID and timestamps
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let now_millis = now.timestamp_millis();

        // Check for existing memoir with same name
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM memoirs WHERE name = ?",
                params![memoir.name],
                |row| row.get(0),
            )
            .into_icm_result()?;

        if exists {
            return Err(IcmError::AlreadyExists(format!(
                "Memoir with name '{}' already exists",
                memoir.name
            )));
        }

        // Insert memoir
        conn.execute(
            r#"
            INSERT INTO memoirs (id, name, description, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            params![
                &id,
                &memoir.name,
                &memoir.description,
                now_millis,
                now_millis,
                "{}",
            ],
        )
        .into_icm_result()?;

        Ok(Memoir {
            id,
            name: memoir.name,
            description: memoir.description,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        })
    }

    fn list_memoirs(&self) -> IcmResult<Vec<MemoirSummary>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT 
                    m.id, m.name, m.description, m.created_at, m.updated_at,
                    COUNT(DISTINCT c.id) as concept_count,
                    COUNT(DISTINCT cl.id) as link_count
                FROM memoirs m
                LEFT JOIN concepts c ON c.memoir_id = m.id
                LEFT JOIN concept_links cl ON cl.memoir_id = m.id
                GROUP BY m.id
                ORDER BY m.created_at DESC
                "#,
            )
            .into_icm_result()?;

        let memoirs = stmt
            .query_map([], |row| {
                let created_at_millis: i64 = row.get(3)?;
                let updated_at_millis: i64 = row.get(4)?;

                Ok(MemoirSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    concept_count: row.get::<_, i64>(5)? as u32,
                    link_count: row.get::<_, i64>(6)? as u32,
                    created_at: Utc
                        .timestamp_millis_opt(created_at_millis)
                        .single()
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                    updated_at: Utc
                        .timestamp_millis_opt(updated_at_millis)
                        .single()
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                })
            })
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;

        Ok(memoirs)
    }

    fn get_memoir(&self, name: &str) -> IcmResult<Option<MemoirDetail>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Get memoir metadata
        let memoir_data: Option<(String, String, String, i64, i64)> = conn
            .query_row(
                "SELECT id, name, description, created_at, updated_at FROM memoirs WHERE name = ?",
                params![name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()
            .into_icm_result()?;

        let Some((memoir_id, memoir_name, description, created_at_millis, updated_at_millis)) =
            memoir_data
        else {
            return Ok(None);
        };

        // Get all concepts in the memoir
        let mut concept_stmt = conn
            .prepare(
                r#"
                SELECT id, memoir_id, name, definition, labels, created_at, updated_at, metadata
                FROM concepts
                WHERE memoir_id = ?
                ORDER BY name
                "#,
            )
            .into_icm_result()?;

        let concepts = concept_stmt
            .query_map(params![&memoir_id], |row| {
                let created_at_millis: i64 = row.get(5)?;
                let updated_at_millis: i64 = row.get(6)?;
                let labels_json: String = row.get(4)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();

                Ok(Concept {
                    id: row.get(0)?,
                    memoir_id: row.get(1)?,
                    name: row.get(2)?,
                    definition: row.get(3)?,
                    labels,
                    created_at: Utc
                        .timestamp_millis_opt(created_at_millis)
                        .single()
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                    updated_at: Utc
                        .timestamp_millis_opt(updated_at_millis)
                        .single()
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(7)?)
                        .unwrap_or(serde_json::json!({})),
                })
            })
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;

        // Get all links with concept names
        let mut link_stmt = conn
            .prepare(
                r#"
                SELECT 
                    cl.id,
                    src.name as source_name,
                    cl.relation,
                    tgt.name as target_name,
                    cl.weight
                FROM concept_links cl
                JOIN concepts src ON cl.source_id = src.id
                JOIN concepts tgt ON cl.target_id = tgt.id
                WHERE cl.memoir_id = ?
                ORDER BY src.name, tgt.name
                "#,
            )
            .into_icm_result()?;

        let links = link_stmt
            .query_map(params![&memoir_id], |row| {
                let relation_str: String = row.get(2)?;
                let relation = RelationType::from_str(&relation_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(LinkInfo {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    relation,
                    target: row.get(3)?,
                    weight: row.get(4)?,
                })
            })
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;

        Ok(Some(MemoirDetail {
            id: memoir_id,
            name: memoir_name,
            description,
            concepts,
            links,
            created_at: Utc
                .timestamp_millis_opt(created_at_millis)
                .single()
                .ok_or_else(|| IcmError::Database("Invalid timestamp".to_string()))?,
            updated_at: Utc
                .timestamp_millis_opt(updated_at_millis)
                .single()
                .ok_or_else(|| IcmError::Database("Invalid timestamp".to_string()))?,
        }))
    }

    fn add_concept(&self, concept: NewConcept) -> IcmResult<Concept> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Get memoir ID
        let memoir_id: Option<String> = conn
            .query_row(
                "SELECT id FROM memoirs WHERE name = ?",
                params![&concept.memoir_name],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(memoir_id) = memoir_id else {
            return Err(IcmError::NotFound {
                entity: "Memoir".to_string(),
                field: "name".to_string(),
                value: concept.memoir_name.clone(),
            });
        };

        // Check for existing concept with same name in memoir
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM concepts WHERE memoir_id = ? AND name = ?",
                params![&memoir_id, &concept.name],
                |row| row.get(0),
            )
            .into_icm_result()?;

        if exists {
            return Err(IcmError::AlreadyExists(format!(
                "Concept '{}' already exists in memoir '{}'",
                concept.name, concept.memoir_name
            )));
        }

        // Generate ULID and timestamps
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let now_millis = now.timestamp_millis();
        let labels_json = serde_json::to_string(&concept.labels)
            .map_err(|e| IcmError::InvalidInput(format!("Invalid labels: {}", e)))?;

        // Insert concept
        conn.execute(
            r#"
            INSERT INTO concepts (id, memoir_id, name, definition, labels, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                &id,
                &memoir_id,
                &concept.name,
                &concept.definition,
                &labels_json,
                now_millis,
                now_millis,
                "{}",
            ],
        )
        .into_icm_result()?;

        // Update memoir's updated_at timestamp
        conn.execute(
            "UPDATE memoirs SET updated_at = ? WHERE id = ?",
            params![now_millis, &memoir_id],
        )
        .into_icm_result()?;

        Ok(Concept {
            id,
            memoir_id,
            name: concept.name,
            definition: concept.definition,
            labels: concept.labels,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        })
    }

    fn update_concept(&self, memoir: &str, concept: &str, updates: ConceptUpdate) -> IcmResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Get memoir ID
        let memoir_id: Option<String> = conn
            .query_row(
                "SELECT id FROM memoirs WHERE name = ?",
                params![memoir],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(memoir_id) = memoir_id else {
            return Err(IcmError::NotFound {
                entity: "Memoir".to_string(),
                field: "name".to_string(),
                value: memoir.to_string(),
            });
        };

        // Check concept exists
        let concept_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM concepts WHERE memoir_id = ? AND name = ?",
                params![&memoir_id, concept],
                |row| row.get(0),
            )
            .into_icm_result()?;

        if !concept_exists {
            return Err(IcmError::NotFound {
                entity: "Concept".to_string(),
                field: "name".to_string(),
                value: format!("{} (in memoir {})", concept, memoir),
            });
        }

        let now_millis = Utc::now().timestamp_millis();

        // Build dynamic update query
        let mut update_fields = Vec::new();
        let mut params_list: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(definition) = &updates.definition {
            update_fields.push("definition = ?");
            params_list.push(Box::new(definition.clone()));
        }

        if let Some(labels) = &updates.labels {
            let labels_json = serde_json::to_string(labels)
                .map_err(|e| IcmError::InvalidInput(format!("Invalid labels: {}", e)))?;
            update_fields.push("labels = ?");
            params_list.push(Box::new(labels_json));
        }

        if update_fields.is_empty() {
            return Ok(()); // Nothing to update
        }

        update_fields.push("updated_at = ?");
        params_list.push(Box::new(now_millis));

        let query = format!(
            "UPDATE concepts SET {} WHERE memoir_id = ? AND name = ?",
            update_fields.join(", ")
        );

        params_list.push(Box::new(memoir_id.clone()));
        params_list.push(Box::new(concept.to_string()));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_list
            .iter()
            .map(|p| p.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        conn.execute(&query, params_refs.as_slice())
            .into_icm_result()?;

        // Update memoir's updated_at timestamp
        conn.execute(
            "UPDATE memoirs SET updated_at = ? WHERE id = ?",
            params![now_millis, &memoir_id],
        )
        .into_icm_result()?;

        Ok(())
    }

    fn search_concepts(&self, memoir: &str, query: &str, limit: u32) -> IcmResult<Vec<Concept>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Get memoir ID
        let memoir_id: Option<String> = conn
            .query_row(
                "SELECT id FROM memoirs WHERE name = ?",
                params![memoir],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(memoir_id) = memoir_id else {
            return Err(IcmError::NotFound {
                entity: "Memoir".to_string(),
                field: "name".to_string(),
                value: memoir.to_string(),
            });
        };

        // FTS5 search within memoir
        let mut stmt = conn
            .prepare(
                r#"
                SELECT 
                    c.id, c.memoir_id, c.name, c.definition, c.labels, 
                    c.created_at, c.updated_at, c.metadata
                FROM concepts_fts fts
                JOIN concepts c ON fts.rowid = c.rowid
                WHERE fts.memoir_id = ? AND concepts_fts MATCH ?
                ORDER BY rank
                LIMIT ?
                "#,
            )
            .into_icm_result()?;

        let concepts = stmt
            .query_map(params![&memoir_id, query, limit], |row| {
                let created_at_millis: i64 = row.get(5)?;
                let updated_at_millis: i64 = row.get(6)?;
                let labels_json: String = row.get(4)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();

                Ok(Concept {
                    id: row.get(0)?,
                    memoir_id: row.get(1)?,
                    name: row.get(2)?,
                    definition: row.get(3)?,
                    labels,
                    created_at: Utc
                        .timestamp_millis_opt(created_at_millis)
                        .single()
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                    updated_at: Utc
                        .timestamp_millis_opt(updated_at_millis)
                        .single()
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(7)?)
                        .unwrap_or(serde_json::json!({})),
                })
            })
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;

        Ok(concepts)
    }

    fn search_concepts_all(&self, query: &str, limit: u32) -> IcmResult<Vec<ConceptMatch>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // FTS5 search across all memoirs
        let mut stmt = conn
            .prepare(
                r#"
                SELECT 
                    m.name as memoir_name,
                    c.id, c.memoir_id, c.name, c.definition, c.labels,
                    c.created_at, c.updated_at, c.metadata,
                    rank
                FROM concepts_fts fts
                JOIN concepts c ON fts.rowid = c.rowid
                JOIN memoirs m ON c.memoir_id = m.id
                WHERE concepts_fts MATCH ?
                ORDER BY rank
                LIMIT ?
                "#,
            )
            .into_icm_result()?;

        let matches = stmt
            .query_map(params![query, limit], |row| {
                let created_at_millis: i64 = row.get(6)?;
                let updated_at_millis: i64 = row.get(7)?;
                let labels_json: String = row.get(5)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
                let rank: f32 = row.get(9)?;

                Ok(ConceptMatch {
                    memoir_name: row.get(0)?,
                    concept: Concept {
                        id: row.get(1)?,
                        memoir_id: row.get(2)?,
                        name: row.get(3)?,
                        definition: row.get(4)?,
                        labels,
                        created_at: Utc
                            .timestamp_millis_opt(created_at_millis)
                            .single()
                            .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                        updated_at: Utc
                            .timestamp_millis_opt(updated_at_millis)
                            .single()
                            .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                        metadata: serde_json::from_str(&row.get::<_, String>(8)?)
                            .unwrap_or(serde_json::json!({})),
                    },
                    score: -rank, // Negate rank (higher is better, but FTS5 rank is negative)
                })
            })
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;

        Ok(matches)
    }

    fn link_concepts(&self, link: NewConceptLink) -> IcmResult<ConceptLink> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Get memoir ID
        let memoir_id: Option<String> = conn
            .query_row(
                "SELECT id FROM memoirs WHERE name = ?",
                params![&link.memoir_name],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(memoir_id) = memoir_id else {
            return Err(IcmError::NotFound {
                entity: "Memoir".to_string(),
                field: "name".to_string(),
                value: link.memoir_name.clone(),
            });
        };

        // Get source concept ID
        let source_id: Option<String> = conn
            .query_row(
                "SELECT id FROM concepts WHERE memoir_id = ? AND name = ?",
                params![&memoir_id, &link.source_name],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(source_id) = source_id else {
            return Err(IcmError::NotFound {
                entity: "Concept".to_string(),
                field: "name".to_string(),
                value: format!(
                    "{} (source in memoir {})",
                    link.source_name, link.memoir_name
                ),
            });
        };

        // Get target concept ID
        let target_id: Option<String> = conn
            .query_row(
                "SELECT id FROM concepts WHERE memoir_id = ? AND name = ?",
                params![&memoir_id, &link.target_name],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(target_id) = target_id else {
            return Err(IcmError::NotFound {
                entity: "Concept".to_string(),
                field: "name".to_string(),
                value: format!(
                    "{} (target in memoir {})",
                    link.target_name, link.memoir_name
                ),
            });
        };

        // Check for self-loop
        if source_id == target_id {
            return Err(IcmError::InvalidInput(format!(
                "Cannot create self-loop: source and target are the same concept '{}'",
                link.source_name
            )));
        }

        // Check if link already exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM concept_links WHERE memoir_id = ? AND source_id = ? AND target_id = ? AND relation = ?",
                params![&memoir_id, &source_id, &target_id, link.relation.as_str()],
                |row| row.get(0),
            )
            .into_icm_result()?;

        if exists {
            return Err(IcmError::AlreadyExists(format!(
                "Link already exists: {} --[{}]--> {}",
                link.source_name,
                link.relation.as_str(),
                link.target_name
            )));
        }

        // Generate ULID and timestamp
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let now_millis = now.timestamp_millis();

        // Insert link
        conn.execute(
            r#"
            INSERT INTO concept_links (id, memoir_id, source_id, target_id, relation, weight, created_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                &id,
                &memoir_id,
                &source_id,
                &target_id,
                link.relation.as_str(),
                link.weight,
                now_millis,
                "{}",
            ],
        )
        .into_icm_result()?;

        // Update memoir's updated_at timestamp
        conn.execute(
            "UPDATE memoirs SET updated_at = ? WHERE id = ?",
            params![now_millis, &memoir_id],
        )
        .into_icm_result()?;

        Ok(ConceptLink {
            id,
            memoir_id,
            source_id,
            target_id,
            relation: link.relation,
            weight: link.weight,
            created_at: now,
            metadata: serde_json::json!({}),
        })
    }

    fn inspect_concept(
        &self,
        memoir: &str,
        concept: &str,
        depth: u8,
    ) -> IcmResult<ConceptNeighborhood> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to acquire connection lock: {}", e)))?;

        // Get memoir ID
        let memoir_id: Option<String> = conn
            .query_row(
                "SELECT id FROM memoirs WHERE name = ?",
                params![memoir],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?;

        let Some(memoir_id) = memoir_id else {
            return Err(IcmError::NotFound {
                entity: "Memoir".to_string(),
                field: "name".to_string(),
                value: memoir.to_string(),
            });
        };

        // Get concept
        type ConceptRow = (String, String, String, String, String, i64, i64, String);
        let concept_data: Option<ConceptRow> = conn
            .query_row(
                "SELECT id, memoir_id, name, definition, labels, created_at, updated_at, metadata FROM concepts WHERE memoir_id = ? AND name = ?",
                params![&memoir_id, concept],
                |row: &rusqlite::Row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, String>(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .optional()
            .into_icm_result()?;

        let Some((
            concept_id,
            _,
            name,
            definition,
            labels_json,
            created_at_millis,
            updated_at_millis,
            metadata_json,
        )) = concept_data
        else {
            return Err(IcmError::NotFound {
                entity: "Concept".to_string(),
                field: "name".to_string(),
                value: format!("{} (in memoir {})", concept, memoir),
            });
        };

        let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
        let concept_obj = Concept {
            id: concept_id.clone(),
            memoir_id: memoir_id.clone(),
            name,
            definition,
            labels,
            created_at: Utc
                .timestamp_millis_opt(created_at_millis)
                .single()
                .ok_or_else(|| IcmError::Database("Invalid timestamp".to_string()))?,
            updated_at: Utc
                .timestamp_millis_opt(updated_at_millis)
                .single()
                .ok_or_else(|| IcmError::Database("Invalid timestamp".to_string()))?,
            metadata: serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({})),
        };

        // BFS traversal using recursive CTE
        // We need to traverse both outgoing and incoming links
        let mut stmt = conn
            .prepare(
                r#"
                WITH RECURSIVE neighborhood(concept_id, relation, direction, weight, level) AS (
                    -- Base case: direct neighbors (outgoing)
                    SELECT 
                        cl.target_id as concept_id,
                        cl.relation,
                        'outgoing' as direction,
                        cl.weight,
                        1 as level
                    FROM concept_links cl
                    WHERE cl.source_id = ?1
                    
                    UNION ALL
                    
                    -- Base case: direct neighbors (incoming)
                    SELECT 
                        cl.source_id as concept_id,
                        cl.relation,
                        'incoming' as direction,
                        cl.weight,
                        1 as level
                    FROM concept_links cl
                    WHERE cl.target_id = ?1
                    
                    UNION ALL
                    
                    -- Recursive case: expand further (outgoing)
                    SELECT 
                        cl.target_id as concept_id,
                        cl.relation,
                        'outgoing' as direction,
                        cl.weight,
                        n.level + 1
                    FROM neighborhood n
                    JOIN concept_links cl ON cl.source_id = n.concept_id
                    WHERE n.level < ?2
                    
                    UNION ALL
                    
                    -- Recursive case: expand further (incoming)
                    SELECT 
                        cl.source_id as concept_id,
                        cl.relation,
                        'incoming' as direction,
                        cl.weight,
                        n.level + 1
                    FROM neighborhood n
                    JOIN concept_links cl ON cl.target_id = n.concept_id
                    WHERE n.level < ?2
                )
                SELECT DISTINCT
                    n.relation,
                    n.direction,
                    n.weight,
                    n.level,
                    c.id, c.memoir_id, c.name, c.definition, c.labels,
                    c.created_at, c.updated_at, c.metadata
                FROM neighborhood n
                JOIN concepts c ON n.concept_id = c.id
                WHERE n.concept_id != ?1
                ORDER BY n.level, c.name
                "#,
            )
            .into_icm_result()?;

        let neighbors = stmt
            .query_map(params![&concept_id, depth], |row| {
                let relation_str: String = row.get(0)?;
                let direction_str: String = row.get(1)?;
                let weight: f32 = row.get(2)?;
                let level: u8 = row.get::<_, i64>(3)? as u8;

                let relation = RelationType::from_str(&relation_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                let direction = match direction_str.as_str() {
                    "outgoing" => LinkDirection::Outgoing,
                    "incoming" => LinkDirection::Incoming,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };

                let created_at_millis: i64 = row.get(9)?;
                let updated_at_millis: i64 = row.get(10)?;
                let labels_json: String = row.get(8)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();

                Ok(NeighborInfo {
                    relation,
                    direction,
                    concept: Concept {
                        id: row.get(4)?,
                        memoir_id: row.get(5)?,
                        name: row.get(6)?,
                        definition: row.get(7)?,
                        labels,
                        created_at: Utc
                            .timestamp_millis_opt(created_at_millis)
                            .single()
                            .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                        updated_at: Utc
                            .timestamp_millis_opt(updated_at_millis)
                            .single()
                            .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                        metadata: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or(serde_json::json!({})),
                    },
                    weight,
                    level,
                })
            })
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;

        Ok(ConceptNeighborhood {
            concept: concept_obj,
            neighbors,
            depth,
        })
    }
}
