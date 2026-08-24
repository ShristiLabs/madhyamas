//! Plugin project templates — scaffolding for new plugins.
//!
//! Provides built-in templates that generate a complete, compilable plugin
//! project (Cargo.toml, src/lib.rs, manifest) ready for development.
//!
//! # Usage
//!
//! ```no_run
//! use madhyamas_core::{PluginTemplates, TemplateId};
//!
//! // List available templates.
//! for t in PluginTemplates::all() {
//!     println!("{}: {}", t.id.as_str(), t.description);
//! }
//!
//! // Scaffold a new plugin from the "cors" template.
//! PluginTemplates::scaffold(&TemplateId::Cors, "my-cors-plugin", std::path::Path::new("/path/to/output"))
//!     .expect("failed to scaffold");
//! ```

use crate::Error;
use std::path::Path;

/// Identifier for a built-in plugin template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateId {
    /// Minimal "hello world" plugin that logs every request.
    Basic,
    /// CORS helper — adds CORS headers to every response.
    Cors,
    /// Request logger — logs method, host, and path of every request.
    RequestLogger,
    /// Domain blocker — blocks requests to configurable domains (with settings).
    DomainBlocker,
    /// Response modifier — modify response headers and body.
    ResponseModifier,
}

impl TemplateId {
    /// Returns the string identifier for this template.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Cors => "cors",
            Self::RequestLogger => "request-logger",
            Self::DomainBlocker => "domain-blocker",
            Self::ResponseModifier => "response-modifier",
        }
    }

    /// Parse a template id from a string.
    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "basic" => Some(Self::Basic),
            "cors" => Some(Self::Cors),
            "request-logger" => Some(Self::RequestLogger),
            "domain-blocker" => Some(Self::DomainBlocker),
            "response-modifier" => Some(Self::ResponseModifier),
            _ => None,
        }
    }

    /// Returns all available template ids.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Basic,
            Self::Cors,
            Self::RequestLogger,
            Self::DomainBlocker,
            Self::ResponseModifier,
        ]
    }
}

/// Metadata about a plugin template.
#[derive(Debug)]
pub struct PluginTemplate {
    pub id: TemplateId,
    pub name: &'static str,
    pub description: &'static str,
    pub hooks: &'static [&'static str],
}

/// Built-in plugin templates.
pub struct PluginTemplates;

impl PluginTemplates {
    /// Returns metadata for all available templates.
    pub fn all() -> Vec<PluginTemplate> {
        vec![
            PluginTemplate {
                id: TemplateId::Basic,
                name: "Basic Plugin",
                description: "Minimal plugin that logs every request. Good starting point.",
                hooks: &["on_request"],
            },
            PluginTemplate {
                id: TemplateId::Cors,
                name: "CORS Helper",
                description: "Adds CORS headers (Access-Control-Allow-Origin: *) to every response.",
                hooks: &["on_response"],
            },
            PluginTemplate {
                id: TemplateId::RequestLogger,
                name: "Request Logger",
                description: "Logs the method, host, and path of every request.",
                hooks: &["on_request"],
            },
            PluginTemplate {
                id: TemplateId::DomainBlocker,
                name: "Domain Blocker",
                description: "Blocks requests to configurable domains. Demonstrates settings + short-circuit.",
                hooks: &["on_request"],
            },
            PluginTemplate {
                id: TemplateId::ResponseModifier,
                name: "Response Modifier",
                description: "Modifies response headers and body. Demonstrates response interception.",
                hooks: &["on_response"],
            },
        ]
    }

