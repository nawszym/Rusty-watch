using System.Text.Json;

namespace RustyWatch;

/// <summary>
/// Ustawienia aplikacji współdzielone przez strony (plik settings.json w %APPDATA%\RustyWatch).
/// Jeden magazyn zapobiega nadpisywaniu języka przez autora, autora przez folder itd.
/// </summary>
internal static class AppSettingsStore
{
    private static string? _language;
    private static string? _defaultAuthor;
    private static string? _projectsDirectory;
    private static string? _watchfacePreviewLanguage;
    private static string? _measurementSystem;
    private static string? _firstDayOfWeek;
    private static bool _snapToGrid;
    // Domyślnie włączone — tak samo jak przed wprowadzeniem opcji w flyoucie.
    private static bool _showUnselectedOutline = true;
    // Niestandardowe dane podglądu to ustawienie EDYTORA — nie trafia do project.json
    // ani do eksportowanego ZIP, więc nie może zmienić zachowania tarczy na zegarku.
    private static bool _customPreviewDataEnabled;
    private static readonly Dictionary<string, string> _customPreviewData = new();
    private static bool _loaded;

    public static string? Language
    {
        get { Load(); return _language; }
    }

    /// <summary>Pusty oznacza „użyj nazwy użytkownika Windows".</summary>
    public static string DefaultAuthor
    {
        get { Load(); return _defaultAuthor ?? ""; }
    }

    /// <summary>Katalog główny nowych projektów. Pusty = domyślny folder w %APPDATA%.</summary>
    public static string ProjectsDirectory
    {
        get
        {
            Load();
            return string.IsNullOrWhiteSpace(_projectsDirectory)
                ? AppPaths.DefaultProjectsDirectory
                : _projectsDirectory!;
        }
    }

    /// <summary>
    /// Język używany do podglądu tekstów tarczy: `auto` (język Windows), `pl` albo `en`.
    /// Sama tarcza zawsze wykrywa język zegarka w runtime — to ustawienie dotyczy tylko podglądu.
    /// </summary>
    public static string WatchfacePreviewLanguage
    {
        get
        {
            Load();
            return string.IsNullOrWhiteSpace(_watchfacePreviewLanguage)
                ? "auto"
                : _watchfacePreviewLanguage!;
        }
    }

    /// <summary>
    /// Domyślny system jednostek dla widgetów dystansu: `watch` (jednostka z zegarka,
    /// domyślnie), `metric` albo `imperial`.
    /// </summary>
    public static string MeasurementSystem
    {
        get
        {
            Load();
            return string.IsNullOrWhiteSpace(_measurementSystem)
                ? "watch"
                : _measurementSystem!;
        }
    }

    public static void SetMeasurementSystem(string value)
    {
        Load();
        var normalized = NormalizeMeasurementSystem(value);
        if (string.Equals(_measurementSystem, normalized, StringComparison.Ordinal)) return;
        _measurementSystem = normalized;
        Save();
    }

    private static string NormalizeMeasurementSystem(string? value)
    {
        if (string.Equals(value, "imperial", StringComparison.OrdinalIgnoreCase)) return "imperial";
        if (string.Equals(value, "metric", StringComparison.OrdinalIgnoreCase)) return "metric";
        return "watch";
    }

    /// <summary>
    /// Pierwszy dzień tygodnia dla pola „Dzień tygodnia (liczba)": `monday` (domyślnie)
    /// albo `sunday`. Wartość jest zapisywana w projekcie, więc dotyczy też eksportu.
    /// </summary>
    public static string FirstDayOfWeek
    {
        get
        {
            Load();
            return string.Equals(_firstDayOfWeek, "sunday", StringComparison.OrdinalIgnoreCase)
                ? "sunday"
                : "monday";
        }
    }

    public static void SetFirstDayOfWeek(string value)
    {
        Load();
        var normalized = string.Equals(value, "sunday", StringComparison.OrdinalIgnoreCase)
            ? "sunday"
            : "monday";
        if (string.Equals(_firstDayOfWeek, normalized, StringComparison.Ordinal)) return;
        _firstDayOfWeek = normalized;
        Save();
    }

    public static void SetWatchfacePreviewLanguage(string value)
    {
        Load();
        var normalized = string.IsNullOrWhiteSpace(value) ? "auto" : value.Trim().ToLowerInvariant();
        if (string.Equals(_watchfacePreviewLanguage, normalized, StringComparison.Ordinal)) return;
        _watchfacePreviewLanguage = normalized;
        Save();
    }

    /// <summary>Przyciąganie ramki do siatki w edytorze (domyślnie wyłączone).</summary>
    public static bool SnapToGrid
    {
        get { Load(); return _snapToGrid; }
    }

    public static void SetSnapToGrid(bool value)
    {
        Load();
        if (_snapToGrid == value) return;
        _snapToGrid = value;
        Save();
    }

    /// <summary>Obrys ramek niezaznaczonych widgetów w podglądzie (domyślnie włączony).</summary>
    public static bool ShowUnselectedOutline
    {
        get { Load(); return _showUnselectedOutline; }
    }

