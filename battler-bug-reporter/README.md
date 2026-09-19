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
| `RATE_LIMIT_GLOBAL_PERMIT_LIMIT` | `RateLimiting:GlobalPermitLimit` | `30` | Max total bug submissions allowed per hour across all clients combined. |
| `RATE_LIMIT_GLOBAL_WINDOW_HOURS` | `RateLimiting:GlobalWindowHours` | `1` | Sliding window duration in hours for global rate limiting. |
| `RATE_LIMIT_GLOBAL_CONCURRENCY_LIMIT` | `RateLimiting:GlobalConcurrencyLimit` | `2` | Max concurrent in-flight bug report requests to prevent GitHub abuse bursts. |
| `RATE_LIMIT_IP_PERMIT_LIMIT` | `RateLimiting:IpPermitLimit` | `10` | Max submissions allowed per hour per individual client IP. |
| `RATE_LIMIT_IP_WINDOW_HOURS` | `RateLimiting:IpWindowHours` | `1` | Sliding window duration in hours for per-IP rate limiting. |

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

* **Validation & Abuse Protection**:
  * `title`: Required, maximum 250 characters. Newlines stripped and normalized to `[Bug]: <Title>`.
  * `description`: Required, maximum 65,536 characters.
  * **Layered Rate Limiting**:
    1. **Global Concurrency Limiter**: Caps simultaneous in-flight requests (default: 2) to protect against GitHub secondary abuse bursts.
    2. **Global Sliding Window**: Caps total submissions across all clients combined (default: 30 / hour) to defend against distributed IP botnets.
    3. **Per-IP Sliding Window**: Caps requests per individual client IP (default: 10 / hour). Supports `X-Forwarded-For` for reverse proxies.
  * **Rejection Response**: Returns `HTTP 429 Too Many Requests` with a structured JSON payload:
    ```json
    {
      "error": "Too Many Requests",
      "detail": "Bug reporting rate limit exceeded. Please slow down and try again later."
    }
    ```
  * **GitHub Secondary Abuse Handling**: Catches Octokit `AbuseException` and `RateLimitExceededException` to return clean `429` responses rather than server crashes.

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
├── Program.cs                  # ASP.NET Core Minimal API routing, CORS, and endpoint registration
├── RateLimiterConfig.cs        # Layered rate limiting (global concurrency, global sliding window, per-IP)
├── Models.cs                   # Strongly-typed records (BugReportRequest, StoredBugReport, etc.)
├── IssueBuilder.cs             # Pure domain logic (title normalization, tag heuristics, markdown template)
├── ReportStore.cs              # Safe local report persistence with path traversal guards
├── ReportPreviewRenderer.cs    # HTML/CSS presentation layer for soft-launch preview (DOMPurify sanitized)
├── JsonDefaults.cs             # High-performance static pre-warmed JsonSerializerOptions
├── BugReporter.csproj          # .NET 10 project definition with Octokit dependency
└── Dockerfile                  # Multi-stage container build optimized for Cloud Run
```

---

## Production Deployment & Security Hardening

Deploy directly from source using the Google Cloud CLI with defensive autoscaling guardrails:

```bash
gcloud run deploy battler-bug-reporter \
  --source . \
  --region us-central1 \
  --allow-unauthenticated \
  --max-instances 2 \
  --concurrency 40 \
  --cpu-throttling \
  --memory 512Mi \
  --cpu 1 \
  --set-secrets GITHUB_TOKEN=battler-github-token:latest
```

### Production Security Checklist

1. **Denial-of-Wallet & Horizontal Scaling Protection**:
   * `--max-instances 2`: Prevents rogue traffic from spinning up dozens of containers and inflating GCP billing.
   * `--cpu-throttling`: Ensures CPU is only allocated during request processing, keeping idle costs at $0.
2. **Reverse Proxy & Forwarded Headers**:
   * The container automatically processes `X-Forwarded-For` and `X-Forwarded-Proto` behind Google Cloud Run load balancers.
3. **Edge WAF & Reverse Proxy (Recommended for Public Production)**:
   * Route public traffic through **Cloudflare** (or GCP Cloud Armor) with an edge rate-limiting rule (e.g. 5 req/min on `/api/report-bug`).
   * Edge filtering prevents unauthorized volumetric requests from ever reaching Cloud Run instances.
4. **GitHub Token Permissions**:
   * When creating the `GITHUB_TOKEN` secret in Google Secret Manager, use a dedicated Personal Access Token (fine-grained) or GitHub App token restricted strictly to:
     * **Repository**: `jackson-nestelroad/battler`
     * **Permissions**: `Issues: Read and Write`, `Gists: Read and Write`
     * No administration, code write, or workflow access.
