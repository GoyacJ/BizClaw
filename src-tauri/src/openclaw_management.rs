use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{anyhow, bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    process_exec::new_command,
    types::{
        ClawHubSkillSearchResult, CreateLocalSkillRequest, CreateOpenClawAgentRequest,
        InstallClawHubSkillRequest, OpenClawAgentBinding, OpenClawAgentSummary,
        OpenClawSkillCheckReport, OpenClawSkillInfo, OpenClawSkillInstallHint,
        OpenClawSkillInventory, OpenClawSkillLocationKind, OpenClawSkillRequirements,
        OpenClawSkillSummary, SearchClawHubSkillsRequest, UpdateOpenClawAgentIdentityRequest,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SkillRoots {
    workspace_root: Option<PathBuf>,
    shared_root: Option<PathBuf>,
}

#[derive(Debug)]
struct CommandCapture {
    success: bool,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalSkillLocation {
    WorkspaceLocal,
    SharedLocal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliAgentBindingMatch {
    channel: String,
    #[serde(default)]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliAgentBinding {
    agent_id: String,
    #[serde(rename = "match")]
    binding_match: CliAgentBindingMatch,
    description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliSkillInventory {
    #[serde(default)]
    workspace_dir: Option<String>,
    #[serde(default)]
    managed_skills_dir: Option<String>,
    #[serde(default)]
    skills: Vec<CliSkillSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliSkillSummary {
    name: String,
    description: String,
    #[serde(default)]
    emoji: Option<String>,
    eligible: bool,
    disabled: bool,
    blocked_by_allowlist: bool,
    source: String,
    bundled: bool,
    #[serde(default)]
    primary_env: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    missing: OpenClawSkillRequirements,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliSkillInfo {
    name: String,
    description: String,
    source: String,
    bundled: bool,
    file_path: String,
    base_dir: String,
    skill_key: String,
    #[serde(default)]
    emoji: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    primary_env: Option<String>,
    always: bool,
    disabled: bool,
    blocked_by_allowlist: bool,
    eligible: bool,
    #[serde(default)]
    requirements: OpenClawSkillRequirements,
    #[serde(default)]
    missing: OpenClawSkillRequirements,
    #[serde(default)]
    config_checks: Vec<String>,
    #[serde(default)]
    install: Vec<OpenClawSkillInstallHint>,
}

impl SkillRoots {
    fn from_inventory(inventory: &CliSkillInventory) -> Self {
        Self {
            workspace_root: inventory
                .workspace_dir
                .as_ref()
                .map(|path| PathBuf::from(path).join("skills")),
            shared_root: inventory.managed_skills_dir.as_ref().map(PathBuf::from),
        }
    }

    fn classify_listed_skill(
        &self,
        skill_name: &str,
        source: &str,
        bundled: bool,
    ) -> OpenClawSkillLocationKind {
        if self
            .local_skill_dir(skill_name, LocalSkillLocation::WorkspaceLocal)
            .is_some()
        {
            return OpenClawSkillLocationKind::WorkspaceLocal;
        }
        if self
            .local_skill_dir(skill_name, LocalSkillLocation::SharedLocal)
            .is_some()
        {
            return OpenClawSkillLocationKind::SharedLocal;
        }
        if source.contains("workspace") {
            return OpenClawSkillLocationKind::WorkspaceLocal;
        }
        if bundled {
            return OpenClawSkillLocationKind::Bundled;
        }
        OpenClawSkillLocationKind::External
    }

    fn classify_skill_dir(
        &self,
        base_dir: &Path,
        source: &str,
        bundled: bool,
    ) -> OpenClawSkillLocationKind {
        if let Ok((_, location)) = resolve_deletable_skill_dir(base_dir, self) {
            return match location {
                LocalSkillLocation::WorkspaceLocal => OpenClawSkillLocationKind::WorkspaceLocal,
                LocalSkillLocation::SharedLocal => OpenClawSkillLocationKind::SharedLocal,
            };
        }
        if source.contains("workspace") {
            return OpenClawSkillLocationKind::WorkspaceLocal;
        }
        if bundled {
            return OpenClawSkillLocationKind::Bundled;
        }
        OpenClawSkillLocationKind::External
    }

    fn local_skill_dir(&self, skill_name: &str, location: LocalSkillLocation) -> Option<PathBuf> {
        let root = match location {
            LocalSkillLocation::WorkspaceLocal => self.workspace_root.as_ref(),
            LocalSkillLocation::SharedLocal => self.shared_root.as_ref(),
        }?;
        let skill_dir = root.join(skill_name);
        if skill_dir.is_dir() && skill_dir.join("SKILL.md").is_file() {
            Some(skill_dir)
        } else {
            None
        }
    }
}

pub(crate) fn list_agents() -> Result<Vec<OpenClawAgentSummary>> {
    run_openclaw_json(&["agents", "list", "--json", "--bindings"])
}

pub(crate) fn create_agent(request: &CreateOpenClawAgentRequest) -> Result<Value> {
    let name = request.name.trim();
    let workspace = request.workspace.trim();
    if name.is_empty() {
        bail!("Agent name is required.");
    }
    if workspace.is_empty() {
        bail!("Workspace path is required.");
    }

    let mut args = vec![
        "agents".to_string(),
        "add".to_string(),
        name.to_string(),
        "--workspace".to_string(),
        workspace.to_string(),
        "--non-interactive".to_string(),
        "--json".to_string(),
    ];
    if let Some(model) = request
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    for binding in request
        .bindings
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--bind".to_string());
        args.push(binding.to_string());
    }
    run_openclaw_json_from_args(&args)
}

pub(crate) fn update_agent_identity(request: &UpdateOpenClawAgentIdentityRequest) -> Result<Value> {
    let agent_id = request.agent_id.trim();
    if agent_id.is_empty() {
        bail!("Agent id is required.");
    }

    let mut args = vec![
        "agents".to_string(),
        "set-identity".to_string(),
        "--agent".to_string(),
        agent_id.to_string(),
        "--json".to_string(),
    ];
    let mut changed = false;
    for (flag, value) in [
        ("--name", request.name.as_deref()),
        ("--emoji", request.emoji.as_deref()),
        ("--theme", request.theme.as_deref()),
        ("--avatar", request.avatar.as_deref()),
    ] {
        if let Some(value) = value.map(str::trim).filter(|item| !item.is_empty()) {
            args.push(flag.to_string());
            args.push(value.to_string());
            changed = true;
        }
    }
    if !changed {
        return Ok(json!({
            "agentId": agent_id,
            "updated": false,
        }));
    }
    run_openclaw_json_from_args(&args)
}

pub(crate) fn delete_agent(agent_id: &str) -> Result<Value> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        bail!("Agent id is required.");
    }
    let args = ["agents", "delete", agent_id, "--force", "--json"];
    run_openclaw_json(&args)
}

pub(crate) fn list_agent_bindings(agent_id: Option<&str>) -> Result<Vec<OpenClawAgentBinding>> {
    let mut args = vec![
        "agents".to_string(),
        "bindings".to_string(),
        "--json".to_string(),
    ];
    if let Some(agent_id) = agent_id.map(str::trim).filter(|value| !value.is_empty()) {
        args.push("--agent".to_string());
        args.push(agent_id.to_string());
    }
    let bindings = run_openclaw_json_from_args::<Vec<CliAgentBinding>>(&args)?;
    Ok(bindings
        .into_iter()
        .map(|binding| OpenClawAgentBinding {
            agent_id: binding.agent_id,
            channel: binding.binding_match.channel,
            account_id: binding.binding_match.account_id,
            description: binding.description,
        })
        .collect())
}

pub(crate) fn add_agent_bindings(agent_id: &str, bindings: &[String]) -> Result<Value> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        bail!("Agent id is required.");
    }
    let bindings = cleaned_bindings(bindings);
    if bindings.is_empty() {
        return Ok(json!({
            "agentId": agent_id,
            "added": [],
        }));
    }

    let mut args = vec![
        "agents".to_string(),
        "bind".to_string(),
        "--agent".to_string(),
        agent_id.to_string(),
        "--json".to_string(),
    ];
    for binding in bindings {
        args.push("--bind".to_string());
        args.push(binding);
    }
    run_openclaw_json_from_args(&args)
}

pub(crate) fn remove_agent_bindings(
    agent_id: &str,
    bindings: &[String],
    remove_all: bool,
) -> Result<Value> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        bail!("Agent id is required.");
    }

    let mut args = vec![
        "agents".to_string(),
        "unbind".to_string(),
        "--agent".to_string(),
        agent_id.to_string(),
        "--json".to_string(),
    ];
    if remove_all {
        args.push("--all".to_string());
    } else {
        let bindings = cleaned_bindings(bindings);
        if bindings.is_empty() {
            return Ok(json!({
                "agentId": agent_id,
                "removed": [],
            }));
        }
        for binding in bindings {
            args.push("--bind".to_string());
            args.push(binding);
        }
    }
    run_openclaw_json_from_args(&args)
}

