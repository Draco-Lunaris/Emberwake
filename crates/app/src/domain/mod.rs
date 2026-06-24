//! Shared domain types for the Emberwake application.
//! These types are compiled for both SSR and WASM (hydrate) targets.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Visibility level for content items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Restricted,
}
impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Private => write!(f, "private"),
            Self::Restricted => write!(f, "restricted"),
        }
    }
}

impl std::str::FromStr for Visibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            "restricted" => Ok(Self::Restricted),
            other => Err(format!("invalid visibility: {other}")),
        }
    }
}

/// A category grouping services and/or bookmarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub order_index: i64,
    pub visibility: Visibility,
    pub created_at: String,
    pub updated_at: String,
}

/// A category with item counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryWithItems {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub order_index: i64,
    pub visibility: Visibility,
    pub service_count: i64,
    pub bookmark_count: i64,
}

/// A launchable and optionally monitored service tile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub is_pinned: bool,
    pub order_index: i64,
    pub visibility: Visibility,
    pub monitor_enabled: bool,
    pub monitor_kind: Option<String>,
    pub monitor_target: Option<String>,
    pub monitor_interval_s: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// A launchable tile (no monitoring).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub is_pinned: bool,
    pub order_index: i64,
    pub visibility: Visibility,
    pub created_at: String,
    pub updated_at: String,
}

/// A bookmark link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub order_index: i64,
    pub visibility: Visibility,
    pub created_at: String,
    pub updated_at: String,
}

/// A category with its bookmarks (for dashboard view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryWithBookmarks {
    pub category: Category,
    pub bookmarks: Vec<Bookmark>,
}

/// Dashboard view: pinned services + pinned categories with bookmarks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardView {
    pub pinned_services: Vec<Service>,
    pub pinned_categories: Vec<CategoryWithBookmarks>,
    #[serde(default)]
    pub applications: Vec<Application>,
}

/// Filter for service queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceFilter {
    pub category_id: Option<Uuid>,
}

/// Search provider configuration for prefix routing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchProviderConfig {
    pub providers: Vec<SearchProvider>,
    pub default_provider: Option<String>,
}

/// A single search provider with prefix routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProvider {
    pub prefix: String,
    pub name: String,
    pub url_template: String,
}

/// Visibility filter for SQL queries.
/// Determines which rows are returned based on caller authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityFilter {
    /// Only return public rows (for anonymous/unauthorized callers).
    PublicOnly,
    /// Return public + private rows, excluding restricted (for authenticated non-admin callers).
    All,
    /// Return all rows including restricted (for admin callers).
    AllIncludingRestricted,
}

impl VisibilityFilter {
    /// Get the SQL WHERE clause fragment for this filter.
    pub fn where_clause(&self) -> &'static str {
        match self {
            Self::PublicOnly => "visibility = 'public'",
            Self::All => "visibility IN ('public', 'private')",
            Self::AllIncludingRestricted => "1=1",
        }
    }
}

// --- Input/Patch types for content CRUD ---

/// Input for creating a category.
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct CategoryInput {
    #[garde(length(min = 1))]
    pub name: String,
    #[garde(skip)]
    pub icon: Option<String>,
    #[garde(skip)]
    pub visibility: Visibility,
}

/// Patch for updating a category (all fields optional; None = no change).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryPatch {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Input for creating a service.
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct ServiceInput {
    #[garde(skip)]
    pub category_id: Option<Uuid>,
    #[garde(length(min = 1))]
    pub name: String,
    #[garde(url)]
    pub url: String,
    #[garde(skip)]
    pub icon: Option<String>,
    #[garde(skip)]
    pub description: Option<String>,
    #[garde(skip)]
    pub is_pinned: bool,
    #[garde(skip)]
    pub visibility: Visibility,
    #[garde(skip)]
    pub monitor_enabled: bool,
    #[garde(skip)]
    pub monitor_kind: Option<String>,
    #[garde(skip)]
    pub monitor_target: Option<String>,
    #[garde(skip)]
    pub monitor_interval_s: Option<i64>,
}

/// Patch for updating a service (all fields optional; None = no change).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServicePatch {
    pub category_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub description: Option<Option<String>>,
    pub is_pinned: Option<bool>,
    pub visibility: Option<Visibility>,
    pub monitor_enabled: Option<bool>,
    pub monitor_kind: Option<String>,
    pub monitor_target: Option<String>,
    pub monitor_interval_s: Option<Option<i64>>,
}

