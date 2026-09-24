//! Login SaaS do AutoBO.
//!
//! Tokens ficam no keyring. `company_id` e papel saem da membership AutoBO no
//! AutoPlatform, nunca de `user_metadata`/`app_metadata` do JWT. ADMIN AutoOS
//! não concede FISCAL nem `autobo_access`.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::secrets::{self, SecretsError};

const PRODUCT: &str = "autobo";
const CAPABILITY: &str = "autobo_access";
const KEYRING_SESSION: &str = "saas.session.v1";
const REFRESH_MARGIN_SECS: i64 = 60;
const HTTP_TIMEOUT_SECS: u64 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Operator,
    Fiscal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Operate,
    ManageCompany,
    Fiscal,
}

impl Role {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "admin" => Some(Self::Admin),
            "operator" => Some(Self::Operator),
            "fiscal" => Some(Self::Fiscal),
            _ => None,
        }
    }

    fn permissions(self) -> Vec<Permission> {
        match self {
            Self::Admin => vec![Permission::Operate, Permission::ManageCompany],
            Self::Operator => vec![Permission::Operate],
            Self::Fiscal => vec![Permission::Fiscal],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    Configuration(String),
    InvalidCredentials,
    Unavailable(String),
    Network,
    Expired,
    Keyring,
    Denied,
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(message) | Self::Unavailable(message) => formatter.write_str(message),
            Self::InvalidCredentials => formatter.write_str("Email ou senha inválidos."),
            Self::Network => formatter.write_str("Sem conexão com o serviço de autenticação."),
            Self::Expired => formatter.write_str("Sua sessão expirou. Entre novamente para continuar."),
            Self::Keyring => formatter.write_str("Não foi possível acessar o cofre seguro do sistema."),
            Self::Denied => formatter.write_str("Esta conta não tem acesso ao AutoBO."),
        }
    }
}