pub(crate) fn list_skills() -> Result<OpenClawSkillInventory> {
    let inventory = fetch_skill_inventory()?;
    let roots = SkillRoots::from_inventory(&inventory);
    Ok(OpenClawSkillInventory {
        workspace_dir: inventory.workspace_dir.clone(),
        managed_skills_dir: inventory.managed_skills_dir.clone(),
        skills: inventory
            .skills
            .into_iter()
            .map(|skill| {
                let location_kind =
                    roots.classify_listed_skill(&skill.name, &skill.source, skill.bundled);
                let can_delete = matches!(
                    location_kind,
                    OpenClawSkillLocationKind::WorkspaceLocal
                        | OpenClawSkillLocationKind::SharedLocal
                ) && resolve_listed_skill_dir(&skill.name, &roots, location_kind)
                    .is_some();

                OpenClawSkillSummary {
                    name: skill.name,
                    description: normalize_text(skill.description),
                    emoji: skill.emoji,
                    eligible: skill.eligible,
                    disabled: skill.disabled,
                    blocked_by_allowlist: skill.blocked_by_allowlist,
                    source: skill.source,
                    bundled: skill.bundled,
                    primary_env: skill.primary_env,
                    homepage: skill.homepage,
                    missing: skill.missing,
                    location_kind,
                    can_delete,
                }
            })
            .collect(),
    })
}

