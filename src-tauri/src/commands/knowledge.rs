use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub document_count: u32,
    pub documents: Vec<KnowledgeDocument>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDocument {
    pub id: String,
    pub name: String,
    pub content: String,
    pub doc_type: String,
    pub created_at: i64,
}

pub struct KnowledgeState(pub Mutex<Vec<KnowledgeBase>>);

#[tauri::command]
pub fn get_knowledge_bases(state: State<'_, KnowledgeState>) -> Result<Vec<KnowledgeBase>, String> {
    let bases = state.0.lock().map_err(|e| e.to_string())?;
    Ok(bases.clone())
}

#[tauri::command]
pub fn create_knowledge_base(data: serde_json::Value, state: State<'_, KnowledgeState>) -> Result<KnowledgeBase, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let base = KnowledgeBase {
        id,
        name: data.get("name").and_then(|v| v.as_str()).unwrap_or("新知识库").to_string(),
        description: data.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        document_count: 0,
        documents: vec![],
        created_at: now,
        updated_at: now,
    };
    let mut bases = state.0.lock().map_err(|e| e.to_string())?;
    bases.push(base.clone());
    Ok(base)
}

#[tauri::command]
pub fn delete_knowledge_base(id: String, state: State<'_, KnowledgeState>) -> Result<(), String> {
    let mut bases = state.0.lock().map_err(|e| e.to_string())?;
    bases.retain(|b| b.id != id);
    Ok(())
}

#[tauri::command]
pub fn add_knowledge_document(base_id: String, doc: serde_json::Value, state: State<'_, KnowledgeState>) -> Result<serde_json::Value, String> {
    let mut bases = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(base) = bases.iter_mut().find(|b| b.id == base_id) {
        let doc_entry = KnowledgeDocument {
            id: uuid::Uuid::new_v4().to_string(),
            name: doc.get("name").and_then(|v| v.as_str()).unwrap_or("untitled").to_string(),
            content: doc.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            doc_type: doc.get("type").and_then(|v| v.as_str()).unwrap_or("text").to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
        };
        base.documents.push(doc_entry);
        base.document_count = base.documents.len() as u32;
        base.updated_at = chrono::Utc::now().timestamp_millis();
        Ok(serde_json::json!({ "success": true }))
    } else {
        Err("Knowledge base not found".to_string())
    }
}

#[tauri::command]
pub fn search_knowledge(base_id: String, query: String, top_k: u32, state: State<'_, KnowledgeState>) -> Result<Vec<serde_json::Value>, String> {
    let bases = state.0.lock().map_err(|e| e.to_string())?;
    let base = bases.iter().find(|b| b.id == base_id);
    let base = match base {
        Some(b) => b,
        None => return Ok(vec![]),
    };

    let q = query.to_lowercase();
    let results: Vec<serde_json::Value> = base
        .documents
        .iter()
        .filter(|d| d.content.to_lowercase().contains(&q) || d.name.to_lowercase().contains(&q))
        .take(top_k as usize)
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "name": d.name,
                "content": if d.content.len() > 500 { &d.content[..500] } else { &d.content },
                "score": 1.0
            })
        })
        .collect();

    Ok(results)
}