    /// Scaffold a new plugin project from a template.
    ///
    /// Creates a directory at `output_dir/plugin_name/` with:
    /// - `Cargo.toml`
    /// - `src/lib.rs`
    /// - `madhyamas-plugin.toml`
    pub fn scaffold(
        template: &TemplateId,
        plugin_name: &str,
        output_dir: &Path,
    ) -> crate::Result<()> {
        let plugin_dir = output_dir.join(plugin_name);
        if plugin_dir.exists() {
            return Err(Error::Config(format!(
                "output directory already exists: {:?}",
                plugin_dir
            )));
        }

        std::fs::create_dir_all(&plugin_dir)?;
        std::fs::create_dir_all(plugin_dir.join("src"))?;

        // Generate Cargo.toml.
        let cargo_toml = generate_cargo_toml(plugin_name);
        std::fs::write(plugin_dir.join("Cargo.toml"), cargo_toml)?;

        // Generate src/lib.rs.
        let lib_rs = generate_lib_rs(template, plugin_name);
        std::fs::write(plugin_dir.join("src").join("lib.rs"), lib_rs)?;

        // Generate manifest.
        let manifest = generate_manifest(template, plugin_name);
        std::fs::write(plugin_dir.join("madhyamas-plugin.toml"), manifest)?;

        // Generate a .gitignore.
        std::fs::write(plugin_dir.join(".gitignore"), "/target\n")?;

        // Generate a README.
        let readme = generate_readme(template, plugin_name);
        std::fs::write(plugin_dir.join("README.md"), readme)?;

        Ok(())
    }
}

fn generate_cargo_toml(plugin_name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
madhyamas-plugin-sdk = {{ path = "../../crates/madhyamas-plugin-sdk" }}
serde = {{ version = "1.0", default-features = false, features = ["derive", "alloc"] }}
serde_json = {{ version = "1.0", default-features = false, features = ["alloc"] }}
"#,
        name = plugin_name
    )
}

fn generate_manifest(template: &TemplateId, plugin_name: &str) -> String {
    let (hooks, extra) = match template {
        TemplateId::Basic => ("on_request", ""),
        TemplateId::Cors => ("on_response", ""),
        TemplateId::RequestLogger => ("on_request", ""),
        TemplateId::DomainBlocker => (
            "on_request",
            "\n[[settings.fields]]\nkey = \"blocked_domains\"\nlabel = \"Blocked Domains\"\nfield_type = \"textarea\"\ndefault = \"example.com\\nevil.com\"\ndescription = \"One domain per line\"\n",
        ),
        TemplateId::ResponseModifier => ("on_response", ""),
    };
    format!(
        "id = \"{}\"\nname = \"{}\"\nversion = \"0.1.0\"\ndescription = \"Generated from the {} template\"\nhooks = [\"{}\"]\nenabled_by_default = false\n\n[capabilities]\ncapabilities = [\"intercept_{}\"]\n\nmax_memory_pages = 64\nfuel_limit = 10000000\n{}",
        plugin_name,
        plugin_name.replace('-', " "),
        template.as_str(),
        hooks,
        hooks,
        extra,
    )
}