pub(crate) fn check_skills() -> Result<OpenClawSkillCheckReport> {
    run_openclaw_json(&["skills", "check", "--json"])
}

pub(crate) fn get_skill_info(name: &str) -> Result<OpenClawSkillInfo> {
    let skill_name = name.trim();
    if skill_name.is_empty() {
        bail!("Skill name is required.");
    }
    let info: CliSkillInfo = run_openclaw_json(&["skills", "info", skill_name, "--json"])?;
    let roots = SkillRoots::from_inventory(&fetch_skill_inventory()?);
    let base_dir = Path::new(&info.base_dir);
    let location_kind = roots.classify_skill_dir(base_dir, &info.source, info.bundled);
    let can_delete = resolve_deletable_skill_dir(base_dir, &roots).is_ok();

    Ok(OpenClawSkillInfo {
        name: info.name,
        description: normalize_text(info.description),
        source: info.source,
        bundled: info.bundled,
        file_path: info.file_path,
        base_dir: info.base_dir,
        skill_key: info.skill_key,
        emoji: info.emoji,
        homepage: info.homepage,
        primary_env: info.primary_env,
        always: info.always,
        disabled: info.disabled,
        blocked_by_allowlist: info.blocked_by_allowlist,
        eligible: info.eligible,
        requirements: info.requirements,
        missing: info.missing,
        config_checks: info.config_checks,
        install: info.install,
        location_kind,
        can_delete,
    })
}

