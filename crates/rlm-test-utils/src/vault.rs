//! Secure key vault integration for RLM testing.
//!
//! This module provides a unified interface for accessing secrets from various
//! key vault providers, including local file-based storage for development
//! and production-grade solutions like HashiCorp Vault, AWS Secrets Manager,
//! and Azure Key Vault.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{TestResult, VaultError};

/// Configuration for vault providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// The vault provider type.
    pub provider: VaultProviderType,
    /// Provider-specific configuration.
    pub config: serde_json::Value,
}

/// Supported vault provider types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VaultProviderType {
    /// Local file-based secrets (development only).
    Local,
    /// HashiCorp Vault.
    HashiCorp,
    /// AWS Secrets Manager.
    AwsSecrets,
    /// Azure Key Vault.
    AzureKeyVault,
    /// Supabase Vault (both self-hosted and cloud).
    SupabaseVault,
}

/// Trait for key vault providers.
#[async_trait]
pub trait KeyVaultProvider: Send + Sync {
    /// Get a secret by key.
    async fn get_secret(&self, key: &str) -> Result<String, VaultError>;

    /// Get multiple secrets at once.
    async fn get_secrets(&self, keys: &[&str]) -> Result<HashMap<String, String>, VaultError> {
        let mut results = HashMap::new();
        for key in keys {
            match self.get_secret(key).await {
                Ok(value) => { results.insert(key.to_string(), value); }
                Err(e) => {
                    warn!("Failed to get secret '{}': {}", key, e);
                    return Err(e);
                }
            }
        }
        Ok(results)
    }

    /// Check if the vault is healthy and accessible.
    async fn health_check(&self) -> Result<(), VaultError>;

    /// Get vault metadata (provider type, endpoint, etc).
    fn metadata(&self) -> VaultMetadata;
}

/// Metadata about a vault provider.
#[derive(Debug, Clone)]
pub struct VaultMetadata {
    /// The provider type.
    pub provider_type: VaultProviderType,
    /// The endpoint URL (if applicable).
    pub endpoint: Option<String>,
    /// Additional metadata.
    pub extra: HashMap<String, String>,
}

/// Factory for creating vault providers from environment.
pub struct VaultFactory;

impl VaultFactory {
    /// Create a vault provider from environment variables.
    pub async fn from_env() -> TestResult<Box<dyn KeyVaultProvider>> {
        let provider_type = env::var("RLM_VAULT_PROVIDER")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase();

        match provider_type.as_str() {
            "local" => Ok(Box::new(LocalVaultProvider::from_env().await?)),
            "hashicorp" | "vault" => {
                #[cfg(feature = "hashicorp-vault")]
                {
                    Ok(Box::new(HashiCorpVaultProvider::from_env().await?))
                }
                #[cfg(not(feature = "hashicorp-vault"))]
                {
                    Err(VaultError::invalid_config(
                        "HashiCorp Vault support not enabled. Enable 'hashicorp-vault' feature"
                    ).into())
                }
            }
            "aws" | "aws-secrets" => {
                #[cfg(feature = "aws-secrets")]
                {
                    Ok(Box::new(AwsSecretsProvider::from_env().await?))
                }
                #[cfg(not(feature = "aws-secrets"))]
                {
                    Err(VaultError::invalid_config(
                        "AWS Secrets Manager support not enabled. Enable 'aws-secrets' feature"
                    ).into())
                }
            }
            "azure" | "azure-keyvault" => {
                #[cfg(feature = "azure-keyvault")]
                {
                    Ok(Box::new(AzureKeyVaultProvider::from_env().await?))
                }
                #[cfg(not(feature = "azure-keyvault"))]
                {
                    Err(VaultError::invalid_config(
                        "Azure Key Vault support not enabled. Enable 'azure-keyvault' feature"
                    ).into())
                }
            }
            "supabase" | "supabase-vault" => {
                #[cfg(feature = "supabase-vault")]
                {
                    Ok(Box::new(SupabaseVaultProvider::from_env().await?))
                }
                #[cfg(not(feature = "supabase-vault"))]
                {
                    Err(VaultError::invalid_config(
                        "Supabase Vault support not enabled. Enable 'supabase-vault' feature"
                    ).into())
                }
            }
            _ => Err(VaultError::invalid_config(format!(
                "Unsupported vault provider: {}. Use 'local', 'hashicorp', 'aws', 'azure', or 'supabase'",
                provider_type
            )).into()),
        }
    }