fn generate_lib_rs(template: &TemplateId, plugin_name: &str) -> String {
    match template {
        TemplateId::Basic => format!(
            r#"//! {name} — basic Madhyamas plugin.

#![no_std]
extern crate alloc;

use alloc::format;
use madhyamas_plugin_sdk::{{
    log, log_level, register_plugin, Context, Outcome, Plugin,
}};

#[derive(Default)]
struct {struct_name};

impl Plugin for {struct_name} {{
    fn on_request(&mut self, ctx: &mut Context) -> Outcome {{
        if let Some(req) = ctx.request() {{
            log(
                log_level::INFO,
                &format!("{{}} {{}}", req.method, req.url),
            );
        }}
        Outcome::pass()
    }}
}}

register_plugin!({struct_name});
"#,
            name = plugin_name,
            struct_name = to_struct_name(plugin_name),
        ),

        TemplateId::Cors => format!(
            r#"//! {name} — CORS helper plugin.

#![no_std]
extern crate alloc;

use madhyamas_plugin_sdk::{{register_plugin, Context, Outcome, Plugin}};

#[derive(Default)]
struct {struct_name};

impl Plugin for {struct_name} {{
    fn on_response(&mut self, ctx: &mut Context) -> Outcome {{
        if let Some(resp) = ctx.response_mut() {{
            resp.headers.insert(
                "Access-Control-Allow-Origin".into(),
                "*".into(),
            );
            resp.headers.insert(
                "Access-Control-Allow-Methods".into(),
                "GET, POST, PUT, DELETE, OPTIONS".into(),
            );
            resp.headers.insert(
                "Access-Control-Allow-Headers".into(),
                "*".into(),
            );
        }}
        Outcome::modified()
    }}
}}

register_plugin!({struct_name});
"#,
            name = plugin_name,
            struct_name = to_struct_name(plugin_name),
        ),

        TemplateId::RequestLogger => format!(
            r#"//! {name} — request logger plugin.

#![no_std]
extern crate alloc;

use alloc::format;
use madhyamas_plugin_sdk::{{log, log_level, register_plugin, Context, Outcome, Plugin}};

#[derive(Default)]
struct {struct_name};

impl Plugin for {struct_name} {{
    fn on_request(&mut self, ctx: &mut Context) -> Outcome {{
        if let Some(req) = ctx.request() {{
            log(
                log_level::INFO,
                &format!("{{}} {{}}{{}}", req.method, req.host, req.path),
            );
        }}
        Outcome::pass()
    }}
}}

register_plugin!({struct_name});
"#,
            name = plugin_name,
            struct_name = to_struct_name(plugin_name),
        ),

        TemplateId::DomainBlocker => format!(
            r#"//! {name} — domain blocker plugin (with settings).

#![no_std]
extern crate alloc;

use alloc::{{format, string::{{String, ToString}}, vec::Vec}};
use madhyamas_plugin_sdk::{{log, log_level, register_plugin, Context, Outcome, Plugin}};

#[derive(Default)]
struct {struct_name};

impl Plugin for {struct_name} {{
    fn on_request(&mut self, ctx: &mut Context) -> Outcome {{
        // Read blocked domains from settings (newline-separated string).
        let blocked: Vec<String> = ctx
            .setting("blocked_domains")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default()
            .split('\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if let Some(req) = ctx.request() {{
            if blocked.iter().any(|d| &req.host == d) {{
                log(
                    log_level::WARN,
                    &format!("blocking request to {{}}", req.host),
                );
                return Outcome::respond(403, "blocked by domain-blocker plugin");
            }}
        }}
        Outcome::pass()
    }}
}}

register_plugin!({struct_name});
"#,
            name = plugin_name,
            struct_name = to_struct_name(plugin_name),
        ),

        TemplateId::ResponseModifier => format!(
            r#"//! {name} — response modifier plugin.

#![no_std]
extern crate alloc;

use madhyamas_plugin_sdk::{{register_plugin, Context, Outcome, Plugin}};

#[derive(Default)]
struct {struct_name};

impl Plugin for {struct_name} {{
    fn on_response(&mut self, ctx: &mut Context) -> Outcome {{
        if let Some(resp) = ctx.response_mut() {{
            // Add a custom header.
            resp.headers.insert(
                "X-Modified-By".into(),
                "{name}".into(),
            );
            // You can also modify the body, status code, etc.
        }}
        Outcome::modified()
    }}
}}

register_plugin!({struct_name});
"#,
            name = plugin_name,
            struct_name = to_struct_name(plugin_name),
        ),
    }
}

fn generate_readme(template: &TemplateId, plugin_name: &str) -> String {
    format!(
        r#"# {name}

Generated from the `{template}` template.

## Building

```bash
# Add the WASM target
rustup target add wasm32-unknown-unknown

# Build
cargo build --target wasm32-unknown-unknown --release
```

The output is at `target/wasm32-unknown-unknown/release/{wasm_name}.wasm`.
Copy it to `plugin.wasm` in this directory.

## Installation

```bash
madhyamas plugins install --source url file://$(pwd)
```

Or copy this directory to `~/.madhyamas/plugins/{name}/`.
"#,
        name = plugin_name,
        template = template.as_str(),
        wasm_name = plugin_name.replace('-', "_"),
    )
}

/// Convert a kebab-case plugin name to a PascalCase struct name.
fn to_struct_name(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_struct_name() {
        assert_eq!(to_struct_name("my-plugin"), "MyPlugin");
        assert_eq!(to_struct_name("cors-helper"), "CorsHelper");
        assert_eq!(to_struct_name("simple"), "Simple");
    }
}