pub(crate) fn search_clawhub_skills(
    request: &SearchClawHubSkillsRequest,
) -> Result<Vec<ClawHubSkillSearchResult>> {
    let query = request.query.trim();
    if query.is_empty() {
        bail!("Search query is required.");
    }

    let mut args = vec!["search".to_string(), query.to_string()];
    let limit = request.limit.unwrap_or(8).clamp(1, 20);
    args.push("--limit".to_string());
    args.push(limit.to_string());

    let capture = run_clawhub_command(&args, None)?;
    if !capture.success {
        bail!("{}", format_command_failure("clawhub", &args, &capture));
    }

    parse_clawhub_search_results(&capture.stdout)
}

pub(crate) fn install_clawhub_skill(request: &InstallClawHubSkillRequest) -> Result<Value> {
    let slug = request.slug.trim();
    if slug.is_empty() {
        bail!("Skill slug is required.");
    }

    let inventory = fetch_skill_inventory()?;
    let install_cwd = resolve_clawhub_install_cwd(&inventory, request.location)?;
    let args = vec!["install".to_string(), slug.to_string()];
    let capture = run_clawhub_command(&args, Some(&install_cwd))?;
    if !capture.success {
        bail!("{}", format_command_failure("clawhub", &args, &capture));
    }

    Ok(json!({
        "skillName": slug,
        "location": request.location,
        "cwd": install_cwd,
        "stdout": capture.stdout.trim(),
    }))
}

pub(crate) fn create_local_skill(request: &CreateLocalSkillRequest) -> Result<Value> {
    let roots_inventory = fetch_skill_inventory()?;
    let roots = SkillRoots::from_inventory(&roots_inventory);
    let requested_name = request.name.trim();
    let (root, location_kind) = match request.location {
        OpenClawSkillLocationKind::WorkspaceLocal => (
            roots
                .workspace_root
                .as_deref()
                .context("OpenClaw did not report a workspace skills directory.")?,
            OpenClawSkillLocationKind::WorkspaceLocal,
        ),
        OpenClawSkillLocationKind::SharedLocal => (
            roots
                .shared_root
                .as_deref()
                .context("OpenClaw did not report a shared local skills directory.")?,
            OpenClawSkillLocationKind::SharedLocal,
        ),
        OpenClawSkillLocationKind::Bundled | OpenClawSkillLocationKind::External => {
            bail!("Only workspace-local or shared-local skills can be created.")
        }
    };

    let created_dir = create_local_skill_in_root(root, requested_name)?;
    Ok(json!({
        "name": requested_name,
        "locationKind": location_kind,
        "baseDir": created_dir,
        "filePath": created_dir.join("SKILL.md"),
    }))
}

pub(crate) fn delete_local_skill(name: &str) -> Result<Value> {
    let skill_name = name.trim();
    if skill_name.is_empty() {
        bail!("Skill name is required.");
    }

    let roots = SkillRoots::from_inventory(&fetch_skill_inventory()?);
    let info: CliSkillInfo = run_openclaw_json(&["skills", "info", skill_name, "--json"])?;
    let (target_dir, location) = resolve_deletable_skill_dir(Path::new(&info.base_dir), &roots)?;
    fs::remove_dir_all(&target_dir).with_context(|| {
        format!(
            "Failed to delete local skill directory: {}",
            target_dir.display()
        )
    })?;

    let location_kind = match location {
        LocalSkillLocation::WorkspaceLocal => OpenClawSkillLocationKind::WorkspaceLocal,
        LocalSkillLocation::SharedLocal => OpenClawSkillLocationKind::SharedLocal,
    };

    Ok(json!({
        "name": info.name,
        "locationKind": location_kind,
        "baseDir": target_dir,
    }))
}

fn parse_clawhub_search_results(output: &str) -> Result<Vec<ClawHubSkillSearchResult>> {
    let mut results = Vec::new();

    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if line.starts_with("- Searching") || line.starts_with("Searching") {
            continue;
        }

        let (line_without_score, score) = split_clawhub_score(line);
        let Some((slug, title)) = split_clawhub_result_line(line_without_score) else {
            continue;
        };

        results.push(ClawHubSkillSearchResult {
            slug: slug.to_string(),
            title: title.to_string(),
            score,
        });
    }

    Ok(results)
}