    /// Create a vault provider from explicit configuration.
    pub async fn from_config(config: VaultConfig) -> TestResult<Box<dyn KeyVaultProvider>> {
        match config.provider {
            VaultProviderType::Local => Ok(Box::new(LocalVaultProvider::from_config(config.config).await?)),
            VaultProviderType::HashiCorp => {
                #[cfg(feature = "hashicorp-vault")]
                {
                    Ok(Box::new(HashiCorpVaultProvider::from_config(config.config).await?))
                }
                #[cfg(not(feature = "hashicorp-vault"))]
                {
                    Err(VaultError::invalid_config("HashiCorp Vault support not enabled").into())
                }
            }
            VaultProviderType::AwsSecrets => {
                #[cfg(feature = "aws-secrets")]
                {
                    Ok(Box::new(AwsSecretsProvider::from_config(config.config).await?))
                }
                #[cfg(not(feature = "aws-secrets"))]
                {
                    Err(VaultError::invalid_config("AWS Secrets Manager support not enabled").into())
                }
            }
            VaultProviderType::AzureKeyVault => {
                #[cfg(feature = "azure-keyvault")]
                {
                    Ok(Box::new(AzureKeyVaultProvider::from_config(config.config).await?))
                }
                #[cfg(not(feature = "azure-keyvault"))]
                {
                    Err(VaultError::invalid_config("Azure Key Vault support not enabled").into())
                }
            }
            VaultProviderType::SupabaseVault => {
                #[cfg(feature = "supabase-vault")]
                {
                    Ok(Box::new(SupabaseVaultProvider::from_config(config.config).await?))
                }
                #[cfg(not(feature = "supabase-vault"))]
                {
                    Err(VaultError::invalid_config("Supabase Vault support not enabled").into())
                }
            }
        }
    }
}

/// Local file-based vault provider for development.
#[derive(Debug)]
pub struct LocalVaultProvider {
    secrets_file: PathBuf,
    secrets: HashMap<String, String>,
}

impl LocalVaultProvider {
    /// Create a new local vault provider.
    pub async fn new<P: Into<PathBuf>>(secrets_file: P) -> Result<Self, VaultError> {
        let secrets_file = secrets_file.into();

        if !secrets_file.exists() {
            return Err(VaultError::local_secrets_file(format!(
                "Secrets file not found: {}",
                secrets_file.display()
            )));
        }

        let content = tokio::fs::read_to_string(&secrets_file)
            .await
            .map_err(|e| VaultError::local_secrets_file(format!(
                "Failed to read secrets file: {}", e
            )))?;

        let secrets: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| VaultError::local_secrets_file(format!(
                "Failed to parse secrets file: {}", e
            )))?;

        info!("Loaded {} secrets from {}", secrets.len(), secrets_file.display());

        Ok(Self { secrets_file, secrets })
    }

    /// Create from environment variables.
    pub async fn from_env() -> Result<Self, VaultError> {
        let secrets_file = env::var("RLM_LOCAL_SECRETS_FILE")
            .or_else(|_| {
                dirs::home_dir()
                    .map(|home| home.join(".rlm/secrets/test-secrets.json").to_string_lossy().to_string())
                    .ok_or_else(|| VaultError::missing_env_var("RLM_LOCAL_SECRETS_FILE or HOME"))
            })
            .map_err(|e| match e {
                VaultError::MissingEnvVar { .. } => e,
                _ => VaultError::missing_env_var("RLM_LOCAL_SECRETS_FILE"),
            })?;

        Self::new(secrets_file).await
    }

    /// Create from configuration.
    pub async fn from_config(config: serde_json::Value) -> Result<Self, VaultError> {
        #[derive(Deserialize)]
        struct LocalConfig {
            secrets_file: String,
        }

        let local_config: LocalConfig = serde_json::from_value(config)
            .map_err(|e| VaultError::invalid_config(format!("Invalid local vault config: {}", e)))?;

        Self::new(local_config.secrets_file).await
    }
}