impl From<SecretsError> for AuthError {
    fn from(error: SecretsError) -> Self {
        match error {
            SecretsError::NotFound => Self::Expired,
            _ => Self::Keyring,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredSession {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    account_id: Uuid,
    email: String,
    company_id: Option<Uuid>,
}

impl fmt::Debug for StoredSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredSession")
            .field("account_id", &self.account_id)
            .field("email", &self.email)
            .field("company_id", &self.company_id)
            .field("expires_at", &self.expires_at)
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSession {
    pub account_id: Uuid,
    pub company_id: Uuid,
    pub email: String,
    pub role: Role,
    pub permissions: Vec<Permission>,
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOption {
    pub company_id: Uuid,
    pub label: String,
    pub role: Role,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthView {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub companies: Option<Vec<CompanyOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<PublicSession>,
}

impl AuthView {
    fn signed_out(message: Option<String>) -> Self {
        Self {
            kind: "signed_out".into(),
            message,
            email: None,
            companies: None,
            session: None,
        }
    }

    fn locked(email: Option<String>) -> Self {
        Self {
            kind: "locked".into(),
            message: Some("Sessão bloqueada neste dispositivo.".into()),
            email,
            companies: None,
            session: None,
        }
    }

    fn expired(email: Option<String>, message: Option<String>) -> Self {
        Self {
            kind: "expired".into(),
            message: Some(message.unwrap_or_else(|| AuthError::Expired.to_string())),
            email,
            companies: None,
            session: None,
        }
    }

    fn needs_company(email: String, companies: Vec<CompanyOption>) -> Self {
        Self {
            kind: "needs_company".into(),
            message: None,
            email: Some(email),
            companies: Some(companies),
            session: None,
        }
    }

    fn authenticated(session: PublicSession) -> Self {
        Self {
            kind: "authenticated".into(),
            message: None,
            email: Some(session.email.clone()),
            companies: None,
            session: Some(session),
        }
    }
}

#[derive(Clone)]
struct IssuedAuth {
    account_id: Uuid,
    email: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

struct Membership {
    company_id: Uuid,
    label: String,
    role: Role,
}

#[derive(Clone)]
enum Store {
    #[allow(dead_code)]
    Memory(Arc<Mutex<Option<StoredSession>>>),
    Keyring,
}

impl Store {
    fn load(&self) -> Result<Option<StoredSession>, AuthError> {
        match self {
            Self::Memory(slot) => Ok(slot.lock().expect("session store").clone()),
            Self::Keyring => match secrets::get_secret(KEYRING_SESSION) {
                Ok(raw) => {
                    let session: StoredSession =
                        serde_json::from_str(&raw).map_err(|_| AuthError::Expired)?;
                    Ok(Some(session))
                }
                Err(SecretsError::NotFound) => Ok(None),
                Err(_) => Err(AuthError::Keyring),
            },
        }
    }

    fn save(&self, session: &StoredSession) -> Result<(), AuthError> {
        match self {
            Self::Memory(slot) => {
                *slot.lock().expect("session store") = Some(session.clone());
                Ok(())
            }
            Self::Keyring => {
                let raw = serde_json::to_string(session).map_err(|_| AuthError::Keyring)?;
                secrets::set_secret(KEYRING_SESSION, &raw).map_err(AuthError::from)
            }
        }
    }

    fn clear(&self) -> Result<(), AuthError> {
        match self {
            Self::Memory(slot) => {
                *slot.lock().expect("session store") = None;
                Ok(())
            }
            Self::Keyring => secrets::delete_secret(KEYRING_SESSION).map_err(AuthError::from),
        }
    }
}

#[derive(Clone)]
struct FakeUser {
    password: String,
    issued: IssuedAuth,
}

#[derive(Clone, Default)]
struct FakeIssuer {
    users: HashMap<String, FakeUser>,
}

impl FakeIssuer {
    fn with_user(mut self, email: &str, password: &str, account_id: Uuid) -> Self {
        self.users.insert(
            email.to_ascii_lowercase(),
            FakeUser {
                password: password.into(),
                issued: IssuedAuth {
                    account_id,
                    email: email.to_ascii_lowercase(),
                    access_token: format!("access-{account_id}"),
                    refresh_token: format!("refresh-{account_id}"),
                    expires_at: 1_900_000_000,
                },
            },
        );
        self
    }

    fn password_grant(&self, email: &str, password: &str) -> Result<IssuedAuth, AuthError> {
        let user = self
            .users
            .get(&email.to_ascii_lowercase())
            .ok_or(AuthError::InvalidCredentials)?;
        if user.password != password {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(user.issued.clone())
    }

    fn refresh(&self, refresh_token: &str) -> Result<IssuedAuth, AuthError> {
        self.users
            .values()
            .find(|user| user.issued.refresh_token == refresh_token)
            .map(|user| user.issued.clone())
            .ok_or(AuthError::Expired)
    }
}

#[derive(Clone)]
struct HttpIssuer {
    url: String,
    publishable_key: String,
    client: Client,
}

impl HttpIssuer {
    fn new(url: String, publishable_key: String) -> Result<Self, AuthError> {
        validate_https_url(&url, "SUPABASE_URL")?;
        validate_publishable_key(&publishable_key)?;
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            publishable_key,
            client: http_client()?,
        })
    }

    async fn password_grant(&self, email: &str, password: &str) -> Result<IssuedAuth, AuthError> {
        let response = self
            .client
            .post(format!("{}/auth/v1/token?grant_type=password", self.url))
            .header("apikey", &self.publishable_key)
            .header("Authorization", format!("Bearer {}", self.publishable_key))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(map_reqwest)?;
        parse_supabase_session(response.status(), response.text().await.map_err(map_reqwest)?)
    }

    async fn refresh(&self, refresh_token: &str) -> Result<IssuedAuth, AuthError> {
        let response = self
            .client
            .post(format!(
                "{}/auth/v1/token?grant_type=refresh_token",
                self.url
            ))
            .header("apikey", &self.publishable_key)
            .header("Authorization", format!("Bearer {}", self.publishable_key))
            .json(&serde_json::json!({ "refresh_token": refresh_token }))
            .send()
            .await
            .map_err(map_reqwest)?;
        parse_supabase_session(response.status(), response.text().await.map_err(map_reqwest)?)
    }

    async fn sign_out(&self, access_token: &str) -> Result<(), AuthError> {
        let response = self
            .client
            .post(format!("{}/auth/v1/logout", self.url))
            .header("apikey", &self.publishable_key)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(map_reqwest)?;
        if response.status().is_success() || response.status() == StatusCode::NO_CONTENT {
            return Ok(());
        }
        Err(AuthError::Network)
    }
}

#[derive(Clone)]
enum Issuer {
    Fake(FakeIssuer),
    Http(HttpIssuer),
}

impl Issuer {
    async fn password_grant(&self, email: &str, password: &str) -> Result<IssuedAuth, AuthError> {
        match self {
            Self::Fake(fake) => fake.password_grant(email, password),
            Self::Http(http) => http.password_grant(email, password).await,
        }
    }

    async fn refresh(&self, refresh_token: &str) -> Result<IssuedAuth, AuthError> {
        match self {
            Self::Fake(fake) => fake.refresh(refresh_token),
            Self::Http(http) => http.refresh(refresh_token).await,
        }
    }

    async fn sign_out(&self, access_token: &str) -> Result<(), AuthError> {
        match self {
            Self::Fake(_) => Ok(()),
            Self::Http(http) => http.sign_out(access_token).await,
        }
    }
}

#[derive(Clone)]
struct FakeMembership {
    account_id: Uuid,
    company_id: Uuid,
    label: String,
    product: String,
    role: Role,
    status: String,
    capabilities: Vec<String>,
}

#[derive(Clone, Default)]
struct FakePlatform {
    rows: Vec<FakeMembership>,
}

impl FakePlatform {
    fn with_row(mut self, row: FakeMembership) -> Self {
        self.rows.push(row);
        self
    }

    fn list(&self, account_id: Uuid, company_id: Option<Uuid>) -> Result<Vec<Membership>, AuthError> {
        let rows = self
            .rows
            .iter()
            .filter(|row| {
                row.account_id == account_id
                    && row.product == PRODUCT
                    && row.status == "active"
                    && row.capabilities.iter().any(|item| item == CAPABILITY)
                    && company_id.map(|id| id == row.company_id).unwrap_or(true)
            })
            .map(|row| Membership {
                company_id: row.company_id,
                label: row.label.clone(),
                role: row.role,
            })
            .collect::<Vec<_>>();
        if company_id.is_some() && rows.is_empty() {
            return Err(AuthError::Denied);
        }
        Ok(rows)
    }
}

#[derive(Clone)]
struct HttpPlatform {
    url: String,
    client: Client,
}

impl HttpPlatform {
    fn new(url: String) -> Result<Self, AuthError> {
        validate_https_url(&url, "AUTOPLATFORM_URL")?;
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            client: http_client()?,
        })
    }

    async fn list(
        &self,
        access_token: &str,
        company_id: Option<Uuid>,
    ) -> Result<(Uuid, Vec<Membership>), AuthError> {
        let mut body = serde_json::json!({ "product": PRODUCT });
        if let Some(company_id) = company_id {
            body["company_id"] = serde_json::json!(company_id);
        }
        let response = self
            .client
            .post(format!("{}/v1/session/resolve", self.url))
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest)?;
        let status = response.status();
        let text = response.text().await.map_err(map_reqwest)?;
        parse_platform_memberships(status, &text, company_id)
    }
}

#[derive(Clone)]
enum Platform {
    Fake(FakePlatform),
    Http(HttpPlatform),
}

impl Platform {
    async fn list(
        &self,
        account_id: Uuid,
        access_token: &str,
        company_id: Option<Uuid>,
    ) -> Result<Vec<Membership>, AuthError> {
        match self {
            Self::Fake(fake) => fake.list(account_id, company_id),
            Self::Http(http) => {
                let (resolved_account, memberships) = http.list(access_token, company_id).await?;
                if resolved_account != account_id {
                    return Err(AuthError::Denied);
                }
                Ok(memberships)
            }
        }
    }
}

#[derive(Clone)]
pub struct AuthRuntime {
    store: Store,
    issuer: Issuer,
    platform: Platform,
    unlocked: Arc<Mutex<bool>>,
    last_session: Arc<Mutex<Option<PublicSession>>>,
}

impl AuthRuntime {
    pub fn from_env() -> Result<Self, AuthError> {
        crate::db::load_dotenv();
        let adapter = std::env::var("AUTOBO_AUTH_ADAPTER").unwrap_or_else(|_| "http".into());
        if adapter == "fake" {
            return Ok(Self::fake_demo());
        }
        Ok(Self {
            store: Store::Keyring,
            issuer: Issuer::Http(HttpIssuer::new(
                required_env("SUPABASE_URL")?,
                required_env("SUPABASE_PUBLISHABLE_KEY")?,
            )?),
            platform: Platform::Http(HttpPlatform::new(required_env("AUTOPLATFORM_URL")?)?),
            unlocked: Arc::new(Mutex::new(false)),
            last_session: Arc::new(Mutex::new(None)),
        })
    }

    fn fake_demo() -> Self {
        Self::harness(demo_issuer(), demo_platform(), Store::Keyring)
    }

    fn harness(issuer: FakeIssuer, platform: FakePlatform, store: Store) -> Self {
        Self {
            store,
            issuer: Issuer::Fake(issuer),
            platform: Platform::Fake(platform),
            unlocked: Arc::new(Mutex::new(false)),
            last_session: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthView, AuthError> {
        let email = email.trim().to_ascii_lowercase();
        if email.is_empty() || password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }
        let issued = self.issuer.password_grant(&email, password).await?;
        self.activate(issued).await
    }

    pub async fn restore(&self) -> Result<AuthView, AuthError> {
        let Some(stored) = self.store.load()? else {
            return Ok(AuthView::signed_out(None));
        };
        let email = stored.email.clone();
        let issued = match self.refresh_if_needed(stored).await {
            Ok(issued) => issued,
            Err(AuthError::Expired) => return Ok(AuthView::expired(Some(email), None)),
            Err(error) => return Err(error),
        };
        self.activate(issued).await
    }

    pub async fn select_company(&self, company_id: Uuid) -> Result<AuthView, AuthError> {
        let Some(stored) = self.store.load()? else {
            return Ok(AuthView::signed_out(None));
        };
        let issued = self.refresh_if_needed(stored).await?;
        let memberships = self
            .platform
            .list(issued.account_id, &issued.access_token, Some(company_id))
            .await?;
        let membership = memberships.into_iter().next().ok_or(AuthError::Denied)?;
        let session = to_public(&issued, &membership);
        self.persist_and_unlock(&issued, Some(membership.company_id), Some(session.clone()))?;
        Ok(AuthView::authenticated(session))
    }

    pub fn lock(&self) -> Result<AuthView, AuthError> {
        let stored = self.store.load()?;
        *self.unlocked.lock().expect("unlock flag") = false;
        *self.last_session.lock().expect("last session") = None;
        Ok(AuthView::locked(stored.map(|session| session.email)))
    }

    pub async fn sign_out(&self) -> Result<AuthView, AuthError> {
        if let Some(stored) = self.store.load()? {
            let _ = self.issuer.sign_out(&stored.access_token).await;
        }
        self.store.clear()?;
        *self.unlocked.lock().expect("unlock flag") = false;
        *self.last_session.lock().expect("last session") = None;
        Ok(AuthView::signed_out(Some(
            "Sessão encerrada neste dispositivo.".into(),
        )))
    }

    pub fn current(&self) -> Result<AuthView, AuthError> {
        if let Some(session) = self.last_session.lock().expect("last session").clone() {
            return Ok(AuthView::authenticated(session));
        }
        let email = self.store.load()?.map(|session| session.email);
        if email.is_some() {
            return Ok(AuthView::locked(email));
        }
        Ok(AuthView::signed_out(None))
    }

    async fn activate(&self, issued: IssuedAuth) -> Result<AuthView, AuthError> {
        let memberships = match self
            .platform
            .list(issued.account_id, &issued.access_token, None)
            .await
        {
            Ok(rows) => rows,
            Err(AuthError::Denied) => {
                self.store.clear()?;
                return Err(AuthError::Denied);
            }
            Err(error) => return Err(error),
        };
        if memberships.is_empty() {
            self.store.clear()?;
            return Err(AuthError::Denied);
        }
        if memberships.len() == 1 {
            let membership = memberships.into_iter().next().expect("one membership");
            let session = to_public(&issued, &membership);
            self.persist_and_unlock(&issued, Some(membership.company_id), Some(session.clone()))?;
            return Ok(AuthView::authenticated(session));
        }
        let preferred = self.store.load()?.and_then(|session| session.company_id);
        if let Some(company_id) = preferred {
            if let Some(membership) = memberships
                .iter()
                .find(|membership| membership.company_id == company_id)
            {
                let session = to_public(&issued, membership);
                self.persist_and_unlock(&issued, Some(membership.company_id), Some(session.clone()))?;
                return Ok(AuthView::authenticated(session));
            }
        }
        self.persist_and_unlock(&issued, None, None)?;
        Ok(AuthView::needs_company(
            issued.email,
            memberships
                .into_iter()
                .map(|membership| CompanyOption {
                    company_id: membership.company_id,
                    label: membership.label,
                    role: membership.role,
                })
                .collect(),
        ))
    }

    async fn refresh_if_needed(&self, stored: StoredSession) -> Result<IssuedAuth, AuthError> {
        let now = now_secs();
        if stored.expires_at > now + REFRESH_MARGIN_SECS {
            return Ok(IssuedAuth {
                account_id: stored.account_id,
                email: stored.email,
                access_token: stored.access_token,
                refresh_token: stored.refresh_token,
                expires_at: stored.expires_at,
            });
        }
        match self.issuer.refresh(&stored.refresh_token).await {
            Ok(issued) => Ok(issued),
            Err(AuthError::Network) => Err(AuthError::Network),
            Err(_) => {
                self.store.clear()?;
                Err(AuthError::Expired)
            }
        }
    }

    fn persist_and_unlock(
        &self,
        issued: &IssuedAuth,
        company_id: Option<Uuid>,
        session: Option<PublicSession>,
    ) -> Result<(), AuthError> {
        self.store.save(&StoredSession {
            access_token: issued.access_token.clone(),
            refresh_token: issued.refresh_token.clone(),
            expires_at: issued.expires_at,
            account_id: issued.account_id,
            email: issued.email.clone(),
            company_id,
        })?;
        *self.unlocked.lock().expect("unlock flag") = session.is_some();
        *self.last_session.lock().expect("last session") = session;
        Ok(())
    }
}

fn to_public(issued: &IssuedAuth, membership: &Membership) -> PublicSession {
    PublicSession {
        account_id: issued.account_id,
        company_id: membership.company_id,
        email: issued.email.clone(),
        role: membership.role,
        permissions: membership.role.permissions(),
        expires_at: issued.expires_at,
    }
}

fn required_env(key: &str) -> Result<String, AuthError> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AuthError::Configuration(format!("{key} não foi configurada.")))
}

fn http_client() -> Result<Client, AuthError> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|_| AuthError::Network)
}