fn split_clawhub_score(line: &str) -> (&str, Option<f64>) {
    let Some(start) = line.rfind(" (") else {
        return (line.trim_end(), None);
    };
    if !line.ends_with(')') {
        return (line.trim_end(), None);
    }

    let Ok(score) = line[start + 2..line.len() - 1].parse::<f64>() else {
        return (line.trim_end(), None);
    };
    (line[..start].trim_end(), Some(score))
}

fn split_clawhub_result_line(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    for index in 0..bytes.len().saturating_sub(1) {
        if bytes[index] == b' ' && bytes[index + 1] == b' ' {
            let slug = line[..index].trim();
            let title = line[index..].trim();
            if !slug.is_empty() && !title.is_empty() {
                return Some((slug, title));
            }
        }
    }
    None
}

fn resolve_clawhub_install_cwd(
    inventory: &CliSkillInventory,
    location: OpenClawSkillLocationKind,
) -> Result<PathBuf> {
    match location {
        OpenClawSkillLocationKind::WorkspaceLocal => inventory
            .workspace_dir
            .as_ref()
            .map(PathBuf::from)
            .context("OpenClaw did not report a workspace directory for skill installs."),
        OpenClawSkillLocationKind::SharedLocal => inventory
            .managed_skills_dir
            .as_ref()
            .map(PathBuf::from)
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .context("OpenClaw did not report a shared skills directory for skill installs."),
        OpenClawSkillLocationKind::Bundled | OpenClawSkillLocationKind::External => {
            bail!("Only workspace-local or shared-local skills can be installed.")
        }
    }
}

