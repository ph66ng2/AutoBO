use crate::services::auth::{AuthRuntime, AuthView};
use uuid::Uuid;

pub struct AuthAppState {
    runtime: Result<AuthRuntime, String>,
}

impl AuthAppState {
    pub fn from_env() -> Self {
        Self {
            runtime: AuthRuntime::from_env().map_err(|error| error.to_string()),
        }
    }

    fn runtime(&self) -> Result<&AuthRuntime, String> {
        self.runtime.as_ref().map_err(|error| error.clone())
    }
}

fn map_error(error: crate::services::auth::AuthError) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn login_autobo(
    state: tauri::State<'_, AuthAppState>,
    email: String,
    password: String,
) -> Result<AuthView, String> {
    state
        .runtime()?
        .login(&email, &password)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub async fn restaurar_sessao_autobo(
    state: tauri::State<'_, AuthAppState>,
) -> Result<AuthView, String> {
    state.runtime()?.restore().await.map_err(map_error)
}

#[tauri::command]
pub async fn selecionar_empresa_autobo(
    state: tauri::State<'_, AuthAppState>,
    company_id: String,
) -> Result<AuthView, String> {
    let company_id = Uuid::parse_str(&company_id).map_err(|_| "Empresa inválida.".to_string())?;
    state
        .runtime()?
        .select_company(company_id)
        .await
        .map_err(map_error)
}

#[tauri::command]
pub fn bloquear_sessao_autobo(state: tauri::State<'_, AuthAppState>) -> Result<AuthView, String> {
    state.runtime()?.lock().map_err(map_error)
}

#[tauri::command]
pub async fn sair_sessao_autobo(
    state: tauri::State<'_, AuthAppState>,
) -> Result<AuthView, String> {
    state.runtime()?.sign_out().await.map_err(map_error)
}

#[tauri::command]
pub fn perfil_sessao_autobo(state: tauri::State<'_, AuthAppState>) -> Result<AuthView, String> {
    state.runtime()?.current().map_err(map_error)
}