/// Input for creating an application.
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct ApplicationInput {
    #[garde(skip)]
    pub category_id: Option<Uuid>,
    #[garde(length(min = 1))]
    pub name: String,
    #[garde(url)]
    pub url: String,
    #[garde(skip)]
    pub icon: Option<String>,
    #[garde(skip)]
    pub description: Option<String>,
    #[garde(skip)]
    pub is_pinned: bool,
    #[garde(skip)]
    pub visibility: Visibility,
}

/// Patch for updating an application (all fields optional; None = no change).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApplicationPatch {
    pub category_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub description: Option<Option<String>>,
    pub is_pinned: Option<bool>,
    pub visibility: Option<Visibility>,
}

/// Input for creating a bookmark.
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct BookmarkInput {
    #[garde(skip)]
    pub category_id: Option<Uuid>,
    #[garde(length(min = 1))]
    pub name: String,
    #[garde(url)]
    pub url: String,
    #[garde(skip)]
    pub icon: Option<String>,
    #[garde(skip)]
    pub visibility: Visibility,
}

/// Patch for updating a bookmark (all fields optional; None = no change).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookmarkPatch {
    pub category_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Reference to an uploaded icon file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconRef {
    pub path: String,
}

// --- Auth domain types ---

/// User role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    User,
    Admin,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::User => write!(f, "user"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            other => Err(format!("invalid role: {other}")),
        }
    }
}

/// First-run setup state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SetupState {
    Open,
    Complete,
}

/// Input for first-run admin setup.
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct AdminSetupInput {
    #[garde(length(min = 1))]
    pub username: String,
    #[garde(length(min = 1))]
    pub password: String,
    #[garde(skip)]
    pub email: Option<String>,
}

/// Input for login.
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct LoginInput {
    #[garde(length(min = 1))]
    pub username: String,
    #[garde(length(min = 1))]
    pub password: String,
}

/// Summary of a session (for session list UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub last_used_at: String,
}

/// Summary of a user (for user management UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub role: Role,
    pub is_active: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

/// Input for creating a new user (admin action).
#[derive(Debug, Clone, Serialize, Deserialize, garde::Validate)]
pub struct NewUserInput {
    #[garde(length(min = 1))]
    pub username: String,
    #[garde(length(min = 1))]
    pub password: String,
    #[garde(skip)]
    pub email: Option<String>,
    #[garde(skip)]
    pub role: Role,
}

/// Patch for updating a user (admin action).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPatch {
    pub username: Option<String>,
    pub email: Option<Option<String>>,
    pub role: Option<Role>,
    pub is_active: Option<bool>,
    pub password: Option<String>,
}

// --- Extended auth domain types (US4) ---

/// An external identity linked to a user (OIDC provider subject).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub subject: String,
    pub created_at: String,
}

/// Summary of an API token (no secret — hash only is stored).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenSummary {
    pub id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Input for creating an API token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenInput {
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}

/// The secret returned once when an API token is created.
/// The secret is never stored — only its hash. Shown once to the operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenSecret {
    pub id: Uuid,
    pub secret: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}

/// A redirect URL for OIDC login begin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectUrl {
    pub url: String,
}

/// Summary of a registered passkey (for account UI listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeySummary {
    pub id: String,
    pub created_at: String,
}

/// WebAuthn credential creation options (JSON sent to navigator.credentials.create).
/// Uses serde_json::Value to avoid adding webauthn-rs-proto to the app crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialCreationOptions {
    pub challenge: serde_json::Value,
}

/// WebAuthn request options (JSON sent to navigator.credentials.get).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOptions {
    pub challenge: serde_json::Value,
}

/// WebAuthn registration response (from navigator.credentials.create).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub credential: serde_json::Value,
}

/// WebAuthn authentication response (from navigator.credentials.get).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub assertion: serde_json::Value,
}

/// Valid API token scopes.
pub const VALID_SCOPES: &[&str] = &[
    "services:read",
    "services:write",
    "bookmarks:read",
    "export",
];

// --- Monitor domain types (US6) ---

/// Health state of a monitored service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MonitorState {
    Up,
    Down,
    Degraded,
}

impl std::fmt::Display for MonitorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => write!(f, "up"),
            Self::Down => write!(f, "down"),
            Self::Degraded => write!(f, "degraded"),
        }
    }
}

impl std::str::FromStr for MonitorState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "degraded" => Ok(Self::Degraded),
            other => Err(format!("invalid monitor state: {other}")),
        }
    }
}

