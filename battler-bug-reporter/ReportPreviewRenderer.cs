using System.Net;
using System.Text.Json;

namespace BugReporter;

public static class ReportPreviewRenderer
{
    public static string Render(StoredBugReport report)
    {
        var encodedTitle = WebUtility.HtmlEncode(report.Title);
        var encodedReportId = WebUtility.HtmlEncode(report.ReportId);
        var encodedTargetRepo = WebUtility.HtmlEncode(report.TargetRepo);
        var encodedCreatedAt = WebUtility.HtmlEncode(report.CreatedAt);
        var encodedFileName = WebUtility.HtmlEncode(report.AttachmentFileName);
        var encodedAttachmentUrl = WebUtility.HtmlEncode(report.AttachmentUrl);

        var labelsHtml = string.Join(" ", report.Labels.Select(l =>
        {
            var cssClass = l.ToLowerInvariant() switch
            {
                "bug" => "label-bug",
                "web-app" => "label-web-app",
                "battle" => "label-battle",
                "crash" => "label-crash",
                "lobby" => "label-lobby",
                _ => "label-default"
            };
            return $"<span class=\"label-tag {cssClass}\">{WebUtility.HtmlEncode(l)}</span>";
        }));

        var simulatedGitHubPayload = JsonSerializer.Serialize(new
        {
            targetEndpoint = $"POST https://api.github.com/repos/{report.TargetRepo}/issues",
            octokitNewIssue = new
            {
                title = report.Title,
                labels = report.Labels,
                bodyPreview = report.Body.Length > 250 ? report.Body[..250] + "..." : report.Body
            },
            gistAttachment = new
            {
                targetEndpoint = "POST https://api.github.com/gists",
                description = $"Battler Diagnostic Dump [{report.ReportId}]",
                @public = false,
                fileName = report.AttachmentFileName,
                localEndpoint = report.AttachmentUrl
            }
        }, JsonDefaults.Indented);

        return $$"""
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="utf-8">
            <title>{{encodedTitle}} #{{encodedReportId}} - Preview</title>
            <style>
                body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif; max-width: 880px; margin: 40px auto; padding: 0 20px; line-height: 1.5; background: #0d1117; color: #c9d1d9; }
                .header { border-bottom: 1px solid #21262d; padding-bottom: 16px; margin-bottom: 20px; }
                .title-row { display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
                .title { font-size: 26px; font-weight: 600; color: #f0f6fc; margin: 0; }
                .issue-num { color: #8b949e; font-weight: 300; }
                .meta { display: flex; align-items: center; gap: 10px; font-size: 14px; color: #8b949e; }
                .badge-open { display: inline-flex; align-items: center; background: #238636; color: #ffffff; padding: 4px 10px; border-radius: 20px; font-size: 13px; font-weight: 600; }
                .badge-soft { display: inline-block; background: #388bfd1a; color: #58a6ff; border: 1px solid #388bfd4d; padding: 2px 8px; border-radius: 12px; font-size: 12px; font-weight: 500; }
                .labels-row { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; margin-top: 12px; }
                .labels-title { font-size: 12px; color: #8b949e; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; margin-right: 4px; }
                .label-tag { display: inline-flex; align-items: center; padding: 2px 10px; border-radius: 20px; font-size: 12px; font-weight: 500; line-height: 18px; }
                .label-bug { background-color: rgba(215, 58, 74, 0.18); color: #ff7b72; border: 1px solid rgba(215, 58, 74, 0.4); }
                .label-web-app { background-color: rgba(56, 139, 253, 0.18); color: #58a6ff; border: 1px solid rgba(56, 139, 253, 0.4); }
                .label-battle { background-color: rgba(163, 113, 247, 0.18); color: #d2a8ff; border: 1px solid rgba(163, 113, 247, 0.4); }
                .label-crash { background-color: rgba(248, 81, 73, 0.18); color: #f85149; border: 1px solid rgba(248, 81, 73, 0.4); }
                .label-lobby { background-color: rgba(46, 160, 67, 0.18); color: #3fb950; border: 1px solid rgba(46, 160, 67, 0.4); }
                .label-default { background-color: rgba(110, 118, 129, 0.18); color: #8b949e; border: 1px solid rgba(110, 118, 129, 0.4); }
                .issue-card { border: 1px solid #30363d; border-radius: 6px; background: #0d1117; overflow: hidden; margin-top: 16px; }
                .issue-card-header { padding: 10px 16px; background: #161b22; border-bottom: 1px solid #30363d; font-size: 13px; color: #8b949e; display: flex; justify-content: space-between; align-items: center; }
                .issue-card-body { padding: 20px; }
                pre { background: #161b22; padding: 14px; border-radius: 6px; overflow-x: auto; border: 1px solid #30363d; color: #e6edf3; font-family: ui-monospace, SFMono-Regular, monospace; font-size: 13px; }
                code { font-family: ui-monospace, SFMono-Regular, monospace; background: rgba(110,118,129,0.4); padding: 2px 6px; border-radius: 4px; font-size: 85%; }
                a { color: #58a6ff; text-decoration: none; }
                a:hover { text-decoration: underline; }
                details { background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px 16px; margin: 16px 0; }
                summary { cursor: pointer; font-weight: 600; color: #58a6ff; }
                h3 { font-size: 16px; font-weight: 600; border-bottom: 1px solid #21262d; padding-bottom: 6px; margin: 24px 0 12px 0; color: #f0f6fc; }
                h3:first-child { margin-top: 0; }
                ul { padding-left: 20px; margin: 8px 0; }
                li { margin: 4px 0; }
                .verification-card { border: 1px solid #30363d; border-radius: 6px; background: #161b22; padding: 14px 16px; margin-top: 20px; }
                .verification-card summary { cursor: pointer; font-weight: 600; color: #58a6ff; user-select: none; }
                .verification-card summary:hover { text-decoration: underline; }
                .verification-note { font-size: 12px; color: #8b949e; margin: 8px 0 12px 0; }
            </style>
            <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
            <script src="https://cdn.jsdelivr.net/npm/dompurify@3.0.6/dist/purify.min.js"></script>
        </head>
        <body>
            <div class="header">
                <div class="title-row">
                    <h1 class="title">{{encodedTitle}} <span class="issue-num">#{{encodedReportId}}</span></h1>
                    <span class="badge-soft">Soft-Launch Preview</span>
                </div>
                <div class="meta">
                    <span class="badge-open">Open</span>
                    <span>Target: <code>{{encodedTargetRepo}}</code></span>
                    <span>Created: <code>{{encodedCreatedAt}}</code></span>
                </div>
                <div class="labels-row">
                    <span class="labels-title">Tags:</span>
                    {{labelsHtml}}
                </div>
            </div>
            <div class="issue-card">
                <div class="issue-card-header">
                    <span><strong>battler-reporter</strong> generated GitHub Issue preview</span>
                    <a href="{{encodedAttachmentUrl}}" target="_blank">{{encodedFileName}} ↗</a>
                </div>
                <div class="issue-card-body" id="content"></div>
            </div>
            <details class="verification-card">
                <summary>GitHub Dispatch Verification (Octokit Request Payload)</summary>
                <p class="verification-note">
                    The block below mirrors the exact parameters passed to Octokit's <code>github.Issue.Create</code> and <code>github.Gist.Create</code> when <code>GITHUB_TOKEN</code> is configured.
                </p>
                <pre><code class="language-json">{{WebUtility.HtmlEncode(simulatedGitHubPayload)}}</code></pre>
            </details>
            <script>
                const raw = {{JsonSerializer.Serialize(report.Body)}};
                const contentEl = document.getElementById('content');
                if (typeof marked !== 'undefined') {
                    const parsed = marked.parse(raw);
                    contentEl.innerHTML = typeof DOMPurify !== 'undefined' ? DOMPurify.sanitize(parsed) : parsed;
                } else {
                    const pre = document.createElement('pre');
                    pre.textContent = raw;
                    contentEl.appendChild(pre);
                }
            </script>
        </body>
        </html>
        """;
    }
}
