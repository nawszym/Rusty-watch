using System.Linq;
using System.Text.Json;
using Microsoft.Win32;

namespace RustyWatch;

/// <summary>
/// Wykrywa instalację oficjalnego Zepp OS Simulator i stan przygotowanego obrazu
/// urządzenia Active 2 (Round). Nie pobieramy ani nie dołączamy plików symulatora —
/// to oprogramowanie Zepp Health pobierane jest wyłącznie z oficjalnej strony.
/// </summary>
internal static class ZeppSimulator
{
    /// <summary>Oficjalna strona z instalatorami (użytkownik sam akceptuje licencję).</summary>
    public const string DownloadPage = "https://docs.zepp.com/docs/guides/tools/simulator/download/";

    private const int Active2DeviceSource = 8913153;

    /// <summary>Ścieżka do simulator.exe albo null, gdy symulator nie jest zainstalowany.</summary>
    public static string? FindExecutable()
    {
        foreach (var candidate in Candidates())
        {
            if (File.Exists(candidate)) return candidate;
        }
        return null;
    }

    /// <summary>Czy w symulatorze pobrano obraz Active 2 (Round) zawierający nasz deviceSource.</summary>
    public static bool IsDeviceImageReady()
    {
        var config = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "simulator", "config.json");
        if (!File.Exists(config)) return false;

        try
        {
            using var root = JsonDocument.Parse(File.ReadAllText(config));
            if (!root.RootElement.TryGetProperty("selectDeviceList", out var raw)
                || raw.ValueKind != JsonValueKind.String) return false;

            using var device = JsonDocument.Parse(raw.GetString()!);
            var selected = device.RootElement;
            // Zależnie od wersji symulatora pole to bywa obiektem albo tablicą obiektów.
            var entries = selected.ValueKind == JsonValueKind.Array
                ? selected.EnumerateArray().ToArray()
                : new[] { selected };

            foreach (var entry in entries)
            {
                if (entry.ValueKind != JsonValueKind.Object
                    || !entry.TryGetProperty("device", out var ids)
                    || ids.ValueKind != JsonValueKind.Array) continue;
                if (!ids.EnumerateArray().Any(id =>
                        id.ValueKind == JsonValueKind.Number && id.GetInt32() == Active2DeviceSource))
                    continue;

                // status 2 = obraz pobrany i wybrany jako aktywny model.
                if (entry.TryGetProperty("status", out var status)
                    && status.ValueKind == JsonValueKind.Number
                    && status.GetInt32() >= 2) return true;
            }
            return false;
        }
        catch (JsonException)
        {
            return false;
        }
        catch (IOException)
        {
            return false;
        }
    }

    /// <summary>Czy okno zegarka (device simulator) jest aktualnie otwarte.</summary>
    public static bool IsWatchWindowOpen()
    {
        var config = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "simulator", "config.json");
        if (!File.Exists(config)) return false;

        try
        {
            using var root = JsonDocument.Parse(File.ReadAllText(config));
            return (root.RootElement.TryGetProperty("simuOpen", out var open)
                    && open.ValueKind == JsonValueKind.Number
                    && open.GetInt32() == 1)
                   || (root.RootElement.TryGetProperty("firmwareSimulatorIsReady", out var ready)
                       && ready.ValueKind == JsonValueKind.True);
        }
        catch (JsonException)
        {
            return false;
        }
        catch (IOException)
        {
            return false;
        }
    }

    /// <summary>Uruchamia symulator, żeby użytkownik mógł otworzyć okno zegarka / pobrać model.</summary>
    public static void Launch(string executable)
    {
        if (System.Diagnostics.Process.GetProcessesByName("simulator").Length > 0) return;
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = executable,
            UseShellExecute = true,
        });
    }

    private static IEnumerable<string> Candidates()
    {
        yield return Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Programs", "simulator", "simulator.exe");
        yield return Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
            "simulator", "simulator.exe");
        if (Environment.Is64BitOperatingSystem)
        {
            yield return Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
                "simulator", "simulator.exe");
        }

        foreach (var uninstall in UninstallStrings())
        {
            var directory = Path.GetDirectoryName(uninstall);
            if (!string.IsNullOrWhiteSpace(directory))
                yield return Path.Combine(directory, "simulator.exe");
        }
    }

    private static IEnumerable<string> UninstallStrings()
    {
        var keys = new[]
        {
            @"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            @"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        };
        foreach (var hive in new[] { Registry.CurrentUser, Registry.LocalMachine })
        {
            foreach (var keyPath in keys)
            {
                using var key = hive.OpenSubKey(keyPath);
                if (key is null) continue;
                foreach (var name in key.GetSubKeyNames())
                {
                    using var sub = key.OpenSubKey(name);
                    var uninstall = sub?.GetValue("UninstallString") as string;
                    if (string.IsNullOrWhiteSpace(uninstall)) continue;
                    if (uninstall.StartsWith('"'))
                        uninstall = uninstall.Trim('"');
                    yield return uninstall;
                }
            }
        }
    }
}
