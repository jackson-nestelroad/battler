using System.Text.Json;
using System.Threading.RateLimiting;

using BugReporter;

using Microsoft.AspNetCore.HttpOverrides;
using Microsoft.AspNetCore.RateLimiting;

using Octokit;

var builder = WebApplication.CreateBuilder(args);

// 1. Configuration
var githubToken = builder.Configuration["GITHUB_TOKEN"]?.Trim();
var isLiveGitHub = !string.IsNullOrWhiteSpace(githubToken);

var repoOwner = (builder.Configuration["GitHub:RepoOwner"]
    ?? builder.Configuration["GITHUB_REPO_OWNER"]
    ?? "jackson-nestelroad").Trim();

var repoName = (builder.Configuration["GitHub:RepoName"]
    ?? builder.Configuration["GITHUB_REPO_NAME"]
    ?? "battler").Trim();

// 2. Services
// Support reverse proxies (e.g. Google Cloud Run load balancers)
builder.Services.Configure<ForwardedHeadersOptions>(options =>
{
    options.ForwardedHeaders = ForwardedHeaders.XForwardedFor | ForwardedHeaders.XForwardedProto;
    options.KnownIPNetworks.Clear();
    options.KnownProxies.Clear();
});

builder.Services.AddCors(options =>
{
    options.AddDefaultPolicy(policy =>
    {
        policy.AllowAnyOrigin()
              .AllowAnyHeader()
              .AllowAnyMethod();
    });
});

// Sliding-window rate limiter per client IP (10 requests per hour)
builder.Services.AddRateLimiter(options =>
{
    options.RejectionStatusCode = StatusCodes.Status429TooManyRequests;
    options.AddPolicy("ip-limiter", httpContext =>
    {
        var ip = httpContext.Connection.RemoteIpAddress?.ToString();
        if (string.IsNullOrEmpty(ip))
        {
            ip = httpContext.Request.Headers["X-Forwarded-For"].FirstOrDefault()?.Split(',')[0].Trim();
        }
        ip ??= "unknown";

        return RateLimitPartition.GetSlidingWindowLimiter(ip, _ => new SlidingWindowRateLimiterOptions
        {
            PermitLimit = 10,
            Window = TimeSpan.FromHours(1),
            SegmentsPerWindow = 6,
            QueueLimit = 0
        });
    });
});

var reportsDirectory = builder.Configuration["REPORTS_DIRECTORY"]
    ?? Path.Combine(builder.Environment.ContentRootPath, "reports");

builder.Services.AddSingleton(sp =>
    new ReportStore(reportsDirectory, sp.GetRequiredService<ILogger<ReportStore>>()));

if (isLiveGitHub)
{
    builder.Services.AddSingleton<IGitHubClient>(new GitHubClient(new ProductHeaderValue("battler-bug-reporter"))
    {
        Credentials = new Credentials(githubToken)
    });
}

var localReportCounter = 0;

var app = builder.Build();

app.UseForwardedHeaders();
app.UseCors();
app.UseRateLimiter();

app.Logger.LogInformation(
    "Starting battler-bug-reporter in {Mode} mode (Target Repo: {Owner}/{Repo}).",
    isLiveGitHub ? "Live GitHub" : "Soft-Launch Local Preview",
    repoOwner,
    repoName
);

// Health Check
app.MapGet("/health", () => Results.Ok(new
{
    status = "healthy",
    service = "battler-bug-reporter",
    mode = isLiveGitHub ? "live-github" : "soft-launch-local",
    targetRepo = $"{repoOwner}/{repoName}",
    timestamp = DateTime.UtcNow
}));

// Soft-Launch Local Preview Endpoints
if (!isLiveGitHub)
{
    app.MapGet("/reports/{id:regex(^[a-fA-F0-9]{{8}}$)}", async (string id, ReportStore reportStore) =>
    {
        var report = await reportStore.GetReportAsync(id, $"{repoOwner}/{repoName}");
        if (report == null)
        {
            return Results.NotFound(new { error = $"Report '{id}' not found." });
        }

        var html = ReportPreviewRenderer.Render(report);
        return Results.Content(html, "text/html; charset=utf-8");
    }).RequireRateLimiting("ip-limiter");

    app.MapGet("/reports/{id:regex(^[a-fA-F0-9]{{8}}$)}/json", async (string id, ReportStore reportStore) =>
    {
        var rawJson = await reportStore.GetRawJsonAsync(id);
        if (rawJson == null)
        {
            return Results.NotFound(new { error = $"JSON for report '{id}' not found." });
        }

        return Results.Content(rawJson, "application/json; charset=utf-8");
    }).RequireRateLimiting("ip-limiter");
}