#[async_trait]
impl KeyVaultProvider for LocalVaultProvider {
    async fn get_secret(&self, key: &str) -> Result<String, VaultError> {
        self.secrets
            .get(key)
            .cloned()
            .ok_or_else(|| VaultError::secret_not_found(key))
    }

    async fn health_check(&self) -> Result<(), VaultError> {
        if self.secrets_file.exists() {
            debug!("Local vault health check passed: {} secrets available", self.secrets.len());
            Ok(())
        } else {
            Err(VaultError::local_secrets_file("Secrets file not found"))
        }
    }

    fn metadata(&self) -> VaultMetadata {
        let mut extra = HashMap::new();
        extra.insert("secrets_count".to_string(), self.secrets.len().to_string());
        extra.insert("secrets_file".to_string(), self.secrets_file.display().to_string());

        VaultMetadata {
            provider_type: VaultProviderType::Local,
            endpoint: None,
            extra,
        }
    }
}

/// HashiCorp Vault provider.
#[cfg(feature = "hashicorp-vault")]
#[derive(Debug)]
pub struct HashiCorpVaultProvider {
    client: vault::Client,
    mount_path: String,
    role_id: String,
}

#[cfg(feature = "hashicorp-vault")]
impl HashiCorpVaultProvider {
    /// Create a new HashiCorp Vault provider.
    pub async fn new(
        addr: String,
        role_id: String,
        secret_id: String,
        mount_path: Option<String>,
    ) -> Result<Self, VaultError> {
        use vault::auth::AppRoleAuth;

        let client = vault::Client::new(&addr)
            .map_err(|e| VaultError::connection(format!("Failed to create vault client: {}", e)))?;

        // Authenticate with AppRole
        let auth = AppRoleAuth::new(&role_id, &secret_id);
        client.auth(&auth)
            .await
            .map_err(|e| VaultError::authentication(format!("AppRole authentication failed: {}", e)))?;

        let mount_path = mount_path.unwrap_or_else(|| "secret".to_string());

        info!("Connected to HashiCorp Vault at {}", addr);

        Ok(Self {
            client,
            mount_path,
            role_id,
        })
    }

    /// Create from environment variables.
    pub async fn from_env() -> Result<Self, VaultError> {
        let addr = env::var("VAULT_ADDR")
            .map_err(|_| VaultError::missing_env_var("VAULT_ADDR"))?;
        let role_id = env::var("VAULT_ROLE_ID")
            .map_err(|_| VaultError::missing_env_var("VAULT_ROLE_ID"))?;
        let secret_id = env::var("VAULT_SECRET_ID")
            .map_err(|_| VaultError::missing_env_var("VAULT_SECRET_ID"))?;
        let mount_path = env::var("VAULT_MOUNT_PATH").ok();

        Self::new(addr, role_id, secret_id, mount_path).await
    }

    /// Create from configuration.
    pub async fn from_config(config: serde_json::Value) -> Result<Self, VaultError> {
        #[derive(Deserialize)]
        struct HashiCorpConfig {
            addr: String,
            role_id: String,
            secret_id: String,
            mount_path: Option<String>,
        }

        let hc_config: HashiCorpConfig = serde_json::from_value(config)
            .map_err(|e| VaultError::invalid_config(format!("Invalid HashiCorp vault config: {}", e)))?;

        Self::new(hc_config.addr, hc_config.role_id, hc_config.secret_id, hc_config.mount_path).await
    }
}

#[cfg(feature = "hashicorp-vault")]
#[async_trait]
impl KeyVaultProvider for HashiCorpVaultProvider {
    async fn get_secret(&self, key: &str) -> Result<String, VaultError> {
        let path = format!("{}/{}", self.mount_path, key);

        let response = self.client
            .logical()
            .read(&path)
            .await
            .map_err(|e| VaultError::operation(format!("Failed to read secret '{}': {}", key, e)))?;

        response
            .data
            .get("value")
            .or_else(|| response.data.get(key))
            .cloned()
            .ok_or_else(|| VaultError::secret_not_found(key))
    }

