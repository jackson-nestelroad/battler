using System.Text.Json;
using System.Text.Json.Serialization;

namespace BugReporter;

public static class JsonDefaults
{
    public static readonly JsonSerializerOptions Indented = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    public static readonly JsonSerializerOptions Web = new(JsonSerializerDefaults.Web);
}