fn validate_https_url(value: &str, field: &str) -> Result<(), AuthError> {
    let parsed = reqwest::Url::parse(value).map_err(|_| {
        AuthError::Configuration(format!("{field} deve ser uma URL HTTPS válida."))
    })?;
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AuthError::Configuration(format!(
            "{field} deve ser uma URL HTTPS válida."
        )));
    }
    Ok(())
}

fn validate_publishable_key(key: &str) -> Result<(), AuthError> {
    if key.trim().is_empty() {
        return Err(AuthError::Configuration(
            "SUPABASE_PUBLISHABLE_KEY não foi configurada.".into(),
        ));
    }
    if jwt_role(key).as_deref() == Some("service_role") {
        return Err(AuthError::Configuration(
            "A aplicação aceita somente uma chave publicável.".into(),
        ));
    }
    Ok(())
}

fn jwt_role(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let normalized = payload.replace('-', "+").replace('_', "/");
    let padded = format!("{normalized}{}", "=".repeat((4 - normalized.len() % 4) % 4));
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, padded).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    value
        .get("role")
        .and_then(|role| role.as_str())
        .map(str::to_string)
}

fn map_reqwest(error: reqwest::Error) -> AuthError {
    if error.is_timeout() || error.is_connect() {
        AuthError::Network
    } else {
        AuthError::Unavailable("Não foi possível concluir a autenticação agora.".into())
    }
}

