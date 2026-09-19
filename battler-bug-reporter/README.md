# battler-bug-reporter

A lightweight, decoupled C# (.NET 10) ASP.NET Core Minimal API service that accepts diagnostic bug reports from `battler-web-app` and automatically creates structured GitHub issues (and unlisted Gists for diagnostic dumps).

## Dual Execution Modes

The service automatically switches modes depending on whether a GitHub token is configured:

| Mode | Trigger | Behavior |
| :--- | :--- | :--- |
| **Soft-Launch Local Preview** | `GITHUB_TOKEN` is unset or empty | Saves reports locally to `reports/`, serves a GitHub dark-mode preview at `/reports/{id}`, and serves raw JSON at `/reports/{id}/json`. No external GitHub API calls are made. |
| **Live GitHub** | `GITHUB_TOKEN` is set | Uses Octokit to create an unlisted GitHub Gist containing `battler-debug-{id}.json` and files a formatted issue under `[Bug]: <Title>` with color-coded label tags. |

---

## Configuration

Settings can be provided via environment variables or `appsettings.json`:

| Environment Variable | Config Key | Default | Description |
| :--- | :--- | :--- | :--- |
| `GITHUB_TOKEN` | `GitHub:Token` | *(None)* | GitHub Personal Access Token or App Installation Token. If omitted, runs in Soft-Launch mode. |
| `GITHUB_REPO_OWNER` | `GitHub:RepoOwner` | `jackson-nestelroad` | Owner / Organization of target repository. |
| `GITHUB_REPO_NAME` | `GitHub:RepoName` | `battler` | Name of target repository. |
| `REPORTS_DIRECTORY` | `ReportsDirectory` | `./reports` | Local storage directory for soft-launch reports and diagnostics. |

---

## API Endpoints

### 1. Health Check
```http
GET /health
```
**Response:**
```json
{
  "status": "healthy",
  "service": "battler-bug-reporter",
  "mode": "soft-launch-local",
  "targetRepo": "jackson-nestelroad/battler",
  "timestamp": "2026-09-19T04:13:31.619776Z"
}
```

---

### 2. Submit Bug Report
```http
POST /api/report-bug
Content-Type: application/json
```

**Payload Schema:**
```json
{
  "title": "Player cannot switch Pokemon during turn 4",
  "description": "When selecting switch on turn 4, the UI froze and choices were disabled.",
  "view": "battle",
  "environment": {
    "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
    "viewport": "1920x1080"
  },
  "battleDebug": {
    "battleId": "6f594a4a-6010-4408-82f8-351d8e2783df"
  }
}
```

* **Validation**:
  * `title`: Required, maximum 250 characters. Newlines stripped and normalized to `[Bug]: <Title>`.
  * `description`: Required, maximum 65,536 characters.
  * `rate-limiting`: 10 requests per hour per client IP (sliding window). Supports `X-Forwarded-For` for reverse proxies.

**Response (Soft-Launch Mode):**
```json
{
  "success": true,
  "issueUrl": "http://localhost:5000/reports/b8094722",
  "issueNumber": 1,
  "reportId": "b8094722",
  "labels": ["bug", "web-app", "battle"],
  "softLaunch": true,
  "message": "[Soft Launch] Saved locally to reports/bug-b8094722.md"
}
```

**Response (Live GitHub Mode):**
```json
{
  "success": true,
  "issueUrl": "https://github.com/jackson-nestelroad/battler/issues/42",
  "issueNumber": 42,
  "reportId": "b8094722",
  "labels": ["bug", "web-app", "battle"],
  "softLaunch": false
}
```

---

### 3. Soft-Launch Preview Endpoints *(Soft-Launch Mode Only)*

* **`GET /reports/{id}`**: Renders a GitHub-styled dark mode preview showing the issue title, status badge, tags, rendered Markdown body, and an interactive **GitHub Dispatch Verification** panel containing the exact Octokit request payload.
* **`GET /reports/{id}/json`**: Serves the raw diagnostic dump (`battler-debug-{id}.json`).

> [!NOTE]
> Report IDs are constrained by route regex to 8-character hex strings (`^[a-fA-F0-9]{8}$`), and filesystem access is guarded against directory traversal.

---

## Local Development

```bash
# 1. Start the service (runs on port 5000 by default)
dotnet run --urls "http://localhost:5000"

# 2. Verify health
curl http://localhost:5000/health

# 3. Submit a test report
curl -X POST http://localhost:5000/api/report-bug \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Local Test Bug",
    "description": "Verifying local C# bug reporter integration.",
    "view": "lobby",
    "environment": {
      "userAgent": "TestAgent/1.0",
      "viewport": "1920x1080"
    }
  }'
```

---

## Project Structure

```
battler-bug-reporter/
├── Program.cs                  # ASP.NET Core Minimal API routing, CORS, and rate limiting
├── Models.cs                   # Strongly-typed records (BugReportRequest, StoredBugReport, etc.)
├── IssueBuilder.cs             # Pure domain logic (title normalization, tag heuristics, markdown template)
├── ReportStore.cs              # Safe local report persistence with path traversal guards
├── ReportPreviewRenderer.cs    # HTML/CSS presentation layer for soft-launch preview (DOMPurify sanitized)
├── JsonDefaults.cs             # High-performance static pre-warmed JsonSerializerOptions
├── BugReporter.csproj          # .NET 10 project definition with Octokit dependency
└── Dockerfile                  # Multi-stage container build optimized for Cloud Run
```

---

## Deployment to Google Cloud Run

Deploy directly from source using the GCP CLI:
```bash
gcloud run deploy battler-bug-reporter \
  --source . \
  --region us-central1 \
  --allow-unauthenticated \
  --set-secrets GITHUB_TOKEN=battler-github-token:latest
```

* The container automatically handles `ForwardedHeaders` (`X-Forwarded-For`, `X-Forwarded-Proto`) behind Google Cloud Run load balancers.
* Injecting `GITHUB_TOKEN` automatically enables Live GitHub filing mode.
