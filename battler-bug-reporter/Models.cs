using System.Text.Json;
using System.Text.Json.Serialization;

namespace BugReporter;

public record BugReportRequest(
    string Title,
    string Description,
    EnvironmentInfo? Environment = null,
    string? View = null,
    AppStateInfo? AppState = null,
    ReactCrashInfo? ReactCrash = null,
    JsonElement? BattleDebug = null
)
{
    [JsonExtensionData]
    public Dictionary<string, JsonElement>? ExtensionData { get; set; }
}

public record ReactCrashInfo(
    string? Message = null,
    string? Stack = null,
    string? ComponentStack = null
);

public record EnvironmentInfo(string? UserAgent, string? Viewport);
public record AppStateInfo(string? CurrentView, string? ConnectionStatus = null);

public class GitHubOptions
{
    public const string SectionName = "GitHub";
    public string RepoOwner { get; set; } = "jackson-nestelroad";
    public string RepoName { get; set; } = "battler";
}

public record StoredBugReport(
    string ReportId,
    int ReportNumber,
    string Title,
    string Body,
    List<string> Labels,
    string AttachmentFileName,
    string AttachmentUrl,
    string TargetRepo,
    string CreatedAt,
    BugReportRequest Request
);
