# Security Guidelines

## Core Principles

1. **Defense in Depth** - Multiple layers of security
2. **Least Privilege** - Minimal permissions by default
3. **Fail Secure** - Errors should not expose vulnerabilities
4. **Input Validation** - Never trust external input

## Input Validation

```rust
pub fn process_prompt(input: &str) -> Result<String> {
    // Sanitize
    let sanitized = sanitize_input(input)?;

    // Validate length
    if sanitized.len() > MAX_PROMPT_LENGTH {
        return Err(Error::InputTooLong);
    }

    // Check for injection patterns
    if contains_injection_patterns(&sanitized) {
        return Err(Error::PotentialInjection);
    }

    Ok(sanitized)
}

fn sanitize_input(input: &str) -> Result<String> {
    // Remove null bytes
    let cleaned = input.replace('\0', "");

    // Normalize unicode
    let normalized = unicode_normalization::nfc(&cleaned);

    Ok(normalized.to_string())
}
```

## Secrets Management

Rules:
- **Never** store secrets in code or tracked config files
- Use environment variables or secure vaults
- Secrets in `.local.md` files must be gitignored
- Rotate API keys regularly
- Use `gear-core cli secrets` for encrypted storage

```rust
// Reading secrets safely
pub fn get_api_key() -> Result<String> {
    // Priority: env var > secrets store > error
    if let Ok(key) = std::env::var("GEAR_API_KEY") {
        return Ok(key);
    }

    if let Some(key) = secrets_store::get("api_key")? {
        return Ok(key);
    }

    Err(Error::MissingApiKey)
}
```

## Network Security

Requirements:
- All uplink communication over TLS 1.3+
- Certificate pinning for critical endpoints
- Rate limiting on all APIs
- Authentication required for cross-machine operations

```rust
// Secure HTTP client configuration
pub fn secure_client() -> reqwest::Client {
    reqwest::Client::builder()
        .min_tls_version(tls::Version::TLS_1_3)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build secure client")
}
```

## Hook Security

Guidelines:
- Hooks run with user permissions
- Validate all hook input JSON
- Use `${CLAUDE_PLUGIN_ROOT}` for paths
- Timeout all hook operations
- Never execute arbitrary commands from hook input

```rust
// Safe hook input parsing
pub fn parse_hook_input(input: &str) -> Result<HookInput> {
    let parsed: HookInput = serde_json::from_str(input)
        .map_err(|_| Error::InvalidHookInput)?;

    // Validate fields
    if parsed.tool_name.contains("..") {
        return Err(Error::PathTraversal);
    }

    Ok(parsed)
}
```

## File System Security

```rust
// Safe path handling
pub fn safe_path(base: &Path, user_path: &str) -> Result<PathBuf> {
    let requested = PathBuf::from(user_path);

    // Prevent path traversal
    if user_path.contains("..") {
        return Err(Error::PathTraversal);
    }

    let full_path = base.join(&requested);

    // Ensure path stays within base
    if !full_path.starts_with(base) {
        return Err(Error::PathEscape);
    }

    Ok(full_path)
}
```

## Authentication

For uplink and cross-machine operations:
- Use short-lived tokens (1 hour max)
- Implement token refresh mechanism
- Support MFA for sensitive operations
- Log all authentication attempts

## Audit Logging

```rust
pub fn audit_log(event: &AuditEvent) {
    tracing::info!(
        event_type = %event.event_type,
        user = %event.user,
        resource = %event.resource,
        action = %event.action,
        result = %event.result,
        "Audit event"
    );
}
```

## Security Checklist for PRs

- [ ] No hardcoded secrets
- [ ] Input validation on all external data
- [ ] Proper error handling (no sensitive info in errors)
- [ ] Rate limiting considered
- [ ] Authentication/authorization checks
- [ ] Audit logging for sensitive operations
