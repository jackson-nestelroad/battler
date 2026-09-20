using System.Net;
using System.Threading.RateLimiting;

using BugReporter;

using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.RateLimiting;
using Microsoft.Extensions.Configuration;

using Xunit;

namespace BugReporter.Tests;

public class RateLimiterConfigTests
{
    [Fact]
    public void GetClientIp_WhenRemoteIpExists_ReturnsRemoteIp()
    {
        var context = new DefaultHttpContext();
        context.Connection.RemoteIpAddress = IPAddress.Parse("192.168.1.100");

        var ip = RateLimiterConfig.GetClientIp(context);

        Assert.Equal("192.168.1.100", ip);
    }

    [Fact]
    public void GetClientIp_WhenRemoteIpNull_ExtractsFirstForwardedFor()
    {
        var context = new DefaultHttpContext();
        context.Connection.RemoteIpAddress = null;
        context.Request.Headers["X-Forwarded-For"] = "203.0.113.195, 70.41.3.18";

        var ip = RateLimiterConfig.GetClientIp(context);

        Assert.Equal("203.0.113.195", ip);
    }

    [Fact]
    public void GetClientIp_WhenNoIpAvailable_ReturnsUnknown()
    {
        var context = new DefaultHttpContext();
        context.Connection.RemoteIpAddress = null;

        var ip = RateLimiterConfig.GetClientIp(context);

        Assert.Equal("unknown", ip);
    }

    [Theory]
    [InlineData("/health")]
    [InlineData("/reports/b8094722")]
    [InlineData("/reports/b8094722/json")]
    [InlineData("/favicon.ico")]
    public async Task NonBugReportEndpoints_BypassGlobalLimiter(string path)
    {
        var limiter = RateLimiterConfig.CreateGlobalLimiter(concurrencyLimit: 1, permitLimit: 1);

        var context = new DefaultHttpContext();
        context.Request.Path = path;

        // Acquire multiple times simultaneously without consuming permits
        using var lease1 = await limiter.AcquireAsync(context, 1);
        using var lease2 = await limiter.AcquireAsync(context, 1);
        using var lease3 = await limiter.AcquireAsync(context, 1);

        Assert.True(lease1.IsAcquired);
        Assert.True(lease2.IsAcquired);
        Assert.True(lease3.IsAcquired);
    }

    [Fact]
    public async Task GlobalConcurrencyLimiter_EnforcesLimitAndReleasesSlot()
    {
        var limiter = RateLimiterConfig.CreateGlobalLimiter(concurrencyLimit: 2, permitLimit: 100);

        var apiContext = new DefaultHttpContext();
        apiContext.Request.Path = "/api/report-bug";

        var lease1 = await limiter.AcquireAsync(apiContext, 1);
        var lease2 = await limiter.AcquireAsync(apiContext, 1);

        Assert.True(lease1.IsAcquired);
        Assert.True(lease2.IsAcquired);

        // 3rd concurrent attempt exceeds concurrency limit of 2
        var lease3 = await limiter.AcquireAsync(apiContext, 1);
        Assert.False(lease3.IsAcquired);

        // Disposing one in-flight lease frees up a slot
        lease1.Dispose();

        var lease4 = await limiter.AcquireAsync(apiContext, 1);
        Assert.True(lease4.IsAcquired);

        lease2.Dispose();
        lease4.Dispose();
    }

    [Fact]
    public async Task GlobalSlidingWindow_EnforcesTotalPermitLimit()
    {
        var limiter = RateLimiterConfig.CreateGlobalLimiter(concurrencyLimit: 10, permitLimit: 3);

        var apiContext = new DefaultHttpContext();
        apiContext.Request.Path = "/api/report-bug";

        // 3 sequential requests acquire and release concurrency
        for (var i = 0; i < 3; i++)
        {
            using var lease = await limiter.AcquireAsync(apiContext, 1);
            Assert.True(lease.IsAcquired);
        }

        // 4th request exceeds permit limit of 3
        using var rejectedLease = await limiter.AcquireAsync(apiContext, 1);
        Assert.False(rejectedLease.IsAcquired);
    }

    [Fact]
    public void Configure_RegistersGlobalLimiterAndIpPolicy()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["RateLimiting:GlobalPermitLimit"] = "50",
                ["RateLimiting:GlobalConcurrencyLimit"] = "3",
                ["RateLimiting:IpPermitLimit"] = "5"
            })
            .Build();

        var options = new RateLimiterOptions();
        RateLimiterConfig.Configure(options, configuration);

        Assert.Equal(StatusCodes.Status429TooManyRequests, options.RejectionStatusCode);
        Assert.NotNull(options.GlobalLimiter);
        Assert.NotNull(options.OnRejected);
    }
}
