use extism_pdk::*;
use magi_pdk::DataType;
use serde_json::json;

// =============================================================================
// Plugin exports
// =============================================================================

#[plugin_fn]
pub fn describe() -> FnResult<Json<DataType>> {
    Ok(Json(DataType::from_json(json!({
        "name": "github",
        "version": "0.1.0",
        "description": "GitHub API integration for repos, issues, PRs, and code search",
        "label": "mcp",
        "tools": [
            {"name": "list_repos", "description": "List repositories for a user or org"},
            {"name": "get_repo", "description": "Get repository details"},
            {"name": "list_issues", "description": "List issues for a repository"},
            {"name": "create_issue", "description": "Create a new issue"},
            {"name": "list_prs", "description": "List pull requests for a repository"},
            {"name": "get_pr", "description": "Get pull request details"},
            {"name": "get_file", "description": "Get file contents from a repository"},
            {"name": "search_code", "description": "Search code across repositories"}
        ]
    }))))
}

#[plugin_fn]
pub fn config_schema() -> FnResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "type": "object",
        "properties": {
            "github_token": {
                "type": "string",
                "description": "GitHub personal access token"
            },
            "default_owner": {
                "type": "string",
                "description": "Default repository owner (user or org)"
            }
        },
        "required": ["github_token"]
    })))
}

#[plugin_fn]
pub fn init(Json(input): Json<DataType>) -> FnResult<Json<DataType>> {
    let config = input.get("config").cloned().unwrap_or(DataType::Null);
    if config.get("github_token").and_then(|t| t.as_str()).is_none() {
        return Ok(Json(DataType::from_json(json!({"error": "github_token is required"}))));
    }
    magi_pdk::log_info("GitHub plugin initialized");
    Ok(Json(DataType::from_json(json!({"success": true}))))
}

#[plugin_fn]
pub fn process(Json(input): Json<DataType>) -> FnResult<Json<DataType>> {
    let tool = input
        .get("tool")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let args = input.get("args").cloned().unwrap_or(DataType::Null);

    let config = magi_pdk::get_config().unwrap_or_default();
    let token = config
        .get("github_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match tool.as_str() {
        "list_repos" => list_repos(token, &args),
        "get_repo" => get_repo(token, &args),
        "list_issues" => list_issues(token, &args),
        "create_issue" => create_issue(token, &args),
        "list_prs" => list_prs(token, &args),
        "get_pr" => get_pr(token, &args),
        "get_file" => get_file(token, &args),
        "search_code" => search_code(token, &args),
        _ => Ok(Json(DataType::from_json(json!({"error": format!("unknown tool: {tool}")})))),
    }
}

// =============================================================================
// GitHub API helpers
// =============================================================================

fn github_get(token: &str, path: &str) -> Result<serde_json::Value, Error> {
    let url = if path.starts_with("https://") {
        path.to_string()
    } else {
        format!("https://api.github.com{path}")
    };
    let req = HttpRequest::new(&url)
        .with_header("Authorization", &format!("Bearer {token}"))
        .with_header("Accept", "application/vnd.github+json")
        .with_header("User-Agent", "magi-github-plugin/0.1")
        .with_header("X-GitHub-Api-Version", "2022-11-28");
    let resp = http::request::<String>(&req, None::<String>)?;
    serde_json::from_slice(&resp.body()).map_err(|e| Error::msg(format!("JSON parse error: {e}")))
}

fn github_post(token: &str, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, Error> {
    let url = format!("https://api.github.com{path}");
    let req = HttpRequest::new(&url)
        .with_method("POST")
        .with_header("Authorization", &format!("Bearer {token}"))
        .with_header("Accept", "application/vnd.github+json")
        .with_header("User-Agent", "magi-github-plugin/0.1")
        .with_header("X-GitHub-Api-Version", "2022-11-28")
        .with_header("Content-Type", "application/json");
    let body_str = serde_json::to_string(body)?;
    let resp = http::request::<String>(&req, Some(body_str))?;
    serde_json::from_slice(&resp.body()).map_err(|e| Error::msg(format!("JSON parse error: {e}")))
}

// =============================================================================
// Tool implementations
// =============================================================================

fn list_repos(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let path = if owner.is_empty() {
        "/user/repos?per_page=30&sort=updated".to_string()
    } else {
        format!("/users/{owner}/repos?per_page=30&sort=updated")
    };
    let data = github_get(token, &path)?;
    Ok(Json(DataType::from_json(data)))
}

fn get_repo(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    if owner.is_empty() || repo.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "owner and repo are required"}))));
    }
    let data = github_get(token, &format!("/repos/{owner}/{repo}"))?;
    Ok(Json(DataType::from_json(data)))
}

fn list_issues(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
    if owner.is_empty() || repo.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "owner and repo are required"}))));
    }
    let data = github_get(token, &format!("/repos/{owner}/{repo}/issues?state={state}&per_page=30"))?;
    Ok(Json(DataType::from_json(data)))
}

fn create_issue(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let body_text = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
    if owner.is_empty() || repo.is_empty() || title.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "owner, repo, and title are required"}))));
    }
    let body = json!({"title": title, "body": body_text});
    let data = github_post(token, &format!("/repos/{owner}/{repo}/issues"), &body)?;
    Ok(Json(DataType::from_json(data)))
}

fn list_prs(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
    if owner.is_empty() || repo.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "owner and repo are required"}))));
    }
    let data = github_get(token, &format!("/repos/{owner}/{repo}/pulls?state={state}&per_page=30"))?;
    Ok(Json(DataType::from_json(data)))
}

fn get_pr(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let number = args
        .get("number")
        .map(|v| v.to_json().to_string())
        .unwrap_or_default();
    if owner.is_empty() || repo.is_empty() || number.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "owner, repo, and number are required"}))));
    }
    // Strip quotes if the number was a string
    let num = number.trim_matches('"');
    let data = github_get(token, &format!("/repos/{owner}/{repo}/pulls/{num}"))?;
    Ok(Json(DataType::from_json(data)))
}

fn get_file(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let branch = args.get("branch").and_then(|v| v.as_str()).unwrap_or("main");
    if owner.is_empty() || repo.is_empty() || path.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "owner, repo, and path are required"}))));
    }
    let data = github_get(token, &format!("/repos/{owner}/{repo}/contents/{path}?ref={branch}"))?;
    Ok(Json(DataType::from_json(data)))
}

fn search_code(token: &str, args: &DataType) -> FnResult<Json<DataType>> {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    if query.is_empty() {
        return Ok(Json(DataType::from_json(json!({"error": "query is required"}))));
    }
    let encoded = query.replace(' ', "+");
    let data = github_get(token, &format!("/search/code?q={encoded}&per_page=20"))?;
    Ok(Json(DataType::from_json(data)))
}
