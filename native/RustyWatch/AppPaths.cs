using System.Text.Json;
using System.Text.Json.Nodes;

namespace RustyWatch;

/// <summary>
/// Katalog danych aplikacji w %APPDATA% (Roaming). Dzięki temu ustawienia, lista
/// ostatnich projektów i domyślny folder projektów przetrwają aktualizację aplikacji
/// (w tym podmianę katalogu z plikami wykonywalnymi).
/// </summary>
internal static class AppPaths
{
    public static string DataDirectory { get; private set; } = "";

    public static string SettingsPath => Path.Combine(DataDirectory, "settings.json");

    public static string RecentStorePath => Path.Combine(DataDirectory, "recent.json");

    public static string DefaultProjectsDirectory => Path.Combine(DataDirectory, "Projects");

    /// <summary>Katalog roboczy testów w symulatorze ZeppOS (ZIP + rozpakowany projekt + log).</summary>
    public static string EmulatorDirectory => Path.Combine(DataDirectory, "Emulator");

    /// <summary>Wspólny magazyn czcionek — jedno miejsce dla wszystkich projektów.</summary>
    public static string FontsDirectory => Path.Combine(DataDirectory, "Fonts");

    /// <summary>Wspólny magazyn obrazków dla widgetów IMAGES.</summary>
    public static string ImagesDirectory => Path.Combine(DataDirectory, "Images");

    /// <summary>Osobny magazyn teł — nie dzieli się z obrazkami widgetów IMAGES.</summary>
    public static string BackgroundsDirectory => Path.Combine(DataDirectory, "Backgrounds");

    /// <summary>Wspólny magazyn zestawów wskazówek zegara analogowego.</summary>
    public static string HandsDirectory => Path.Combine(DataDirectory, "Hands");

    /// <summary>
    /// Katalogi języków czytane w runtime. Użytkownik może tu dorzucić własny
    /// plik <c>&lt;kod&gt;.json</c> — nie wymaga to przebudowy aplikacji.
    /// </summary>
    public static string LocalizationDirectory => Path.Combine(DataDirectory, "Localization");

    /// <summary>Wbudowane katalogi języków — źródło kopiowane przy pierwszym uruchomieniu.</summary>
    public static string BundledLocalizationDirectory => Path.Combine(AppContext.BaseDirectory, "Localization");

    /// <summary>Czcionki dołączone do aplikacji (domyślna, licencja OFL).</summary>
    public static string BundledFontsDirectory => Path.Combine(AppContext.BaseDirectory, "Assets", "Fonts");

    /// <summary>Wbudowany zestaw wskazówek analogowych (z lekkim cieniem).</summary>
    public static string BundledHandsDirectory => Path.Combine(AppContext.BaseDirectory, "Assets", "Hands");

    /// <summary>
    /// WinUI 3 ładuje czcionki tylko z `ms-appx://` (katalog .exe), nie z `%APPDATA%`.
    /// Kopiujemy tam pliki z projektu wyłącznie na potrzeby podglądu 1:1.
    /// </summary>
    public static string PreviewFontCacheDirectory => Path.Combine(AppContext.BaseDirectory, "FontCache");

    /// <summary>Tworzy strukturę katalogów i przenosi dane ze starej lokalizacji (%LOCALAPPDATA%).</summary>
    public static void Initialize()
    {
        if (DataDirectory.Length > 0) return;

        DataDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "RustyWatch");
        Directory.CreateDirectory(DataDirectory);
        Directory.CreateDirectory(DefaultProjectsDirectory);
        Directory.CreateDirectory(EmulatorDirectory);
        Directory.CreateDirectory(FontsDirectory);
        Directory.CreateDirectory(ImagesDirectory);
        Directory.CreateDirectory(BackgroundsDirectory);
        Directory.CreateDirectory(HandsDirectory);
        Directory.CreateDirectory(LocalizationDirectory);
        Directory.CreateDirectory(PreviewFontCacheDirectory);

