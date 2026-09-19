# battler-bug-reporter

A lightweight C# (.NET) ASP.NET Core Minimal API service that accepts diagnostic bug reports from `battler-web-app` and automatically creates structured issues (and Gists for large payloads) on GitHub.

## Local Development & Validation

### 1. Run the service
```bash
# Optional: Set real token, or omit for simulated dry-run mode
export GITHUB_TOKEN="ghp_yourTokenHere"

dotnet run
```
By default, the server runs on `http://localhost:5000` (or `http://localhost:5143`).

### 2. Verify Health
```bash
curl http://localhost:5000/health
```

### 3. Send a Test Bug Report
```bash
curl -X POST http://localhost:5000/api/report-bug \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Test Bug Report",
    "description": "Verifying local C# reporter integration",
    "environment": {
      "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
      "viewport": "1920x1080"
    },
    "appState": {
      "currentView": "lobby",
      "connectionStatus": "connected"
    }
  }'
```
* If `GITHUB_TOKEN` is **omitted or empty**, the service operates in **Soft-Launch Local Preview mode**:
  * Saves report markdown and diagnostic JSON locally to `reports/`.
  * Renders a GitHub issue preview at `http://localhost:5000/reports/{id}`.
  * Serves the raw diagnostic payload at `http://localhost:5000/reports/{id}/json`.
* If `GITHUB_TOKEN` is **set**, the service operates in **Live GitHub mode**:
  * Files real issues to GitHub using Octokit.
  * Uploads large diagnostic JSON dumps to secret Gists linked in the issue.

## Deployment to Google Cloud Run

Deploy directly from source using the GCP CLI:
```bash
gcloud run deploy battler-bug-reporter \
  --source . \
  --region us-central1 \
  --allow-unauthenticated \
  --set-secrets GITHUB_TOKEN=battler-github-token:latest
```
When `GITHUB_TOKEN` is injected as a secret, the service automatically runs in Live GitHub mode.

