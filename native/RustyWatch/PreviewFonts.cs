using Microsoft.UI.Xaml.Media;

namespace RustyWatch;

/// <summary>
/// WinUI 3 (aplikacja niepakietowa) ładuje czcionki tylko spod `ms-appx://`, czyli
/// z katalogu pliku wykonywalnego — nie potrafi otworzyć `%APPDATA%` ani dowolnej
/// ścieżki. Kopiujemy więc pliki do `FontCache` przy .exe i tworzymy FontFamily
/// wskazujący na dokładnie ten sam TTF, który trafia do wygenerowanej tarczy.
/// Dzięki temu podgląd 1:1 używa tych samych glifów co ZeppOS.
/// </summary>
internal static class PreviewFonts
{
    private static readonly Dictionary<string, string> Families =
        new(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, FontFamily> Loaded =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Nazwy rodzin z katalogu czcionek (zwracane przez rusty-core).</summary>
    public static void RefreshCatalog(IEnumerable<FontAssetDto> fonts)
    {
        Families.Clear();
        Loaded.Clear();
        foreach (var font in fonts)
        {
            Families[font.File] = font.Family;
        }
    }

    public static string FamilyName(string file) =>
        Families.TryGetValue(file, out var family) ? family : Path.GetFileNameWithoutExtension(file);

    /// <summary>
    /// Zwraca FontFamily dla pliku z projektu. `sourceDirectory` to katalog z plikiem
    /// (`assets/fonts` w projekcie albo `%APPDATA%\RustyWatch\Fonts`).
    /// </summary>
    public static FontFamily Get(string? file, string? sourceDirectory = null)
    {
        if (string.IsNullOrWhiteSpace(file))
        {
            // Starsze projekty bez czcionki używają czcionki systemowej.
            return new FontFamily("Segoe UI");
        }

        var key = file.ToLowerInvariant();
        if (Loaded.TryGetValue(key, out var cached))
        {
            return cached;
        }

        var source = Path.Combine(
            sourceDirectory ?? Path.Combine(AppPaths.DataDirectory, "Fonts"),
            Path.GetFileName(file));
        var cache = Path.Combine(AppPaths.PreviewFontCacheDirectory, Path.GetFileName(file));
        try
        {
            if (File.Exists(source))
            {
                Directory.CreateDirectory(AppPaths.PreviewFontCacheDirectory);
                if (!File.Exists(cache)
                    || new FileInfo(source).Length != new FileInfo(cache).Length
                    || File.GetLastWriteTimeUtc(source) > File.GetLastWriteTimeUtc(cache))
                {
                    File.Copy(source, cache, overwrite: true);
                }
            }
        }
        catch
        {
            // Brak uprawnień do katalogu .exe nie może zablokować edycji — podgląd
            // pokaże wtedy czcionkę systemową zamiast własnej.
        }

        var family = FamilyName(file);
        var escaped = Uri.EscapeDataString(Path.GetFileName(file));
        var familyPart = string.IsNullOrWhiteSpace(family) ? "" : "#" + family;
        var result = new FontFamily($"ms-appx:///FontCache/{escaped}{familyPart}");
        Loaded[key] = result;
        return result;
    }

    /// <summary>Czcionka z magazynu aplikacji (używana jako „Aa" na stronie Assets).</summary>
    public static FontFamily FromStore(string file) =>
        Get(file, Path.Combine(AppPaths.DataDirectory, "Fonts"));
}