        SeedLocalization();
        MigrateFromLocalAppData();
    }

    /// <summary>
    /// Kopiuje wbudowane pliki językowe do %APPDATA% tylko wtedy, gdy ich tam jeszcze nie ma.
    /// Istniejące pliki nie są nadpisywane — uzupełniamy w nich co najwyżej klucze dodane w
    /// nowszej wersji aplikacji. Dzięki temu tłumaczenia użytkownika przetrwają aktualizację,
    /// a usunięcie pliku przywraca jego wersję wbudowaną przy następnym starcie.
    /// </summary>
    private static void SeedLocalization()
    {
        try
        {
            if (!Directory.Exists(BundledLocalizationDirectory)) return;
            foreach (var source in Directory.EnumerateFiles(BundledLocalizationDirectory))
            {
                var destination = Path.Combine(LocalizationDirectory, Path.GetFileName(source));
                if (!File.Exists(destination))
                {
                    try
                    {
                        File.Copy(source, destination);
                    }
                    catch
                    {
                        // Zablokowany plik nie może zatrzymać startu aplikacji.
                    }
                }
                else if (source.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
                {
                    MergeMissingKeys(source, destination);
                }
            }
        }
        catch
        {
            // Brak dostępu do katalogu wyjściowego również nie blokuje startu.
        }
    }

    /// <summary>Dodaje do katalogu użytkownika klucze, których nie było w jego wersji pliku.</summary>
    private static void MergeMissingKeys(string bundledPath, string userPath)
    {
        try
        {
            if (JsonNode.Parse(File.ReadAllText(bundledPath)) is not JsonObject bundled
                || JsonNode.Parse(File.ReadAllText(userPath)) is not JsonObject user
                || bundled["strings"] is not JsonObject bundledStrings
                || user["strings"] is not JsonObject userStrings) return;

            var added = false;
            foreach (var pair in bundledStrings)
            {
                if (userStrings.ContainsKey(pair.Key)) continue;
                userStrings[pair.Key] = pair.Value?.DeepClone();
                added = true;
            }
            if (added)
                File.WriteAllText(userPath,
                    user.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        }
        catch (JsonException)
        {
            // Uszkodzony plik zostawiamy w spokoju — LocalizationService go pominie.
        }
        catch (IOException)
        {
            // Plik zablokowany przez edytor — spróbujemy przy następnym starcie.
        }
        catch (UnauthorizedAccessException)
        {
            // Brak uprawnień do zapisu — aplikacja działa z tym, co jest.
        }
    }

    private static void MigrateFromLocalAppData()
    {
        var oldDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "RustyWatch");
        if (!Directory.Exists(oldDirectory)) return;
        if (string.Equals(
                Path.GetFullPath(oldDirectory),
                Path.GetFullPath(DataDirectory),
                StringComparison.OrdinalIgnoreCase))
            return;

        // Kopiujemy tylko gdy w nowej lokalizacji jeszcze nie ma pliku — dane z %APPDATA% mają
        // pierwszeństwo, żeby aktualizacja nie nadpisała nowszych ustawień.
        foreach (var name in new[] { "settings.json", "recent.json" })
        {
            var source = Path.Combine(oldDirectory, name);
            var destination = Path.Combine(DataDirectory, name);
            if (!File.Exists(source) || File.Exists(destination)) continue;
            try
            {
                File.Copy(source, destination);
            }
            catch
            {
                // Uszkodzony lub zablokowany plik nie blokuje startu aplikacji.
            }
        }

        // Starsze wersje trzymały własne katalogi w %LOCALAPPDATA% — przenosimy je,
        // jeśli w %APPDATA% ich jeszcze nie ma (wbudowane pliki są już zaseedowane).
        var oldLocalization = Path.Combine(oldDirectory, "Localization");
        if (Directory.Exists(oldLocalization))
        {
            foreach (var source in Directory.EnumerateFiles(oldLocalization, "*.json"))
            {
                var destination = Path.Combine(LocalizationDirectory, Path.GetFileName(source));
                if (File.Exists(destination)) continue;
                try
                {
                    File.Copy(source, destination);
                }
                catch
                {
                    // Uszkodzony plik nie blokuje startu aplikacji.
                }
            }
        }
    }
}