    async fn health_check(&self) -> Result<(), VaultError> {
        self.client
            .sys()
            .health()
            .await
            .map_err(|e| VaultError::connection(format!("Vault health check failed: {}", e)))?;

        debug!("HashiCorp Vault health check passed");
        Ok(())
    }

    fn metadata(&self) -> VaultMetadata {
        let mut extra = HashMap::new();
        extra.insert("mount_path".to_string(), self.mount_path.clone());
        extra.insert("role_id".to_string(), self.role_id.clone());

        VaultMetadata {
            provider_type: VaultProviderType::HashiCorp,
            endpoint: Some(self.client.address().to_string()),
            extra,
        }
    }
}

/// AWS Secrets Manager provider.
#[cfg(feature = "aws-secrets")]
#[derive(Debug)]
pub struct AwsSecretsProvider {
    client: aws_sdk_secretsmanager::Client,
    region: String,
}

#[cfg(feature = "aws-secrets")]
impl AwsSecretsProvider {
    /// Create a new AWS Secrets Manager provider.
    pub async fn new(region: Option<String>) -> Result<Self, VaultError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_secretsmanager::Client::new(&config);
        let region = region.unwrap_or_else(|| config.region().unwrap().to_string());

        info!("Connected to AWS Secrets Manager in region {}", region);

        Ok(Self { client, region })
    }

    /// Create from environment variables.
    pub async fn from_env() -> Result<Self, VaultError> {
        let region = env::var("AWS_REGION").ok();
        Self::new(region).await
    }

    /// Create from configuration.
    pub async fn from_config(config: serde_json::Value) -> Result<Self, VaultError> {
        #[derive(Deserialize)]
        struct AwsConfig {
            region: Option<String>,
        }

        let aws_config: AwsConfig = serde_json::from_value(config)
            .map_err(|e| VaultError::invalid_config(format!("Invalid AWS config: {}", e)))?;

        Self::new(aws_config.region).await
    }
}

#[cfg(feature = "aws-secrets")]
#[async_trait]
impl KeyVaultProvider for AwsSecretsProvider {
    async fn get_secret(&self, key: &str) -> Result<String, VaultError> {
        let response = self.client
            .get_secret_value()
            .secret_id(key)
            .send()
            .await
            .map_err(|e| VaultError::operation(format!("Failed to get AWS secret '{}': {}", key, e)))?;

        response
            .secret_string()
            .ok_or_else(|| VaultError::secret_not_found(key))
            .map(|s| s.to_string())
    }

    async fn health_check(&self) -> Result<(), VaultError> {
        // Test connectivity by listing secrets (with minimal results)
        self.client
            .list_secrets()
            .max_results(1)
            .send()
            .await
            .map_err(|e| VaultError::connection(format!("AWS Secrets Manager health check failed: {}", e)))?;

        debug!("AWS Secrets Manager health check passed");
        Ok(())
    }

    fn metadata(&self) -> VaultMetadata {
        let mut extra = HashMap::new();
        extra.insert("region".to_string(), self.region.clone());

        VaultMetadata {
            provider_type: VaultProviderType::AwsSecrets,
            endpoint: Some(format!("https://secretsmanager.{}.amazonaws.com", self.region)),
            extra,
        }
    }
}

/// Azure Key Vault provider.
#[cfg(feature = "azure-keyvault")]
#[derive(Debug)]
pub struct AzureKeyVaultProvider {
    client: azure_security_keyvault::SecretClient,
    vault_url: String,
}

#[cfg(feature = "azure-keyvault")]
impl AzureKeyVaultProvider {
    /// Create a new Azure Key Vault provider.
    pub async fn new(vault_url: String, credential: std::sync::Arc<dyn azure_core::auth::TokenCredential>) -> Result<Self, VaultError> {
        let client = azure_security_keyvault::SecretClient::new(&vault_url, credential)
            .map_err(|e| VaultError::connection(format!("Failed to create Azure Key Vault client: {}", e)))?;

        info!("Connected to Azure Key Vault at {}", vault_url);

        Ok(Self { client, vault_url })
    }