/// Latest health reading for a monitored service (one current row per service).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReading {
    pub service_id: Uuid,
    pub state: MonitorState,
    pub latency_ms: Option<i64>,
    pub reason: Option<String>,
    pub checked_at: String,
}

/// A bounded history entry of service health for uptime tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusHistory {
    pub id: Uuid,
    pub service_id: Uuid,
    pub state: MonitorState,
    pub latency_ms: Option<i64>,
    pub reason: Option<String>,
    pub checked_at: String,
}

/// Uptime summary computed from StatusHistory over a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeSummary {
    pub service_id: Uuid,
    pub window_hours: u32,
    pub total_checks: u64,
    pub up_checks: u64,
    pub uptime_percent: f64,
}

/// A status event pushed via SSE to connected clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseStatusEvent {
    pub service_id: Uuid,
    pub state: MonitorState,
    pub latency_ms: Option<i64>,
    pub visibility: Visibility,
}

// --- Weather widget domain types (US7) ---

/// Latest weather reading (single-row cache).
/// Fields: temp (°C), condition code, is-day, cloud %, upstream timestamp, fetched_at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherReading {
    pub temp: Option<f64>,
    pub condition: Option<String>,
    pub is_day: Option<bool>,
    pub cloud: Option<i64>,
    pub upstream_ts: Option<String>,
    pub fetched_at: String,
}

// --- Settings & theme domain types (US5) ---

/// Design tokens — map to CSS custom properties injected into <head>.
/// All fields are Option so a theme can override only what it needs;
/// the CSS defaults in style/main.css handle the rest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesignTokens {
    // ── Colour palette ──────────────────────────────
    /// Page background. Dark default: #0d0d0f
    pub bg: Option<String>,
    /// Deeper/inset background areas. Dark default: #080809
    pub bg_deep: Option<String>,
    /// Card / component surface. Dark default: #141418
    pub surface: Option<String>,
    /// Elevated surface (modals, dropdowns). Dark default: #1e1e26
    pub surface_raised: Option<String>,
    /// Primary text. Dark default: #eeeef0
    pub text: Option<String>,
    /// Secondary / label text. Dark default: #8888a0
    pub text_muted: Option<String>,
    /// Placeholder / disabled text. Dark default: #484860
    pub text_faint: Option<String>,
    /// Accent colour (ember orange). Dark default: #f97316
    pub accent: Option<String>,
    /// Text on accent backgrounds. Default: #ffffff
    pub accent_text: Option<String>,
    /// Cool accent counterpoint (used sparingly). Default: #6366f1
    pub accent_alt: Option<String>,
    /// Subtle border / divider. Dark default: rgba(255,255,255,0.07)
    pub border: Option<String>,

    // ── Shape ───────────────────────────────────────
    /// Base border radius. Default: 10px
    pub radius: Option<String>,
    /// Small border radius. Default: 6px
    pub radius_sm: Option<String>,
    /// Large border radius (cards/modals). Default: 16px
    pub radius_lg: Option<String>,

    // ── Spacing & type ──────────────────────────────
    /// Base spacing unit. Default: 16px
    pub spacing: Option<String>,
    /// Body font stack. Default: 'Inter', system-ui, sans-serif
    pub font: Option<String>,
    /// Monospace font stack (URLs, tokens). Default: 'JetBrains Mono', monospace
    pub font_mono: Option<String>,

    // ── Compatibility alias ─────────────────────────
    /// Light/dark mode hint stored with theme. Values: "dark" | "light"
    pub mode: Option<String>,
}

/// A complete theme with design tokens and optional custom CSS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: Uuid,
    pub name: String,
    pub tokens: DesignTokens,
    pub custom_css: Option<String>,
    pub is_builtin: bool,
    pub created_by: Option<Uuid>,
    pub created_at: String,
}

/// Summary of a theme (for listing without full tokens/CSS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSummary {
    pub id: Uuid,
    pub name: String,
    pub is_builtin: bool,
    pub created_at: String,
}

/// Input for creating or updating a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeInput {
    pub name: String,
    pub tokens: DesignTokens,
    pub custom_css: Option<String>,
}

/// View of all settings (secrets redacted for non-admins).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsView {
    pub search_providers: SearchProviderConfig,
    pub integrations: IntegrationSettings,
    pub weather: WeatherSettings,
    pub auth: AuthSettings,
    pub theme_active: Option<String>,
}

/// Integration toggle settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationSettings {
    pub docker_enabled: bool,
    pub docker_socket: Option<String>,
    pub k8s_enabled: bool,
}

/// Weather widget settings (API key is secret-bearing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeatherSettings {
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub location: Option<String>,
    pub refresh_interval_s: Option<i64>,
    pub enabled: bool,
}