fn parse_supabase_session(status: StatusCode, body: String) -> Result<IssuedAuth, AuthError> {
    if status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED {
        return Err(AuthError::InvalidCredentials);
    }
    if !status.is_success() {
        return Err(AuthError::Unavailable(
            "Esta conta não está disponível para acesso ao AutoBO.".into(),
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| AuthError::Unavailable("Sessão inválida.".into()))?;
    let access_token = value
        .get("access_token")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .ok_or(AuthError::Unavailable("Sessão incompleta.".into()))?;
    if jwt_role(access_token).as_deref() == Some("service_role") {
        return Err(AuthError::Denied);
    }
    let refresh_token = value
        .get("refresh_token")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .ok_or(AuthError::Unavailable("Sessão incompleta.".into()))?;
    let user = value.get("user").cloned().unwrap_or(serde_json::Value::Null);
    let account_id = user
        .get("id")
        .and_then(|item| item.as_str())
        .and_then(|item| Uuid::parse_str(item).ok())
        .ok_or(AuthError::Unavailable("Identidade inválida.".into()))?;
    let email = user
        .get("email")
        .and_then(|item| item.as_str())
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| item.contains('@'))
        .ok_or(AuthError::Unavailable("Email inválido na sessão.".into()))?;
    let expires_at = value
        .get("expires_at")
        .and_then(|item| item.as_i64())
        .or_else(|| {
            value
                .get("expires_in")
                .and_then(|item| item.as_i64())
                .map(|seconds| now_secs() + seconds)
        })
        .ok_or(AuthError::Unavailable("Expiração inválida.".into()))?;
    Ok(IssuedAuth {
        account_id,
        email,
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        expires_at,
    })
}

fn parse_platform_memberships(
    status: StatusCode,
    body: &str,
    required_company: Option<Uuid>,
) -> Result<(Uuid, Vec<Membership>), AuthError> {
    if status == StatusCode::UNAUTHORIZED {
        return Err(AuthError::Expired);
    }
    if status == StatusCode::FORBIDDEN {
        return Err(AuthError::Denied);
    }
    if !status.is_success() {
        return Err(AuthError::Unavailable(
            "Não foi possível consultar as empresas autorizadas.".into(),
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| AuthError::Denied)?;
    let account_id = value
        .get("account_id")
        .and_then(|item| item.as_str())
        .and_then(|item| Uuid::parse_str(item).ok())
        .ok_or(AuthError::Denied)?;
    let mut memberships = Vec::new();
    for item in value
        .get("memberships")
        .and_then(|item| item.as_array())
        .cloned()
        .unwrap_or_default()
    {
        if item.get("product").and_then(|item| item.as_str()) != Some(PRODUCT) {
            continue;
        }
        if item.get("status").and_then(|item| item.as_str()) != Some("active") {
            continue;
        }
        let capabilities = item
            .get("capabilities")
            .and_then(|item| item.as_array())
            .cloned()
            .unwrap_or_default();
        if !capabilities.iter().any(|item| item.as_str() == Some(CAPABILITY)) {
            continue;
        }
        let company_id = item
            .get("company_id")
            .and_then(|item| item.as_str())
            .and_then(|item| Uuid::parse_str(item).ok())
            .ok_or(AuthError::Denied)?;
        if required_company.map(|id| id != company_id).unwrap_or(false) {
            continue;
        }
        let role = item
            .get("role")
            .and_then(|item| item.as_str())
            .and_then(Role::parse)
            .ok_or(AuthError::Denied)?;
        let label = item
            .get("label")
            .and_then(|item| item.as_str())
            .unwrap_or("Empresa")
            .to_string();
        memberships.push(Membership {
            company_id,
            label,
            role,
        });
    }
    if required_company.is_some() && memberships.is_empty() {
        return Err(AuthError::Denied);
    }
    Ok((account_id, memberships))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn account_a() -> Uuid {
    Uuid::from_u128(0xa1)
}
fn account_b() -> Uuid {
    Uuid::from_u128(0xb2)
}
fn account_c() -> Uuid {
    Uuid::from_u128(0xc3)
}
fn company_a() -> Uuid {
    Uuid::from_u128(0xa10)
}
fn company_b() -> Uuid {
    Uuid::from_u128(0xb10)
}

fn demo_issuer() -> FakeIssuer {
    FakeIssuer::default()
        .with_user("admin.a@example.test", "senha-a", account_a())
        .with_user("fiscal.b@example.test", "senha-b", account_b())
        .with_user("sem.acesso@example.test", "senha-c", account_c())
}

fn demo_platform() -> FakePlatform {
    FakePlatform::default()
        .with_row(FakeMembership {
            account_id: account_a(),
            company_id: company_a(),
            label: "Oficina A".into(),
            product: "autoos".into(),
            role: Role::Admin,
            status: "active".into(),
            capabilities: vec![],
        })
        .with_row(FakeMembership {
            account_id: account_a(),
            company_id: company_a(),
            label: "Oficina A".into(),
            product: PRODUCT.into(),
            role: Role::Admin,
            status: "active".into(),
            capabilities: vec![CAPABILITY.into()],
        })
        .with_row(FakeMembership {
            account_id: account_b(),
            company_id: company_b(),
            label: "Oficina B".into(),
            product: PRODUCT.into(),
            role: Role::Fiscal,
            status: "active".into(),
            capabilities: vec![CAPABILITY.into()],
        })
        .with_row(FakeMembership {
            account_id: account_c(),
            company_id: company_a(),
            label: "Oficina A".into(),
            product: PRODUCT.into(),
            role: Role::Admin,
            status: "active".into(),
            capabilities: vec![],
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> AuthRuntime {
        AuthRuntime::harness(
            demo_issuer(),
            demo_platform(),
            Store::Memory(Arc::new(Mutex::new(None))),
        )
    }

    #[tokio::test]
    async fn login_with_autobo_access_ignores_autoos_admin() {
        let auth = runtime();
        let view = auth.login("Admin.A@example.test", "senha-a").await.unwrap();
        let session = view.session.expect("session");
        assert_eq!(session.company_id, company_a());
        assert_eq!(session.role, Role::Admin);
        assert!(session.permissions.contains(&Permission::ManageCompany));
        assert!(!session.permissions.contains(&Permission::Fiscal));
        assert_eq!(view.kind, "authenticated");
    }

    #[tokio::test]
    async fn fiscal_on_autobo_does_not_manage_company() {
        let auth = runtime();
        let session = auth
            .login("fiscal.b@example.test", "senha-b")
            .await
            .unwrap()
            .session
            .unwrap();
        assert_eq!(session.company_id, company_b());
        assert_eq!(session.role, Role::Fiscal);
        assert_eq!(session.permissions, vec![Permission::Fiscal]);
    }

    #[tokio::test]
    async fn missing_autobo_access_is_denied() {
        let auth = runtime();
        let error = auth
            .login("sem.acesso@example.test", "senha-c")
            .await
            .unwrap_err();
        assert_eq!(error, AuthError::Denied);
        assert!(auth.store.load().unwrap().is_none());
    }

    #[tokio::test]
    async fn tenant_cannot_select_the_other_company() {
        let auth = runtime();
        auth.login("admin.a@example.test", "senha-a").await.unwrap();
        let error = auth.select_company(company_b()).await.unwrap_err();
        assert_eq!(error, AuthError::Denied);
    }

    #[tokio::test]
    async fn lock_keeps_keyring_and_logout_clears_it() {
        let auth = runtime();
        auth.login("admin.a@example.test", "senha-a").await.unwrap();
        let locked = auth.lock().unwrap();
        assert_eq!(locked.kind, "locked");
        assert!(auth.store.load().unwrap().is_some());
        let restored = auth.restore().await.unwrap();
        assert_eq!(restored.kind, "authenticated");
        auth.sign_out().await.unwrap();
        assert!(auth.store.load().unwrap().is_none());
    }

    #[tokio::test]
    async fn expired_refresh_clears_the_store() {
        let auth = AuthRuntime::harness(
            FakeIssuer::default(),
            demo_platform(),
            Store::Memory(Arc::new(Mutex::new(Some(StoredSession {
                access_token: "old".into(),
                refresh_token: "missing".into(),
                expires_at: 1,
                account_id: account_a(),
                email: "admin.a@example.test".into(),
                company_id: Some(company_a()),
            })))),
        );
        let error = auth.restore().await.unwrap();
        assert_eq!(error.kind, "expired");
        assert!(auth.store.load().unwrap().is_none());
    }

    #[test]
    fn https_and_privileged_keys_are_rejected() {
        assert!(validate_https_url("http://project.supabase.co", "SUPABASE_URL").is_err());
        let privileged = encoding_jwt(&serde_json::json!({"role": "service_role"}));
        assert!(validate_publishable_key(&privileged).is_err());
    }

    #[test]
    fn supabase_parser_ignores_company_claim_in_metadata() {
        let access = encoding_jwt(&serde_json::json!({"role": "authenticated", "sub": account_a()}));
        let body = serde_json::json!({
            "access_token": access,
            "refresh_token": "refresh",
            "expires_at": 1_900_000_000,
            "user": {
                "id": account_a(),
                "email": "Admin.A@example.test",
                "app_metadata": { "company_id": company_b(), "profile_role": "ADMIN" }
            }
        })
        .to_string();
        let issued = parse_supabase_session(StatusCode::OK, body).unwrap();
        assert_eq!(issued.account_id, account_a());
        assert_eq!(issued.email, "admin.a@example.test");
    }

    #[test]
    fn platform_parser_drops_autoos_and_missing_capability() {
        let body = serde_json::json!({
            "account_id": account_a(),
            "memberships": [
                {
                    "company_id": company_a(),
                    "label": "A",
                    "product": "autoos",
                    "role": "admin",
                    "status": "active",
                    "capabilities": [CAPABILITY]
                },
                {
                    "company_id": company_b(),
                    "label": "B",
                    "product": "autobo",
                    "role": "fiscal",
                    "status": "active",
                    "capabilities": [CAPABILITY]
                }
            ]
        })
        .to_string();
        let (account, memberships) =
            parse_platform_memberships(StatusCode::OK, &body, None).unwrap();
        assert_eq!(account, account_a());
        assert_eq!(memberships.len(), 1);
        assert_eq!(memberships[0].company_id, company_b());
        assert_eq!(memberships[0].role, Role::Fiscal);
    }

    fn encoding_jwt(payload: &serde_json::Value) -> String {
        let json = payload.to_string();
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            json.as_bytes(),
        );
        format!("header.{encoded}.sig")
    }
}
