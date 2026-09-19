using System.Text.Json;

using BugReporter;

using Microsoft.Extensions.Logging.Abstractions;

using Xunit;

namespace BugReporter.Tests;

public class ReportStoreTests : IDisposable
{
    private readonly string _tempDirectory;
    private readonly ReportStore _store;

    public ReportStoreTests()
    {
        _tempDirectory = Path.Combine(Path.GetTempPath(), "BugReporterTests_" + Guid.NewGuid().ToString("N"));
        _store = new ReportStore(_tempDirectory, NullLogger<ReportStore>.Instance);
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDirectory))
        {
            try
            {
                Directory.Delete(_tempDirectory, recursive: true);
            }
            catch
            {
                // Ignore cleanup errors in temporary test directories
            }
        }
    }

    [Fact]
    public async Task SaveReportAsync_WritesFilesSuccessfully()
    {
        var reportId = "a1b2c3d4";
        var request = new BugReportRequest("Test issue", "Testing save");
        var report = new StoredBugReport(
            reportId,
            1,
            "[Bug]: Test issue",
            "# Issue Details",
            new List<string> { "bug", "web-app" },
            $"battler-debug-{reportId}.json",
            $"/reports/{reportId}/json",
            "owner/repo",
            DateTime.UtcNow.ToString("o"),
            request
        );
        var rawJson = "{\"test\": true}";

        await _store.SaveReportAsync(report, rawJson);

        Assert.True(File.Exists(Path.Combine(_tempDirectory, $"bug-{reportId}.md")));
        Assert.True(File.Exists(Path.Combine(_tempDirectory, $"battler-debug-{reportId}.json")));
        Assert.True(File.Exists(Path.Combine(_tempDirectory, $"report-{reportId}.json")));

        var retrieved = await _store.GetReportAsync(reportId, "owner/repo");
        Assert.NotNull(retrieved);
        Assert.Equal(reportId, retrieved.ReportId);
        Assert.Equal("[Bug]: Test issue", retrieved.Title);
        Assert.Equal("owner/repo", retrieved.TargetRepo);
    }

    [Fact]
    public async Task GetReportAsync_WithNonExistentId_ReturnsNull()
    {
        var result = await _store.GetReportAsync("ffffffff", "owner/repo");
        Assert.Null(result);
    }

    [Theory]
    [InlineData("../../../etc/passwd")]
    [InlineData("..\\..\\secret.txt")]
    [InlineData("sub/dir/test")]
    [InlineData("id with spaces")]
    [InlineData("")]
    public async Task GetReportAsync_WithInvalidOrTraversalId_ReturnsNull(string maliciousId)
    {
        var result = await _store.GetReportAsync(maliciousId, "owner/repo");
        Assert.Null(result);
    }

    [Fact]
    public async Task GetRawJsonAsync_RetrievesStoredContent()
    {
        var reportId = "e5f60718";
        var request = new BugReportRequest("Raw JSON", "Testing raw json retrieval");
        var report = new StoredBugReport(
            reportId,
            1,
            "[Bug]: Raw JSON check",
            "Body",
            new List<string> { "bug" },
            $"battler-debug-{reportId}.json",
            $"/reports/{reportId}/json",
            "owner/repo",
            DateTime.UtcNow.ToString("o"),
            request
        );
        var expectedJson = "{\"engine\": \"battler\", \"status\": \"ok\"}";

        await _store.SaveReportAsync(report, expectedJson);

        var retrievedJson = await _store.GetRawJsonAsync(reportId);
        Assert.Equal(expectedJson, retrievedJson);
    }

    [Theory]
    [InlineData("../passwd")]
    [InlineData("..\\secret")]
    [InlineData("")]
    public async Task GetRawJsonAsync_WithTraversalOrInvalidId_ReturnsNull(string maliciousId)
    {
        var result = await _store.GetRawJsonAsync(maliciousId);
        Assert.Null(result);
    }
}