/// Auth-related settings (OIDC client secret is secret-bearing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthSettings {
    pub oidc_enabled: bool,
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub passkeys_enabled: bool,
}

/// Patch for updating settings (all fields optional; None = no change).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsPatch {
    pub search_providers: Option<SearchProviderConfig>,
    pub integrations: Option<IntegrationSettings>,
    pub weather: Option<WeatherSettings>,
    pub auth: Option<AuthSettings>,
    pub theme_active: Option<Option<String>>,
}

// --- Discovery domain types (US8) ---

/// Source of a discovered service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoverySource {
    Docker,
    Kubernetes,
}

impl std::fmt::Display for DiscoverySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docker => write!(f, "docker"),
            Self::Kubernetes => write!(f, "kubernetes"),
        }
    }
}

/// A service discovered from Docker container labels or K8s Ingress annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub source: DiscoverySource,
    /// Docker container ID or K8s ingress namespace/name — used for SSE event identity.
    pub source_id: String,
}

/// Action for a discovery SSE event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoveryAction {
    Added,
    Removed,
}

/// A discovery event pushed via SSE to connected clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseDiscoveryEvent {
    pub service_id: String,
    pub action: DiscoveryAction,
    pub name: String,
    pub url: String,
}

// --- Import/Export domain types (US9) ---

/// Scope for data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportScope {
    Full,
    Selective(Vec<ExportEntity>),
}

/// Entity types that can be included in a selective export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportEntity {
    Categories,
    Services,
    Bookmarks,
    Settings,
    Themes,
}

/// A complete export document containing all entity types (or selected subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDocument {
    pub version: String,
    pub exported_at: String,
    #[serde(default)]
    pub categories: Vec<ExportCategory>,
    #[serde(default)]
    pub services: Vec<ExportService>,
    #[serde(default)]
    pub bookmarks: Vec<ExportBookmark>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
    #[serde(default)]
    pub themes: Vec<ExportTheme>,
}

/// Export DTO for a category (no id — ids are regenerated on import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCategory {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub order_index: i64,
    pub visibility: Visibility,
}

/// Export DTO for a service (no id — ids are regenerated on import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportService {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_pinned: bool,
    pub order_index: i64,
    pub visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
    pub monitor_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_interval_s: Option<i64>,
}

/// Export DTO for a bookmark (no id — ids are regenerated on import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBookmark {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub order_index: i64,
    pub visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
}

/// Export DTO for a theme (no id — ids are regenerated on import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTheme {
    pub name: String,
    pub tokens: DesignTokens,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
    pub is_builtin: bool,
}

/// Kind of import file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportKind {
    Json,
    HtmlBookmarks,
    Opml,
}

/// Strategy for handling duplicates during import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplicateStrategy {
    #[default]
    Skip,
    Overwrite,
    Rename,
}

/// Options for applying an import.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    pub duplicate_strategy: DuplicateStrategy,
}

/// Result of an import apply operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportResult {
    pub categories_created: usize,
    pub categories_updated: usize,
    pub categories_skipped: usize,
    pub bookmarks_created: usize,
    pub bookmarks_updated: usize,
    pub bookmarks_skipped: usize,
    pub services_created: usize,
    pub services_updated: usize,
    pub services_skipped: usize,
    pub themes_created: usize,
    pub themes_skipped: usize,
}

/// Parsed data ready for preview and apply (internal to server-side parsing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedData {
    #[serde(default)]
    pub categories: Vec<ParsedCategory>,
    #[serde(default)]
    pub bookmarks: Vec<ParsedBookmark>,
    #[serde(default)]
    pub services: Vec<ParsedService>,
    #[serde(default)]
    pub themes: Vec<ParsedTheme>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
}

/// A category parsed from an import file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCategory {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub visibility: Visibility,
}

/// A bookmark parsed from an import file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBookmark {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
    #[serde(default)]
    pub visibility: Visibility,
}

/// A service parsed from an import file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedService {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(default)]
    pub monitor_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_interval_s: Option<i64>,
}

/// A theme parsed from an import file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTheme {
    pub name: String,
    pub tokens: DesignTokens,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
}

/// Preview of an import (shown to user before applying).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewData {
    pub token: String,
    pub category_count: usize,
    pub bookmark_count: usize,
    pub service_count: usize,
    pub theme_count: usize,
    pub has_settings: bool,
    pub sample_categories: Vec<String>,
    pub sample_bookmarks: Vec<String>,
    pub sample_services: Vec<String>,
}