// Bug Report Submission Endpoint
app.MapPost("/api/report-bug", async (
    HttpContext context,
    BugReportRequest request,
    ReportStore reportStore,
    IServiceProvider serviceProvider) =>
{
    if (string.IsNullOrWhiteSpace(request.Title) || string.IsNullOrWhiteSpace(request.Description))
    {
        return Results.BadRequest(new { error = "Title and description are required." });
    }

    if (request.Title.Length > 250)
    {
        return Results.BadRequest(new { error = "Title cannot exceed 250 characters." });
    }

    if (request.Description.Length > 65536)
    {
        return Results.BadRequest(new { error = "Description cannot exceed 65,536 characters." });
    }

    var reportId = Guid.NewGuid().ToString("N")[..8];
    var serverTimestamp = DateTime.UtcNow.ToString("o");
    var attachmentFileName = $"battler-debug-{reportId}.json";
    var rawJson = JsonSerializer.Serialize(request, JsonDefaults.Indented);

    var issueTitle = IssueBuilder.FormatIssueTitle(request.Title);
    var labels = IssueBuilder.DetermineLabels(request);

    // Soft-Launch Mode: Persist locally
    if (!isLiveGitHub)
    {
        var scheme = context.Request.Scheme;
        var host = context.Request.Host.Value;
        var localPreviewUrl = $"{scheme}://{host}/reports/{reportId}";
        var localJsonUrl = $"{scheme}://{host}/reports/{reportId}/json";

        var issueBody = IssueBuilder.BuildMarkdownBody(
            request, reportId, serverTimestamp, attachmentFileName, localJsonUrl);

        var reportNum = Interlocked.Increment(ref localReportCounter);

        var storedReport = new StoredBugReport(
            reportId,
            reportNum,
            issueTitle,
            issueBody,
            labels,
            attachmentFileName,
            localJsonUrl,
            $"{repoOwner}/{repoName}",
            serverTimestamp,
            request
        );

        await reportStore.SaveReportAsync(storedReport, rawJson);

        app.Logger.LogInformation("[SOFT LAUNCH] Bug report #{Num} saved: {Title} ({Url})",
            reportNum, issueTitle, localPreviewUrl);

        return Results.Ok(new
        {
            success = true,
            issueUrl = localPreviewUrl,
            issueNumber = reportNum,
            reportId,
            labels,
            softLaunch = true,
            message = $"[Soft Launch] Saved locally to reports/bug-{reportId}.md"
        });
    }

    // Live GitHub Mode: File via Octokit
    var githubClient = serviceProvider.GetRequiredService<IGitHubClient>();

    try
    {
        string attachmentUrl;
        string? fallbackEmbeddedJson = null;
        try
        {
            var newGist = new NewGist
            {
                Description = $"Battler Diagnostic Dump [{reportId}]",
                Public = false
            };
            newGist.Files.Add(attachmentFileName, rawJson);
            var createdGist = await githubClient.Gist.Create(newGist);
            attachmentUrl = createdGist.HtmlUrl;
        }
        catch (Exception gistEx)
        {
            app.Logger.LogWarning(gistEx, "Could not create Gist attachment for report {ReportId}. Using inline fallback.", reportId);
            attachmentFileName = "Diagnostics Attachment Unavailable (Gist Failed)";
            attachmentUrl = "#";
            if (rawJson.Length <= 40000)
            {
                fallbackEmbeddedJson = rawJson;
            }
        }

        var liveBody = IssueBuilder.BuildMarkdownBody(
            request, reportId, serverTimestamp, attachmentFileName, attachmentUrl, fallbackEmbeddedJson);

        var newIssue = new NewIssue(issueTitle) { Body = liveBody };
        foreach (var label in labels)
        {
            newIssue.Labels.Add(label);
        }

        var issue = await githubClient.Issue.Create(repoOwner, repoName, newIssue);

        app.Logger.LogInformation("[GITHUB] Bug report #{Number} created: {Title} ({Url})",
            issue.Number, issueTitle, issue.HtmlUrl);

        return Results.Ok(new
        {
            success = true,
            issueUrl = issue.HtmlUrl,
            issueNumber = issue.Number,
            reportId,
            labels,
            softLaunch = false
        });
    }
    catch (Exception ex)
    {
        app.Logger.LogError(ex, "Failed to submit GitHub bug report.");
        return Results.Problem(
            detail: ex.Message,
            statusCode: StatusCodes.Status500InternalServerError,
            title: "Failed to submit bug report to GitHub."
        );
    }
}).RequireRateLimiting("ip-limiter");

app.Run();