    /// Create from environment variables.
    pub async fn from_env() -> Result<Self, VaultError> {
        let vault_url = env::var("AZURE_KEYVAULT_URL")
            .map_err(|_| VaultError::missing_env_var("AZURE_KEYVAULT_URL"))?;

        // Use default Azure credential (environment variables, managed identity, etc.)
        let credential = azure_identity::DefaultAzureCredential::default();

        Self::new(vault_url, Arc::new(credential)).await
    }

    /// Create from configuration.
    pub async fn from_config(config: serde_json::Value) -> Result<Self, VaultError> {
        #[derive(Deserialize)]
        struct AzureConfig {
            vault_url: String,
        }

        let azure_config: AzureConfig = serde_json::from_value(config)
            .map_err(|e| VaultError::invalid_config(format!("Invalid Azure config: {}", e)))?;

        let credential = azure_identity::DefaultAzureCredential::default();
        Self::new(azure_config.vault_url, Arc::new(credential)).await
    }
}

#[cfg(feature = "azure-keyvault")]
#[async_trait]
impl KeyVaultProvider for AzureKeyVaultProvider {
    async fn get_secret(&self, key: &str) -> Result<String, VaultError> {
        let secret = self.client
            .get_secret(key, None)
            .await
            .map_err(|e| VaultError::operation(format!("Failed to get Azure Key Vault secret '{}': {}", key, e)))?;

        secret.value
            .ok_or_else(|| VaultError::secret_not_found(key))
    }

    async fn health_check(&self) -> Result<(), VaultError> {
        // Test connectivity by attempting to list secrets (first page only)
        self.client
            .list_secrets(None)
            .await
            .map_err(|e| VaultError::connection(format!("Azure Key Vault health check failed: {}", e)))?;

        debug!("Azure Key Vault health check passed");
        Ok(())
    }

    fn metadata(&self) -> VaultMetadata {
        VaultMetadata {
            provider_type: VaultProviderType::AzureKeyVault,
            endpoint: Some(self.vault_url.clone()),
            extra: HashMap::new(),
        }
    }
}

/// Supabase Vault provider for both self-hosted and cloud installations.
#[cfg(feature = "supabase-vault")]
#[derive(Debug)]
pub struct SupabaseVaultProvider {
    client: postgrest::Postgrest,
    secrets_table: String,
    auth_token: String,
    project_url: String,
}

#[cfg(feature = "supabase-vault")]
impl SupabaseVaultProvider {
    /// Create a new Supabase Vault provider.
    pub async fn new(
        project_url: String,
        auth_token: String,
        secrets_table: Option<String>,
    ) -> Result<Self, VaultError> {
        let secrets_table = secrets_table.unwrap_or_else(|| "vault_secrets".to_string());

        let client = postgrest::Postgrest::new(&project_url)
            .insert_header("authorization", format!("Bearer {}", auth_token))
            .insert_header("apikey", &auth_token);

        info!("Connected to Supabase Vault at {}", project_url);

        Ok(Self {
            client,
            secrets_table,
            auth_token,
            project_url,
        })
    }

    /// Create from environment variables.
    ///
    /// Expected environment variables:
    /// - `SUPABASE_URL`: The Supabase project URL
    /// - `SUPABASE_ANON_KEY` or `SUPABASE_SERVICE_ROLE_KEY`: Authentication key
    /// - `SUPABASE_SECRETS_TABLE` (optional): Name of secrets table (default: "vault_secrets")
    pub async fn from_env() -> Result<Self, VaultError> {
        let project_url = env::var("SUPABASE_URL")
            .map_err(|_| VaultError::missing_env_var("SUPABASE_URL"))?;

        let auth_token = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .or_else(|_| env::var("SUPABASE_ANON_KEY"))
            .map_err(|_| VaultError::missing_env_var("SUPABASE_SERVICE_ROLE_KEY or SUPABASE_ANON_KEY"))?;

        let secrets_table = env::var("SUPABASE_SECRETS_TABLE").ok();

        Self::new(project_url, auth_token, secrets_table).await
    }