    public static void SetShowUnselectedOutline(bool value)
    {
        Load();
        if (_showUnselectedOutline == value) return;
        _showUnselectedOutline = value;
        Save();
    }

    /// <summary>Czy podgląd używa ręcznie wprowadzonych wartości zamiast danych na żywo.</summary>
    public static bool CustomPreviewDataEnabled
    {
        get { Load(); return _customPreviewDataEnabled; }
    }

    public static void SetCustomPreviewDataEnabled(bool value)
    {
        Load();
        if (_customPreviewDataEnabled == value) return;
        _customPreviewDataEnabled = value;
        Save();
    }

    /// <summary>Pusty brak wartości oznacza „użyj domyślnej wartości na żywo".</summary>
    public static string? GetCustomPreviewValue(string key)
    {
        Load();
        return _customPreviewData.TryGetValue(key, out var value) && !string.IsNullOrWhiteSpace(value)
            ? value
            : null;
    }

    public static void SetCustomPreviewValue(string key, string? value)
    {
        Load();
        if (string.IsNullOrWhiteSpace(key)) return;
        if (string.IsNullOrWhiteSpace(value)) _customPreviewData.Remove(key);
        else _customPreviewData[key] = value.Trim();
        Save();
    }

    public static IReadOnlyDictionary<string, string> CustomPreviewValues
    {
        get { Load(); return _customPreviewData; }
    }

    public static void ClearCustomPreviewValues()
    {
        Load();
        if (_customPreviewData.Count == 0) return;
        _customPreviewData.Clear();
        Save();
    }

    public static void SetLanguage(string code)
    {
        Load();
        if (string.Equals(_language, code, StringComparison.Ordinal)) return;
        _language = code;
        Save();
    }

    public static void SetDefaultAuthor(string value)
    {
        Load();
        value ??= string.Empty;
        if (string.Equals(_defaultAuthor, value, StringComparison.Ordinal)) return;
        _defaultAuthor = value;
        Save();
    }

    public static void SetProjectsDirectory(string value)
    {
        Load();
        var normalized = string.IsNullOrWhiteSpace(value)
            ? null
            : Path.GetFullPath(value.Trim().TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        if (string.Equals(_projectsDirectory, normalized, StringComparison.OrdinalIgnoreCase)) return;
        _projectsDirectory = normalized;
        Save();
    }

    private static void Load()
    {
        AppPaths.Initialize();
        if (_loaded) return;
        _loaded = true;
        try
        {
            var path = AppPaths.SettingsPath;
            if (!File.Exists(path)) return;
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            var root = document.RootElement;
            if (root.TryGetProperty("language", out var language))
                _language = language.GetString();
            if (root.TryGetProperty("defaultAuthor", out var author))
                _defaultAuthor = author.GetString();
            if (root.TryGetProperty("projectsDirectory", out var directory))
                _projectsDirectory = directory.GetString();
            if (root.TryGetProperty("watchfacePreviewLanguage", out var previewLanguage))
                _watchfacePreviewLanguage = previewLanguage.GetString();
            if (root.TryGetProperty("measurementSystem", out var measurementSystem))
                _measurementSystem = measurementSystem.GetString();
            if (root.TryGetProperty("firstDayOfWeek", out var firstDay))
                _firstDayOfWeek = firstDay.GetString();
            if (root.TryGetProperty("snapToGrid", out var snap))
                _snapToGrid = snap.ValueKind == JsonValueKind.True;
            if (root.TryGetProperty("showUnselectedOutline", out var outline))
                _showUnselectedOutline = outline.ValueKind == JsonValueKind.True;
            if (root.TryGetProperty("customPreviewDataEnabled", out var customEnabled))
                _customPreviewDataEnabled = customEnabled.ValueKind == JsonValueKind.True;
            if (root.TryGetProperty("customPreviewData", out var customValues)
                && customValues.ValueKind == JsonValueKind.Object)
            {
                _customPreviewData.Clear();
                foreach (var property in customValues.EnumerateObject())
                {
                    if (property.Value.ValueKind == JsonValueKind.String)
                        _customPreviewData[property.Name] = property.Value.GetString() ?? "";
                }
            }
        }
        catch
        {
            // Uszkodzony plik nie blokuje uruchomienia aplikacji.
        }
    }

    private static void Save()
    {
        try
        {
            var path = AppPaths.SettingsPath;
            var directory = Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
            File.WriteAllText(path, JsonSerializer.Serialize(new
            {
                language = _language,
                defaultAuthor = _defaultAuthor,
                projectsDirectory = _projectsDirectory,
                watchfacePreviewLanguage = _watchfacePreviewLanguage,
                measurementSystem = _measurementSystem,
                firstDayOfWeek = _firstDayOfWeek,
                snapToGrid = _snapToGrid,
                showUnselectedOutline = _showUnselectedOutline,
                customPreviewDataEnabled = _customPreviewDataEnabled,
                customPreviewData = _customPreviewData,
            }));
        }
        catch
        {
            // Ustawienia działają w bieżącej sesji nawet gdy zapis się nie udał.
        }
    }
}
