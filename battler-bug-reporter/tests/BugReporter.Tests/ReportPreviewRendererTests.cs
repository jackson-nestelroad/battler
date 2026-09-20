using BugReporter;

using Xunit;

namespace BugReporter.Tests;

public class ReportPreviewRendererTests
{
    [Fact]
    public void Render_ProducesHtmlWithEscapedContentAndTags()
    {
        var request = new BugReportRequest("XSS test", "Testing preview rendering with HTML special characters");
        var report = new StoredBugReport(
            "1a2b3c4d",
            42,
            "[Bug]: <script>alert('xss')</script> Critical Issue",
            "This is the **markdown** body.",
            new List<string> { "bug", "web-app", "battle" },
            "battler-debug-1a2b3c4d.json",
            "/reports/1a2b3c4d/json",
            "jackson-nestelroad/battler",
            "2026-09-19T00:00:00Z",
            request
        );

        var html = ReportPreviewRenderer.Render(report);

        // Document structure
        Assert.Contains("<!DOCTYPE html>", html);
        Assert.Contains("<html", html);
        Assert.Contains("</html>", html);

        // HTML escaping verification
        Assert.DoesNotContain("<script>alert('xss')</script>", html);
        Assert.Contains("&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;", html);

        // Tags and classes
        Assert.Contains("label-bug", html);
        Assert.Contains("label-web-app", html);
        Assert.Contains("label-battle", html);

        // Verification panel
        Assert.Contains("GitHub Dispatch Verification", html);
        Assert.Contains("POST https://api.github.com/repos/jackson-nestelroad/battler/issues", html);
        Assert.Contains("battler-debug-1a2b3c4d.json", html);
    }
}
