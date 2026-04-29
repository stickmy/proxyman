use serde::Deserialize;
use serde_json::Value;

use crate::{
    app_conf,
    core_api::CORE_API_VERSION,
    core_api::session::{
        CoreSessionApi, ExportSessionHarRequest, ImportSessionHarRequest, JsonlSessionStore,
        LoadSessionBodyRequest, ReplayEdit, ReplaySessionRequest, SearchSessionExchangesRequest,
        SearchSessionExchangesResponse,
    },
    storage::{
        search_session_events_from_path, ReplayRequestEdit, ReplayResponse, SessionSearchFilter,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchRequest {
    method: Option<String>,
    host: Option<String>,
    status: Option<u16>,
}

#[tauri::command]
pub fn search_session_events(filter: SessionSearchRequest) -> Result<Vec<Value>, String> {
    search_session_events_from_path(
        app_conf::app_session_events_file(),
        &SessionSearchFilter {
            method: filter.method,
            host: filter.host,
            status: filter.status,
        },
    )
}

#[tauri::command]
pub fn search_session_exchanges(
    request: SearchSessionExchangesRequest,
) -> Result<SearchSessionExchangesResponse, String> {
    core_session_api().search_exchanges(request)
}

#[tauri::command]
pub fn export_session_har() -> Result<Value, String> {
    Ok(core_session_api()
        .export_har(ExportSessionHarRequest {
            api_version: CORE_API_VERSION,
        })?
        .har)
}

#[tauri::command]
pub fn import_session_har(har: Value) -> Result<usize, String> {
    Ok(core_session_api()
        .import_har(ImportSessionHarRequest {
            api_version: CORE_API_VERSION,
            har,
        })?
        .imported)
}

#[tauri::command]
pub fn load_session_body(body_ref: String) -> Result<String, String> {
    Ok(core_session_api()
        .load_body(LoadSessionBodyRequest {
            api_version: CORE_API_VERSION,
            body_ref,
        })?
        .body)
}

#[tauri::command]
pub async fn replay_session_request(
    exchange_id: String,
    edit: ReplayRequestEdit,
) -> Result<ReplayResponse, String> {
    let response = core_session_api()
        .replay(ReplaySessionRequest {
            api_version: CORE_API_VERSION,
            exchange_id,
            edit: ReplayEdit {
                method: edit.method,
                uri: edit.uri,
                headers: edit.headers,
                body: edit.body,
            },
        })
        .await?;

    Ok(ReplayResponse {
        status: response.status,
        headers: response.headers,
        body: response.body,
    })
}

fn core_session_api() -> CoreSessionApi<JsonlSessionStore> {
    CoreSessionApi::new(JsonlSessionStore::new(app_conf::app_session_events_file()))
}