    /// Create from configuration.
    pub async fn from_config(config: serde_json::Value) -> Result<Self, VaultError> {
        #[derive(Deserialize)]
        struct SupabaseConfig {
            project_url: String,
            auth_token: String,
            secrets_table: Option<String>,
        }

        let supabase_config: SupabaseConfig = serde_json::from_value(config)
            .map_err(|e| VaultError::invalid_config(format!("Invalid Supabase config: {}", e)))?;

        Self::new(
            supabase_config.project_url,
            supabase_config.auth_token,
            supabase_config.secrets_table,
        ).await
    }

    /// Initialize the secrets table if it doesn't exist.
    pub async fn init_table(&self) -> Result<(), VaultError> {
        // Create the secrets table with proper RLS policies
        let create_table_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
                key_name TEXT UNIQUE NOT NULL,
                key_value TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            );

            -- Enable RLS
            ALTER TABLE {} ENABLE ROW LEVEL SECURITY;

            -- Create RLS policy (adjust based on your auth setup)
            DO $$ BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM pg_policies
                    WHERE tablename = '{}' AND policyname = 'vault_secrets_policy'
                ) THEN
                    CREATE POLICY vault_secrets_policy ON {}
                        FOR ALL USING (true);
                END IF;
            END $$;
            "#,
            self.secrets_table, self.secrets_table, self.secrets_table, self.secrets_table
        );

        self.client
            .rpc("exec_sql", &serde_json::json!({"sql": create_table_sql}))
            .execute()
            .await
            .map_err(|e| VaultError::operation(format!("Failed to initialize Supabase secrets table: {}", e)))?;

        info!("Supabase secrets table '{}' initialized", self.secrets_table);
        Ok(())
    }

    /// Store a secret in the Supabase vault.
    pub async fn store_secret(&self, key: &str, value: &str) -> Result<(), VaultError> {
        let secret_data = serde_json::json!({
            "key_name": key,
            "key_value": value,
            "updated_at": chrono::Utc::now().to_rfc3339()
        });

        self.client
            .from(&self.secrets_table)
            .upsert(&secret_data.to_string())
            .on_conflict("key_name")
            .execute()
            .await
            .map_err(|e| VaultError::operation(format!("Failed to store secret '{}': {}", key, e)))?;

        debug!("Stored secret '{}' in Supabase vault", key);
        Ok(())
    }

    /// Delete a secret from the Supabase vault.
    pub async fn delete_secret(&self, key: &str) -> Result<(), VaultError> {
        self.client
            .from(&self.secrets_table)
            .delete()
            .eq("key_name", key)
            .execute()
            .await
            .map_err(|e| VaultError::operation(format!("Failed to delete secret '{}': {}", key, e)))?;

        debug!("Deleted secret '{}' from Supabase vault", key);
        Ok(())
    }

    /// List all secret keys (without values) in the vault.
    pub async fn list_secret_keys(&self) -> Result<Vec<String>, VaultError> {
        let response = self.client
            .from(&self.secrets_table)
            .select("key_name")
            .execute()
            .await
            .map_err(|e| VaultError::operation(format!("Failed to list secrets: {}", e)))?;

        let response_text = response.text().await
            .map_err(|e| VaultError::operation(format!("Failed to read response: {}", e)))?;

        #[derive(Deserialize)]
        struct KeyEntry {
            key_name: String,
        }

        let entries: Vec<KeyEntry> = serde_json::from_str(&response_text)
            .map_err(|e| VaultError::operation(format!("Failed to parse secrets list: {}", e)))?;

        Ok(entries.into_iter().map(|entry| entry.key_name).collect())
    }
}