fn cleaned_bindings(bindings: &[String]) -> Vec<String> {
    bindings
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn fetch_skill_inventory() -> Result<CliSkillInventory> {
    run_openclaw_json(&["skills", "list", "--json", "-v"])
}

fn run_openclaw_json<T: DeserializeOwned>(args: &[&str]) -> Result<T> {
    let args = args
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    run_openclaw_json_from_args(&args)
}

fn run_openclaw_json_from_args<T: DeserializeOwned>(args: &[String]) -> Result<T> {
    let capture = run_openclaw_command(args)?;
    if !capture.success {
        bail!("{}", format_command_failure("openclaw", args, &capture));
    }
    parse_json_from_capture(&capture)
}

fn run_openclaw_command(args: &[String]) -> Result<CommandCapture> {
    run_command_capture("openclaw", args, None).context("Failed to run the OpenClaw CLI.")
}

fn run_clawhub_command(args: &[String], cwd: Option<&Path>) -> Result<CommandCapture> {
    if which::which("clawhub").is_ok() {
        return run_command_capture("clawhub", args, cwd).context("Failed to run the ClawHub CLI.");
    }

    let mut npx_args = vec!["--yes".to_string(), "clawhub".to_string()];
    npx_args.extend(args.iter().cloned());
    run_command_capture("npx", &npx_args, cwd).context("Failed to run ClawHub via npx.")
}

fn run_command_capture(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
) -> Result<CommandCapture> {
    let mut command = new_command(program, args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.stdin(Stdio::null()).output()?;

    Ok(CommandCapture {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn parse_json_from_capture<T: DeserializeOwned>(capture: &CommandCapture) -> Result<T> {
    for source in [&capture.stdout, &capture.stderr] {
        if source.trim().is_empty() {
            continue;
        }
        if let Ok(value) = extract_json_value(source) {
            if let Ok(parsed) = serde_json::from_value::<T>(value) {
                return Ok(parsed);
            }
        }
    }

    let combined = if capture.stderr.trim().is_empty() {
        capture.stdout.clone()
    } else {
        format!("{}\n{}", capture.stdout, capture.stderr)
    };
    let value = extract_json_value(&combined).context("OpenClaw CLI returned no JSON payload.")?;
    serde_json::from_value(value).context("Failed to decode OpenClaw JSON payload.")
}

fn extract_json_value(text: &str) -> Result<Value> {
    let mut last_error = None;
    for (index, ch) in text.char_indices() {
        if ch != '{' && ch != '[' {
            continue;
        }
        let candidate = &text[index..];
        let mut stream = serde_json::Deserializer::from_str(candidate).into_iter::<Value>();
        match stream.next() {
            Some(Ok(value)) => return Ok(value),
            Some(Err(error)) => last_error = Some(error),
            None => continue,
        }
    }

    match last_error {
        Some(error) => Err(anyhow!("Failed to parse OpenClaw JSON payload: {error}")),
        None => Err(anyhow!("No JSON payload found in OpenClaw CLI output.")),
    }
}

fn format_command_failure(program: &str, args: &[String], capture: &CommandCapture) -> String {
    let rendered_args = args.join(" ");
    let stdout = capture.stdout.trim();
    let stderr = capture.stderr.trim();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("Command failed: {program} {rendered_args}"),
        (false, true) => {
            format!("Command failed: {program} {rendered_args}\n{stdout}")
        }
        (true, false) => {
            format!("Command failed: {program} {rendered_args}\n{stderr}")
        }
        (false, false) => {
            format!("Command failed: {program} {rendered_args}\n{stdout}\n{stderr}")
        }
    }
}

fn normalize_text(value: String) -> String {
    value.trim().to_string()
}

fn create_local_skill_in_root(root: &Path, name: &str) -> Result<PathBuf> {
    validate_skill_name(name)?;
    fs::create_dir_all(root)
        .with_context(|| format!("Failed to create local skills root: {}", root.display()))?;

    let skill_dir = root.join(name);
    if skill_dir.exists() {
        bail!(
            "A skill named \"{name}\" already exists in {}.",
            root.display()
        );
    }

    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create skill directory: {}", skill_dir.display()))?;
    let skill_md = skill_dir.join("SKILL.md");
    fs::write(&skill_md, build_skill_starter(name))
        .with_context(|| format!("Failed to write starter skill file: {}", skill_md.display()))?;
    Ok(skill_dir)
}

fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Skill name is required.");
    }
    if name.len() > 64 {
        bail!("Skill name must be 64 characters or fewer.");
    }
    let bytes = name.as_bytes();
    let first = bytes[0];
    let last = *bytes.last().unwrap_or(&first);
    if !is_ascii_skill_edge(first) || !is_ascii_skill_edge(last) {
        bail!("Skill name must start and end with lowercase letters or numbers.");
    }
    if bytes
        .iter()
        .any(|byte| !is_ascii_skill_edge(*byte) && *byte != b'-')
    {
        bail!("Skill name must use lowercase letters, numbers, or hyphens.");
    }
    Ok(())
}

fn is_ascii_skill_edge(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

fn build_skill_starter(name: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: Describe when this skill should be used.\n---\n\n# {name}\n\n## Purpose\n\nDescribe what this skill helps OpenClaw do.\n\n## Usage\n\n- Use when the user asks for this workflow.\n- Add references or scripts in this folder as needed.\n"
    )
}

fn resolve_deletable_skill_dir(
    base_dir: &Path,
    roots: &SkillRoots,
) -> Result<(PathBuf, LocalSkillLocation)> {
    if let Some(path) = resolve_dir_within_root(base_dir, roots.workspace_root.as_deref())? {
        return Ok((path, LocalSkillLocation::WorkspaceLocal));
    }
    if let Some(path) = resolve_dir_within_root(base_dir, roots.shared_root.as_deref())? {
        return Ok((path, LocalSkillLocation::SharedLocal));
    }
    bail!("Skill directory resolves outside the managed local skill roots.");
}

fn resolve_dir_within_root(base_dir: &Path, root: Option<&Path>) -> Result<Option<PathBuf>> {
    let Some(root) = root else {
        return Ok(None);
    };
    if !root.exists() || !base_dir.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(base_dir)
        .with_context(|| format!("Failed to inspect skill directory: {}", base_dir.display()))?;
    if metadata.file_type().is_symlink() {
        bail!(
            "Symlinked skill directories are not deletable from BizClaw: {}",
            base_dir.display()
        );
    }

    let canonical_root = fs::canonicalize(root)
        .with_context(|| format!("Failed to resolve skill root: {}", root.display()))?;
    let canonical_dir = fs::canonicalize(base_dir)
        .with_context(|| format!("Failed to resolve skill directory: {}", base_dir.display()))?;
    if canonical_dir == canonical_root {
        return Ok(None);
    }
    if canonical_dir.starts_with(&canonical_root) {
        return Ok(Some(canonical_dir));
    }
    Ok(None)
}

fn resolve_listed_skill_dir(
    name: &str,
    roots: &SkillRoots,
    location_kind: OpenClawSkillLocationKind,
) -> Option<PathBuf> {
    match location_kind {
        OpenClawSkillLocationKind::WorkspaceLocal => roots
            .workspace_root
            .as_ref()
            .map(|root| root.join(name))
            .filter(|path| path.is_dir() && path.join("SKILL.md").is_file()),
        OpenClawSkillLocationKind::SharedLocal => roots
            .shared_root
            .as_ref()
            .map(|root| root.join(name))
            .filter(|path| path.is_dir() && path.join("SKILL.md").is_file()),
        OpenClawSkillLocationKind::Bundled | OpenClawSkillLocationKind::External => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::{
        create_local_skill_in_root, extract_json_value, parse_clawhub_search_results,
        resolve_clawhub_install_cwd, resolve_deletable_skill_dir, CliSkillInventory,
        LocalSkillLocation, SkillRoots,
    };
    use crate::types::OpenClawSkillLocationKind;

    #[test]
    fn extracts_json_payload_after_warning_prefix() {
        let payload = r#"Config warnings:
- plugins.entries.feishu: duplicate id
[skills] Skipping skill path that resolves outside its configured root.
{
  "summary": {
    "total": 2
  }
}"#;

        let value = extract_json_value(payload).expect("json payload should be extracted");
        assert_eq!(value["summary"]["total"].as_u64(), Some(2));
    }

    #[test]
    fn extracts_json_payload_before_trailing_plugin_logs() {
        let payload = r#"[
  {
    "id": "main"
  }
]
[plugins] feishu_doc: Registered feishu_doc, feishu_app_scopes
[plugins] feishu_chat: Registered feishu_chat tool
"#;

        let value = extract_json_value(payload).expect("json payload should be extracted");
        assert_eq!(value[0]["id"].as_str(), Some("main"));
    }

    #[test]
    fn rejects_skill_delete_targets_outside_allowed_roots() {
        let temp = tempdir().expect("temp dir");
        let workspace_root = temp.path().join("workspace").join("skills");
        let shared_root = temp.path().join("shared");
        let external_root = temp.path().join("external");
        fs::create_dir_all(&workspace_root).expect("workspace root");
        fs::create_dir_all(&shared_root).expect("shared root");
        fs::create_dir_all(&external_root).expect("external root");

        let external_skill = external_root.join("rogue-skill");
        fs::create_dir_all(&external_skill).expect("external skill");

        let error = resolve_deletable_skill_dir(
            &external_skill,
            &SkillRoots {
                workspace_root: Some(workspace_root),
                shared_root: Some(shared_root),
            },
        )
        .expect_err("external directories must be rejected");

        assert!(error
            .to_string()
            .contains("outside the managed local skill roots"));
    }

    #[test]
    fn creates_skill_starter_and_rejects_duplicates_or_invalid_names() {
        let temp = tempdir().expect("temp dir");
        let shared_root = temp.path().join("shared");
        fs::create_dir_all(&shared_root).expect("shared root");

        let created = create_local_skill_in_root(&shared_root, "demo-skill")
            .expect("starter skill should be created");
        assert_eq!(created, shared_root.join("demo-skill"));

        let skill_md = created.join("SKILL.md");
        let skill_md_text = fs::read_to_string(&skill_md).expect("starter file");
        assert!(skill_md_text.contains("name: demo-skill"));

        let duplicate_error = create_local_skill_in_root(&shared_root, "demo-skill")
            .expect_err("duplicate name should fail");
        assert!(duplicate_error.to_string().contains("already exists"));

        let invalid_error = create_local_skill_in_root(&shared_root, "../escape")
            .expect_err("invalid name should fail");
        assert!(invalid_error.to_string().contains("lowercase letters"));
    }

    #[test]
    fn allows_deleting_skills_inside_workspace_or_shared_roots() {
        let temp = tempdir().expect("temp dir");
        let workspace_root = temp.path().join("workspace").join("skills");
        let shared_root = temp.path().join("shared");
        fs::create_dir_all(&workspace_root).expect("workspace root");
        fs::create_dir_all(&shared_root).expect("shared root");

        let workspace_skill = workspace_root.join("workspace-skill");
        let shared_skill = shared_root.join("shared-skill");
        fs::create_dir_all(&workspace_skill).expect("workspace skill");
        fs::create_dir_all(&shared_skill).expect("shared skill");

        let (workspace_dir, workspace_kind) = resolve_deletable_skill_dir(
            &workspace_skill,
            &SkillRoots {
                workspace_root: Some(workspace_root.clone()),
                shared_root: Some(shared_root.clone()),
            },
        )
        .expect("workspace skill should be deletable");
        assert_eq!(
            workspace_dir,
            fs::canonicalize(&workspace_skill).expect("canonical workspace skill")
        );
        assert_eq!(workspace_kind, LocalSkillLocation::WorkspaceLocal);

        let (shared_dir, shared_kind) = resolve_deletable_skill_dir(
            &shared_skill,
            &SkillRoots {
                workspace_root: Some(workspace_root),
                shared_root: Some(shared_root),
            },
        )
        .expect("shared skill should be deletable");
        assert_eq!(
            shared_dir,
            fs::canonicalize(&shared_skill).expect("canonical shared skill")
        );
        assert_eq!(shared_kind, LocalSkillLocation::SharedLocal);
    }

    #[test]
    fn parses_clawhub_search_output_with_progress_prefix() {
        let output =
            "- Searching\ncalendar  Calendar  (3.724)\nfeishu-calendar  feishu-calendar  (3.641)\n";

        let results =
            parse_clawhub_search_results(output).expect("search results should be parsed");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].slug, "calendar");
        assert_eq!(results[0].title, "Calendar");
        assert_eq!(results[0].score, Some(3.724));
        assert_eq!(results[1].slug, "feishu-calendar");
    }

    #[test]
    fn resolves_clawhub_install_cwds_for_workspace_and_shared_roots() {
        let inventory = CliSkillInventory {
            workspace_dir: Some("/tmp/demo-workspace".into()),
            managed_skills_dir: Some("/tmp/.openclaw/skills".into()),
            skills: Vec::new(),
        };

        let workspace_cwd =
            resolve_clawhub_install_cwd(&inventory, OpenClawSkillLocationKind::WorkspaceLocal)
                .expect("workspace cwd");
        let shared_cwd =
            resolve_clawhub_install_cwd(&inventory, OpenClawSkillLocationKind::SharedLocal)
                .expect("shared cwd");

        assert_eq!(workspace_cwd, PathBuf::from("/tmp/demo-workspace"));
        assert_eq!(shared_cwd, PathBuf::from("/tmp/.openclaw"));
    }

    #[test]
    fn rejects_missing_roots_for_clawhub_install_resolution() {
        let missing_workspace = CliSkillInventory {
            workspace_dir: None,
            managed_skills_dir: Some("/tmp/.openclaw/skills".into()),
            skills: Vec::new(),
        };
        let workspace_error = resolve_clawhub_install_cwd(
            &missing_workspace,
            OpenClawSkillLocationKind::WorkspaceLocal,
        )
        .expect_err("missing workspace should fail");
        assert!(workspace_error.to_string().contains("workspace directory"));

        let missing_shared = CliSkillInventory {
            workspace_dir: Some("/tmp/demo-workspace".into()),
            managed_skills_dir: None,
            skills: Vec::new(),
        };
        let shared_error =
            resolve_clawhub_install_cwd(&missing_shared, OpenClawSkillLocationKind::SharedLocal)
                .expect_err("missing shared root should fail");
        assert!(shared_error.to_string().contains("shared skills directory"));
    }
}
