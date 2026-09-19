using System.Text.Json;
using System.Threading.RateLimiting;

using Microsoft.AspNetCore.RateLimiting;

namespace BugReporter;

/// <summary>
/// Configures layered rate limiting:
/// 1. Global concurrency limiter (prevents concurrent Octokit/GitHub API bursts).
/// 2. Global sliding window (prevents distributed IP / botnet abuse across the entire service).
/// 3. Per-IP sliding window (prevents individual client spam).
/// </summary>
public static class RateLimiterConfig
{
    public const string IpPolicyName = "ip-limiter";

    public static string GetClientIp(HttpContext httpContext)
    {
        var ip = httpContext.Connection.RemoteIpAddress?.ToString();
        if (string.IsNullOrEmpty(ip))
        {
            ip = httpContext.Request.Headers["X-Forwarded-For"].FirstOrDefault()?.Split(',')[0].Trim();
        }
        return string.IsNullOrEmpty(ip) ? "unknown" : ip;
    }

    public static PartitionedRateLimiter<HttpContext> CreateGlobalLimiter(
        int concurrencyLimit = 2,
        int permitLimit = 30,
        TimeSpan? window = null)
    {
        var timeWindow = window ?? TimeSpan.FromHours(1);

        return PartitionedRateLimiter.CreateChained(
            PartitionedRateLimiter.Create<HttpContext, string>(httpContext =>
            {
                // Only enforce global concurrency limit on bug submissions
                if (!httpContext.Request.Path.StartsWithSegments("/api/report-bug"))
                {
                    return RateLimitPartition.GetNoLimiter("unlimited");
                }

                return RateLimitPartition.GetConcurrencyLimiter("global-concurrency", _ => new ConcurrencyLimiterOptions
                {
                    PermitLimit = concurrencyLimit,
                    QueueLimit = 0
                });
            }),
            PartitionedRateLimiter.Create<HttpContext, string>(httpContext =>
            {
                // Only enforce global sliding window quota on bug submissions
                if (!httpContext.Request.Path.StartsWithSegments("/api/report-bug"))
                {
                    return RateLimitPartition.GetNoLimiter("unlimited");
                }

                return RateLimitPartition.GetSlidingWindowLimiter("global-sliding-window", _ => new SlidingWindowRateLimiterOptions
                {
                    PermitLimit = permitLimit,
                    Window = timeWindow,
                    SegmentsPerWindow = 6,
                    QueueLimit = 0
                });
            })
        );
    }

    public static void Configure(RateLimiterOptions options, IConfiguration configuration)
    {
        options.RejectionStatusCode = StatusCodes.Status429TooManyRequests;

        var globalPermitLimit = int.TryParse(
            configuration["RateLimiting:GlobalPermitLimit"] ?? configuration["RATE_LIMIT_GLOBAL_PERMIT_LIMIT"],
            out var gLimit) ? gLimit : 30;

        var globalWindowHours = double.TryParse(
            configuration["RateLimiting:GlobalWindowHours"] ?? configuration["RATE_LIMIT_GLOBAL_WINDOW_HOURS"],
            out var gHours) ? gHours : 1.0;

        var globalConcurrencyLimit = int.TryParse(
            configuration["RateLimiting:GlobalConcurrencyLimit"] ?? configuration["RATE_LIMIT_GLOBAL_CONCURRENCY_LIMIT"],
            out var gConc) ? gConc : 2;

        var ipPermitLimit = int.TryParse(
            configuration["RateLimiting:IpPermitLimit"] ?? configuration["RATE_LIMIT_IP_PERMIT_LIMIT"],
            out var ipLimit) ? ipLimit : 10;

        var ipWindowHours = double.TryParse(
            configuration["RateLimiting:IpWindowHours"] ?? configuration["RATE_LIMIT_IP_WINDOW_HOURS"],
            out var ipHours) ? ipHours : 1.0;

        // 1. Global Limiter: Chains Concurrency Limiting + Global Sliding Window across all IPs
        options.GlobalLimiter = CreateGlobalLimiter(
            globalConcurrencyLimit,
            globalPermitLimit,
            TimeSpan.FromHours(globalWindowHours)
        );

        // 2. Per-IP Limiter: Sliding Window per individual client IP
        options.AddPolicy(IpPolicyName, httpContext =>
        {
            var ip = GetClientIp(httpContext);

            return RateLimitPartition.GetSlidingWindowLimiter(ip, _ => new SlidingWindowRateLimiterOptions
            {
                PermitLimit = ipPermitLimit,
                Window = TimeSpan.FromHours(ipWindowHours),
                SegmentsPerWindow = 6,
                QueueLimit = 0
            });
        });

        // 3. Informative JSON rejection payload
        options.OnRejected = async (context, token) =>
        {
            context.HttpContext.Response.StatusCode = StatusCodes.Status429TooManyRequests;
            context.HttpContext.Response.ContentType = "application/json";
            await context.HttpContext.Response.WriteAsync(
                JsonSerializer.Serialize(new
                {
                    error = "Too Many Requests",
                    detail = "Bug reporting rate limit exceeded. Please slow down and try again later."
                }, JsonDefaults.Indented), token);
        };
    }
}