#[cfg(feature = "supabase-vault")]
#[async_trait]
impl KeyVaultProvider for SupabaseVaultProvider {
    async fn get_secret(&self, key: &str) -> Result<String, VaultError> {
        let response = self.client
            .from(&self.secrets_table)
            .select("key_value")
            .eq("key_name", key)
            .limit(1)
            .single()
            .execute()
            .await
            .map_err(|e| VaultError::operation(format!("Failed to get secret '{}': {}", key, e)))?;

        let response_text = response.text().await
            .map_err(|e| VaultError::operation(format!("Failed to read response: {}", e)))?;

        if response_text.is_empty() {
            return Err(VaultError::secret_not_found(key));
        }

        #[derive(Deserialize)]
        struct SecretEntry {
            key_value: String,
        }

        let secret: SecretEntry = serde_json::from_str(&response_text)
            .map_err(|e| VaultError::operation(format!("Failed to parse secret '{}': {}", key, e)))?;

        debug!("Retrieved secret '{}' from Supabase vault", key);
        Ok(secret.key_value)
    }

    async fn health_check(&self) -> Result<(), VaultError> {
        // Test connectivity by checking if we can query the secrets table
        let response = self.client
            .from(&self.secrets_table)
            .select("count(*)")
            .limit(1)
            .execute()
            .await
            .map_err(|e| VaultError::connection(format!("Supabase health check failed: {}", e)))?;

        if response.status().is_success() {
            debug!("Supabase Vault health check passed");
            Ok(())
        } else {
            Err(VaultError::connection(format!(
                "Supabase health check failed with status: {}",
                response.status()
            )))
        }
    }

    fn metadata(&self) -> VaultMetadata {
        let mut extra = HashMap::new();
        extra.insert("secrets_table".to_string(), self.secrets_table.clone());
        extra.insert("project_url".to_string(), self.project_url.clone());

        VaultMetadata {
            provider_type: VaultProviderType::SupabaseVault,
            endpoint: Some(self.project_url.clone()),
            extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_local_vault_provider() {
        let temp_dir = TempDir::new().unwrap();
        let secrets_file = temp_dir.path().join("secrets.json");

        let secrets = serde_json::json!({
            "test_key": "test_value",
            "openai_api_key": "sk-test123"
        });

        fs::write(&secrets_file, secrets.to_string()).await.unwrap();

        let provider = LocalVaultProvider::new(&secrets_file).await.unwrap();

        // Test getting existing secret
        assert_eq!(provider.get_secret("test_key").await.unwrap(), "test_value");
        assert_eq!(provider.get_secret("openai_api_key").await.unwrap(), "sk-test123");

        // Test getting non-existent secret
        assert!(provider.get_secret("missing_key").await.is_err());

        // Test health check
        assert!(provider.health_check().await.is_ok());

        // Test metadata
        let metadata = provider.metadata();
        assert!(matches!(metadata.provider_type, VaultProviderType::Local));
        assert_eq!(metadata.extra.get("secrets_count").unwrap(), "2");
    }

    #[tokio::test]
    async fn test_vault_factory_local() {
        let temp_dir = TempDir::new().unwrap();
        let secrets_file = temp_dir.path().join("secrets.json");

        let secrets = serde_json::json!({
            "test_key": "test_value"
        });

        fs::write(&secrets_file, secrets.to_string()).await.unwrap();

        env::set_var("RLM_VAULT_PROVIDER", "local");
        env::set_var("RLM_LOCAL_SECRETS_FILE", secrets_file.to_str().unwrap());

        let provider = VaultFactory::from_env().await.unwrap();
        assert_eq!(provider.get_secret("test_key").await.unwrap(), "test_value");

        env::remove_var("RLM_VAULT_PROVIDER");
        env::remove_var("RLM_LOCAL_SECRETS_FILE");
    }

    #[tokio::test]
    async fn test_vault_error_handling() {
        // Test missing file
        let result = LocalVaultProvider::new("/nonexistent/path").await;
        assert!(result.is_err());

        // Test invalid JSON
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = temp_dir.path().join("invalid.json");
        fs::write(&invalid_file, "invalid json").await.unwrap();

        let result = LocalVaultProvider::new(&invalid_file).await;
        assert!(result.is_err());
    }
}