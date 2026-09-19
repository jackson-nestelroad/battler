using System.Text.Json;

namespace BugReporter;

public static class IssueBuilder
{
    public static string FormatIssueTitle(string rawTitle)
    {
        var singleLine = rawTitle.Replace("\r", " ").Replace("\n", " ").Trim();
        while (singleLine.Contains("  "))
        {
            singleLine = singleLine.Replace("  ", " ");
        }

        if (singleLine.StartsWith("[Bug]:", StringComparison.OrdinalIgnoreCase))
        {
            var content = singleLine["[Bug]:".Length..].Trim();
            return $"[Bug]: {content}";
        }

        return $"[Bug]: {singleLine}";
    }

    public static List<string> DetermineLabels(BugReportRequest request)
    {
        var labels = new List<string> { "bug", "web-app" };

        if (request.ReactCrash != null)
        {
            labels.Add("crash");
        }

        if (request.BattleDebug is { ValueKind: JsonValueKind.Object })
        {
            labels.Add("battle");
        }

        var view = request.View ?? request.AppState?.CurrentView;
        var sanitizedView = SanitizeLabel(view);
        if (!string.IsNullOrEmpty(sanitizedView) && sanitizedView != "unknown" && !labels.Contains(sanitizedView))
        {
            labels.Add(sanitizedView);
        }

        return labels;
    }

    public static string BuildMarkdownBody(
        BugReportRequest request,
        string reportId,
        string serverTimestamp,
        string attachmentFileName,
        string attachmentUrl,
        string? embeddedFallbackJson = null)
    {
        var view = !string.IsNullOrWhiteSpace(request.View)
            ? request.View.Trim()
            : (!string.IsNullOrWhiteSpace(request.AppState?.CurrentView)
                ? request.AppState.CurrentView.Trim()
                : "unknown");

        var ua = !string.IsNullOrWhiteSpace(request.Environment?.UserAgent)
            ? request.Environment.UserAgent.Trim()
            : "unknown";

        var viewport = !string.IsNullOrWhiteSpace(request.Environment?.Viewport)
            ? request.Environment.Viewport.Trim()
            : "unknown";

        string? battleId = null;
        if (request.BattleDebug is { ValueKind: JsonValueKind.Object } battleDebug &&
            battleDebug.TryGetProperty("battleId", out var bId) &&
            bId.ValueKind == JsonValueKind.String)
        {
            battleId = bId.GetString();
        }

        var crashSection = "";
        if (request.ReactCrash != null)
        {
            var crashLines = new List<string>();
            if (!string.IsNullOrWhiteSpace(request.ReactCrash.Message))
            {
                crashLines.Add($"Error: {request.ReactCrash.Message.Trim()}");
            }
            if (!string.IsNullOrWhiteSpace(request.ReactCrash.Stack))
            {
                crashLines.Add(request.ReactCrash.Stack.Trim());
            }

            var crashStack = crashLines.Count > 0 ? string.Join("\n\n", crashLines) : "No stack trace provided.";

            var componentStackSection = !string.IsNullOrWhiteSpace(request.ReactCrash.ComponentStack)
                ? $"""

                <details>
                <summary>Component Stack</summary>

                ```text
                {request.ReactCrash.ComponentStack.Trim()}
                ```
                </details>
                """
                : "";

            crashSection = $"""


            <details open>
            <summary><b>React Component Crash</b></summary>

            ```text
            {crashStack}
            ```
            {componentStackSection}
            </details>
            """;
        }

        var battleSection = !string.IsNullOrWhiteSpace(battleId)
            ? $"\n* **Battle ID**: `{battleId}`"
            : "";

        var embeddedFallbackSection = !string.IsNullOrWhiteSpace(embeddedFallbackJson)
            ? $"""


            <details>
            <summary><b>Embedded Diagnostic Payload (Inline Backup)</b></summary>

            ```json
            {embeddedFallbackJson}
            ```
            </details>
            """
            : "";

        return $"""
        ### Summary
        {request.Description.Trim()}

        ### Environment & Context
        * **View**: `{view}`
        * **Browser/Platform**: `{ua}`
        * **Viewport**: `{viewport}`
        * **Report ID**: `{reportId}` | **Timestamp**: `{serverTimestamp}`{battleSection}{crashSection}

        ### Diagnostics
        📎 **Diagnostic Attachment**: [{attachmentFileName}]({attachmentUrl}){embeddedFallbackSection}
        """;
    }

    private static string? SanitizeLabel(string? raw)
    {
        if (string.IsNullOrWhiteSpace(raw))
        {
            return null;
        }

        var cleaned = new string(raw.Trim().ToLowerInvariant()
            .Where(c => char.IsLetterOrDigit(c) || c == '-' || c == '_')
            .ToArray());

        return cleaned.Length > 50 ? cleaned[..50] : (cleaned.Length > 0 ? cleaned : null);
    }
}
