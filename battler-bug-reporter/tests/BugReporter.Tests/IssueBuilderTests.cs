using System.Text.Json;

using BugReporter;

using Xunit;

namespace BugReporter.Tests;

public class IssueBuilderTests
{
    [Fact]
    public void FormatIssueTitle_WhenLackingPrefix_AddsPrefix()
    {
        var result = IssueBuilder.FormatIssueTitle("Switching Pokemon crashed the UI");
        Assert.Equal("[Bug]: Switching Pokemon crashed the UI", result);
    }

    [Fact]
    public void FormatIssueTitle_WhenAlreadyPrefixed_PreservesPrefixWithoutDuplication()
    {
        var result = IssueBuilder.FormatIssueTitle("[Bug]: Switching Pokemon crashed the UI");
        Assert.Equal("[Bug]: Switching Pokemon crashed the UI", result);
    }

    [Fact]
    public void FormatIssueTitle_StripsNewlinesAndCollapsesSpaces()
    {
        var result = IssueBuilder.FormatIssueTitle("Title with\r\nnewlines and   extra  spaces");
        Assert.Equal("[Bug]: Title with newlines and extra spaces", result);
    }

    [Fact]
    public void DetermineLabels_BaseRequest_IncludesStandardLabels()
    {
        var request = new BugReportRequest(
            Title: "UI glitch",
            Description: "Something broke"
        );

        var labels = IssueBuilder.DetermineLabels(request);

        Assert.Contains("bug", labels);
        Assert.Contains("web-app", labels);
    }

    [Fact]
    public void DetermineLabels_WithCrash_IncludesCrashLabel()
    {
        var request = new BugReportRequest(
            Title: "Crash",
            Description: "Uncaught error",
            ReactCrash: new ReactCrashInfo(Message: "TypeError: undefined")
        );

        var labels = IssueBuilder.DetermineLabels(request);

        Assert.Contains("crash", labels);
    }

    [Fact]
    public void DetermineLabels_WithBattleDebug_IncludesBattleLabel()
    {
        using var doc = JsonDocument.Parse("{\"battleId\": \"b123\"}");
        var request = new BugReportRequest(
            Title: "Move missed unexpectedly",
            Description: "100% accuracy move missed",
            BattleDebug: doc.RootElement.Clone()
        );

        var labels = IssueBuilder.DetermineLabels(request);

        Assert.Contains("battle", labels);
    }

    [Fact]
    public void DetermineLabels_WithView_IncludesViewLabelAndDeduplicates()
    {
        using var doc = JsonDocument.Parse("{\"turn\": 5}");
        var request = new BugReportRequest(
            Title: "Battle error",
            Description: "Issue during battle",
            View: "battle",
            BattleDebug: doc.RootElement.Clone()
        );

        var labels = IssueBuilder.DetermineLabels(request);

        // 'battle' added by both BattleDebug and View, must only appear once
        Assert.Equal(1, labels.Count(l => l == "battle"));
    }

    [Fact]
    public void BuildMarkdownBody_RendersDescriptionAndEnvironment()
    {
        var request = new BugReportRequest(
            Title: "Test bug",
            Description: "Reproduce by clicking A then B.",
            Environment: new EnvironmentInfo(UserAgent: "Mozilla/5.0 TestBrowser", Viewport: "1920x1080"),
            View: "teams"
        );

        var body = IssueBuilder.BuildMarkdownBody(
            request,
            "12345678",
            "2026-09-19T00:00:00Z",
            "battler-debug-12345678.json",
            "https://gist.github.com/12345678"
        );

        Assert.Contains("Reproduce by clicking A then B.", body);
        Assert.Contains("`teams`", body);
        Assert.Contains("Mozilla/5.0 TestBrowser", body);
        Assert.Contains("1920x1080", body);
        Assert.Contains("📎 **Diagnostic Attachment**: [battler-debug-12345678.json](https://gist.github.com/12345678)", body);
    }

    [Fact]
    public void BuildMarkdownBody_WithReactCrash_RendersCrashSection()
    {
        var request = new BugReportRequest(
            Title: "Crash report",
            Description: "App crashed on launch",
            ReactCrash: new ReactCrashInfo(
                Message: "NullReferenceException: Object reference not set",
                Stack: "at Component.Render() in Component.tsx:line 42"
            )
        );

        var body = IssueBuilder.BuildMarkdownBody(
            request,
            "87654321",
            "2026-09-19T00:00:00Z",
            "battler-debug-87654321.json",
            "https://example.com/reports/87654321/json"
        );

        Assert.Contains("React Component Crash", body);
        Assert.Contains("NullReferenceException: Object reference not set", body);
        Assert.Contains("at Component.Render() in Component.tsx:line 42", body);
        Assert.Contains("📎 **Diagnostic Attachment**: [battler-debug-87654321.json](https://example.com/reports/87654321/json)", body);
    }
}
