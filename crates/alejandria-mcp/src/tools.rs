//! MCP Tool handlers
//!
//! This module contains all tool implementations organized by category.

pub mod memoir;
pub mod memory;

// Mock store for testing
#[cfg(test)]
pub struct MockStore;

#[cfg(test)]
impl alejandria_core::MemoryStore for MockStore {
    fn store(&self, _memory: alejandria_core::Memory) -> alejandria_core::IcmResult<String> {
        Ok("01HQ7X8Y9Z0EXAMPLE0000".to_string())
    }

    fn get(&self, _id: &str) -> alejandria_core::IcmResult<Option<alejandria_core::Memory>> {
        Ok(None)
    }

    fn update(&self, _memory: alejandria_core::Memory) -> alejandria_core::IcmResult<()> {
        Ok(())
    }

    fn delete(&self, _id: &str) -> alejandria_core::IcmResult<()> {
        Ok(())
    }

    fn search_by_keywords(
        &self,
        _query: &str,
        _limit: usize,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::Memory>> {
        Ok(vec![])
    }

    fn search_by_embedding(
        &self,
        _embedding: &[f32],
        _limit: usize,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::Memory>> {
        Ok(vec![])
    }

    fn hybrid_search(
        &self,
        _query: &str,
        _embedding: &[f32],
        _limit: usize,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::Memory>> {
        Ok(vec![])
    }

    fn get_by_topic(
        &self,
        _topic: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::Memory>> {
        Ok(vec![])
    }

    fn get_by_topic_key(
        &self,
        _topic_key: &str,
    ) -> alejandria_core::IcmResult<Option<alejandria_core::Memory>> {
        Ok(None)
    }

    fn list_topics(
        &self,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::store::TopicInfo>> {
        Ok(vec![])
    }

    fn count(&self) -> alejandria_core::IcmResult<usize> {
        Ok(0)
    }

    fn stats(&self) -> alejandria_core::IcmResult<alejandria_core::store::StoreStats> {
        Ok(alejandria_core::store::StoreStats {
            total_memories: 0,
            active_memories: 0,
            deleted_memories: 0,
            total_size_mb: 0.0,
            by_importance: alejandria_core::store::ImportanceStats {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
            by_source: alejandria_core::store::SourceStats {
                user: 0,
                agent: 0,
                system: 0,
                external: 0,
            },
            avg_weight: 0.0,
            embeddings_enabled: false,
            last_decay_at: None,
        })
    }

    fn apply_decay(&self, _base_rate: f32) -> alejandria_core::IcmResult<usize> {
        Ok(0)
    }

    fn prune(&self, _threshold: f32) -> alejandria_core::IcmResult<usize> {
        Ok(0)
    }

    fn consolidate_topic(
        &self,
        _topic: &str,
        _min_memories: usize,
        _min_weight: f32,
    ) -> alejandria_core::IcmResult<String> {
        Ok("01HQ7X8Y9Z0CONSOLIDATED".to_string())
    }

    fn set_decay_profile(
        &self,
        _memory_id: &str,
        _profile_name: &str,
        _params: Option<serde_json::Value>,
    ) -> alejandria_core::IcmResult<()> {
        Ok(())
    }

    fn get_decay_stats(&self) -> alejandria_core::IcmResult<alejandria_core::store::DecayStats> {
        Ok(alejandria_core::store::DecayStats {
            total_with_profile: 0,
            total_default: 0,
            by_profile: std::collections::HashMap::new(),
            avg_weight_by_profile: std::collections::HashMap::new(),
            low_weight_count: 0,
            overall_avg_weight: 1.0,
        })
    }

    fn import_memories(
        &self,
        _input_path: &std::path::Path,
        _mode: alejandria_core::import::ImportMode,
    ) -> alejandria_core::IcmResult<alejandria_core::import::ImportResult> {
        Ok(alejandria_core::import::ImportResult::new())
    }
}

#[cfg(test)]
impl alejandria_core::MemoirStore for MockStore {
    fn create_memoir(
        &self,
        _memoir: alejandria_core::memoir_store::NewMemoir,
    ) -> alejandria_core::IcmResult<alejandria_core::Memoir> {
        Ok(alejandria_core::Memoir {
            id: "01HQ7X8Y9Z0EXAMPLE0000".to_string(),
            name: "test-memoir".to_string(),
            description: "Test memoir".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        })
    }

    fn list_memoirs(
        &self,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::memoir_store::MemoirSummary>> {
        Ok(vec![])
    }

    fn get_memoir(
        &self,
        _name: &str,
    ) -> alejandria_core::IcmResult<Option<alejandria_core::memoir_store::MemoirDetail>> {
        Ok(None)
    }

    fn add_concept(
        &self,
        _concept: alejandria_core::memoir_store::NewConcept,
    ) -> alejandria_core::IcmResult<alejandria_core::Concept> {
        Ok(alejandria_core::Concept {
            id: "01HQ7X8Y9Z0CONCEPT000".to_string(),
            memoir_id: "01HQ7X8Y9Z0EXAMPLE0000".to_string(),
            name: "Test Concept".to_string(),
            definition: "Test definition".to_string(),
            labels: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        })
    }

    fn update_concept(
        &self,
        _memoir: &str,
        _concept: &str,
        _updates: alejandria_core::memoir_store::ConceptUpdate,
    ) -> alejandria_core::IcmResult<()> {
        Ok(())
    }

    fn search_concepts(
        &self,
        _memoir: &str,
        _query: &str,
        _limit: u32,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::Concept>> {
        Ok(vec![])
    }

    fn search_concepts_all(
        &self,
        _query: &str,
        _limit: u32,
    ) -> alejandria_core::IcmResult<Vec<alejandria_core::memoir_store::ConceptMatch>> {
        Ok(vec![])
    }

    fn link_concepts(
        &self,
        _link: alejandria_core::memoir_store::NewConceptLink,
    ) -> alejandria_core::IcmResult<alejandria_core::ConceptLink> {
        Ok(alejandria_core::ConceptLink {
            id: "01HQ7X8Y9Z0LINK0000000".to_string(),
            memoir_id: "01HQ7X8Y9Z0EXAMPLE0000".to_string(),
            source_id: "01HQ7X8Y9Z0CONCEPT000".to_string(),
            target_id: "01HQ7X8Y9Z0CONCEPT001".to_string(),
            relation: alejandria_core::RelationType::RelatedTo,
            weight: 1.0,
            created_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        })
    }

    fn inspect_concept(
        &self,
        _memoir: &str,
        _concept: &str,
        _depth: u8,
    ) -> alejandria_core::IcmResult<alejandria_core::memoir_store::ConceptNeighborhood> {
        Ok(alejandria_core::memoir_store::ConceptNeighborhood {
            concept: alejandria_core::Concept {
                id: "01HQ7X8Y9Z0CONCEPT000".to_string(),
                memoir_id: "01HQ7X8Y9Z0EXAMPLE0000".to_string(),
                name: "Test Concept".to_string(),
                definition: "Test definition".to_string(),
                labels: vec![],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                metadata: serde_json::json!({}),
            },
            neighbors: vec![],
            depth: 1,
        })
    }
}
