using System.Text.Json;

using Microsoft.Extensions.Logging;

namespace BugReporter;

public class ReportStore
{
    private readonly string _reportsDirectory;
    private readonly string _normalizedReportsDir;
    private readonly ILogger<ReportStore> _logger;

    public ReportStore(string reportsDirectory, ILogger<ReportStore> logger)
    {
        _reportsDirectory = reportsDirectory;
        _normalizedReportsDir = Path.GetFullPath(reportsDirectory).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        _logger = logger;
        Directory.CreateDirectory(_reportsDirectory);
    }

    public async Task SaveReportAsync(StoredBugReport report, string rawJson)
    {
        var mdPath = GetSafePath($"bug-{report.ReportId}.md");
        var jsonPath = GetSafePath(report.AttachmentFileName);
        var metaPath = GetSafePath($"report-{report.ReportId}.json");

        if (mdPath == null || jsonPath == null || metaPath == null)
        {
            throw new InvalidOperationException($"Invalid report path for ID '{report.ReportId}'.");
        }

        await File.WriteAllTextAsync(mdPath, report.Body);
        await File.WriteAllTextAsync(jsonPath, rawJson);
        await File.WriteAllTextAsync(metaPath, JsonSerializer.Serialize(report, JsonDefaults.Indented));
    }

    public async Task<StoredBugReport?> GetReportAsync(string id, string defaultTargetRepo)
    {
        if (!IsValidId(id))
        {
            return null;
        }

        var metaFile = GetSafePath($"report-{id}.json");
        if (metaFile != null && File.Exists(metaFile))
        {
            try
            {
                var text = await File.ReadAllTextAsync(metaFile);
                return JsonSerializer.Deserialize<StoredBugReport>(text, JsonDefaults.Web);
            }
            catch (JsonException ex)
            {
                _logger.LogWarning(ex, "Corrupted report metadata in {MetaFile}", metaFile);
            }
            catch (IOException ex)
            {
                _logger.LogWarning(ex, "Failed to read report metadata in {MetaFile}", metaFile);
            }
        }

        var mdFile = GetSafePath($"bug-{id}.md");
        if (mdFile == null || !File.Exists(mdFile))
        {
            return null;
        }

        var mdContent = await File.ReadAllTextAsync(mdFile);
        var title = $"Bug Report #{id}";
        var labels = new List<string> { "bug", "web-app" };

        var jsonFile = GetSafePath($"battler-debug-{id}.json");
        if (jsonFile == null || !File.Exists(jsonFile))
        {
            jsonFile = GetSafePath($"bug-{id}.json");
        }

        if (jsonFile != null && File.Exists(jsonFile))
        {
            try
            {
                var rawJson = await File.ReadAllTextAsync(jsonFile);
                var req = JsonSerializer.Deserialize<BugReportRequest>(rawJson, JsonDefaults.Web);
                if (req != null)
                {
                    if (!string.IsNullOrWhiteSpace(req.Title))
                    {
                        title = IssueBuilder.FormatIssueTitle(req.Title);
                    }
                    labels = IssueBuilder.DetermineLabels(req);
                }
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to parse debug JSON from {JsonFile}", jsonFile);
            }
        }

        return new StoredBugReport(
            id,
            1,
            title,
            mdContent,
            labels,
            $"battler-debug-{id}.json",
            $"/reports/{id}/json",
            defaultTargetRepo,
            DateTime.UtcNow.ToString("o"),
            new BugReportRequest(title, mdContent, null, null, null, null, null)
        );
    }

    public async Task<string?> GetRawJsonAsync(string id)
    {
        if (!IsValidId(id))
        {
            return null;
        }

        var jsonFile = GetSafePath($"battler-debug-{id}.json");
        if (jsonFile == null || !File.Exists(jsonFile))
        {
            jsonFile = GetSafePath($"bug-{id}.json");
        }

        if (jsonFile == null || !File.Exists(jsonFile))
        {
            return null;
        }

        return await File.ReadAllTextAsync(jsonFile);
    }

    private static bool IsValidId(string id) =>
        !string.IsNullOrWhiteSpace(id) &&
        id.Length <= 32 &&
        id.All(c => char.IsLetterOrDigit(c) || c == '-');

    private string? GetSafePath(string filename)
    {
        if (string.IsNullOrWhiteSpace(filename) ||
            filename.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0 ||
            Path.GetFileName(filename) != filename)
        {
            return null;
        }

        var fullPath = Path.GetFullPath(Path.Combine(_reportsDirectory, filename));
        return fullPath.StartsWith(_normalizedReportsDir, StringComparison.OrdinalIgnoreCase)
            ? fullPath
            : null;
    }
}
