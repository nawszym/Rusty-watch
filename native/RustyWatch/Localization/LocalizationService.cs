using System.ComponentModel;
using System.Globalization;
using System.Text.Json;

namespace RustyWatch.Localization;

/// <summary>
/// Most wiązaniowy — XAML wiąże się ze <c>Strings[...]</c>, a zmiana języka
/// powiadamia o zmianie całej właściwości <see cref="Strings"/>.
/// </summary>
public sealed class LocalizationBridge : INotifyPropertyChanged
{
    public bool IsSaveAs { get; private set; }

    public StringCatalog Strings => LocalizationService.Current;

    /// <summary>Tytuł i przycisk okna nowego projektu / „Save as..." zależą od trybu.</summary>
    public string DialogTitle => LocalizationService.Get(IsSaveAs ? "SaveAs" : "NewWatchface");

    public string DialogPrimary => LocalizationService.Get(IsSaveAs ? "Save" : "Create");

    public event PropertyChangedEventHandler? PropertyChanged;

    public void SetSaveAs(bool value)
    {
        IsSaveAs = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(DialogTitle)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(DialogPrimary)));
    }

    public void Refresh()
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Strings)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(DialogTitle)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(DialogPrimary)));
    }
}

/// <summary>
/// Rejestr języków z plików <c>%APPDATA%\RustyWatch\Localization\*.json</c>.
/// Nowy język wymaga tylko dodania pliku do tego katalogu — nie trzeba zmieniać kodu
/// ani przebudowywać aplikacji (wystarczy Odśwież w Ustawieniach).
/// </summary>
public static class LocalizationService
{
    private const string DefaultLanguage = "en";

    private static readonly List<StringCatalog> _languages = new();
    private static StringCatalog _current = new(DefaultLanguage, "English", new Dictionary<string, string>());

    public static IReadOnlyList<StringCatalog> Languages => _languages;
    public static StringCatalog Current => _current;

    /// <summary>Katalog, z którego wczytywane są katalogi (dane użytkownika).</summary>
    public static string CatalogsDirectory => AppPaths.LocalizationDirectory;

    public static void Initialize() => LoadCatalogs(preserveCurrent: false);

    /// <summary>
    /// Ponownie skanuje katalog języków po dodaniu/zmianie pliku przez użytkownika.
    /// Wybrany wcześniej język zostaje zachowany, o ile jego plik nadal istnieje.
    /// </summary>
    public static void Reload() => LoadCatalogs(preserveCurrent: true);

    private static void LoadCatalogs(bool preserveCurrent)
    {
        var previousCode = preserveCurrent ? _current.Code : null;
        var catalogs = LoadFrom(AppPaths.LocalizationDirectory);
        // Gdy użytkownik usunie wszystkie pliki, aplikacja musi i tak mieć interfejs.
        if (catalogs.Count == 0) catalogs = LoadFrom(AppPaths.BundledLocalizationDirectory);

        _languages.Clear();
        if (catalogs.Count == 0)
        {
            _languages.Add(_current);
            return;
        }

        // Angielski jest zapasowy dla niepełnych, użytkowniczych tłumaczeń.
        var fallback = catalogs.FirstOrDefault(l => l.Code == DefaultLanguage) ?? catalogs[0];
        foreach (var catalog in catalogs)
            if (!ReferenceEquals(catalog, fallback)) catalog.Fallback = fallback;

        catalogs.Sort((a, b) => string.CompareOrdinal(a.Code, b.Code));
        _languages.AddRange(catalogs);

        var saved = AppSettingsStore.Language ?? CultureInfo.CurrentUICulture.TwoLetterISOLanguageName;
        _current = (previousCode is null
                        ? null
                        : _languages.FirstOrDefault(l => l.Code.Equals(previousCode, StringComparison.OrdinalIgnoreCase)))
                   ?? _languages.FirstOrDefault(l => l.Code.Equals(saved, StringComparison.OrdinalIgnoreCase))
                   ?? _languages.FirstOrDefault(l => l.Code == DefaultLanguage)
                   ?? _languages[0];
    }

    private static List<StringCatalog> LoadFrom(string directory)
    {
        var result = new List<StringCatalog>();
        if (!Directory.Exists(directory)) return result;

        foreach (var file in Directory.EnumerateFiles(directory, "*.json"))
        {
            var code = Path.GetFileNameWithoutExtension(file);
            try
            {
                using var document = JsonDocument.Parse(File.ReadAllText(file));
                var root = document.RootElement;
                var displayName = root.TryGetProperty("languageName", out var name)
                    ? name.GetString() ?? code
                    : code;
                var strings = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                if (root.TryGetProperty("strings", out var values))
                {
                    foreach (var property in values.EnumerateObject())
                        strings[property.Name] = property.Value.GetString() ?? property.Name;
                }
                result.Add(new StringCatalog(code, displayName, strings));
            }
            catch (JsonException)
            {
                // Uszkodzony plik nie blokuje uruchomienia aplikacji.
            }
            catch (IOException)
            {
                // Plik zablokowany przez edytor — pomijamy w tej sesji.
            }
            catch (UnauthorizedAccessException)
            {
                // Brak uprawnień do odczytu — pomijamy w tej sesji.
            }
        }
        return result;
    }

    public static void SetLanguage(string code)
    {
        var next = _languages.FirstOrDefault(l => l.Code == code);
        if (next is null || ReferenceEquals(next, _current)) return;
        _current = next;
        AppSettingsStore.SetLanguage(code);
    }

    /// <summary>
    /// Komunikaty z rusty-core są kanonicznie po polsku; tłumaczymy je na język UI.
    /// Nieznane komunikaty przechodzą bez zmian (np. ścieżki plików).
    /// </summary>
    private static readonly (string Source, string Key)[] ErrorPatterns =
    {
        ("Nazwa tarczy nie może być pusta", "ErrFaceName"),
        ("Nazwa projektu nie może być pusta", "ErrProjectName"),
        ("Autor tarczy nie może być pusty", "ErrAuthor"),
        ("App ID musi być większe od 0", "ErrAppId"),
        ("Wersja tarczy nie może być pusta", "ErrVersionEmpty"),
        ("Wersja tarczy może mieć najwyżej", "ErrVersionLength"),
        ("Wersja tarczy nie może zawierać białych znaków", "ErrVersionChars"),
        ("warstwa nie istnieje: ", "ErrNoLayer"),
        ("Nie można przesunąć warstwy dalej", "ErrLayerBounds"),
        ("Tło jest zablokowane na najniższej warstwie", "ErrBackgroundLocked"),
        ("Projekt musi zawierać przynajmniej jeden widget", "ErrLastWidget"),
        ("Nieobsługiwany typ widgetu: ", "ErrUnsupportedWidget"),
        ("Format czasu nie może być pusty", "ErrTimeEmpty"),
        ("Format czasu może mieć najwyżej", "ErrTimeLength"),
        ("Format czasu nie może zawierać znaków kontrolnych", "ErrTimeControl"),
        ("Format czasu kończy się niedokończonym znakiem ucieczki", "ErrTimeEscape"),
        ("Format czasu musi zawierać przynajmniej jeden token", "ErrTimeToken"),
        ("Format daty nie może być pusty", "ErrDateEmpty"),
        ("Format daty może mieć najwyżej", "ErrDateLength"),
        ("Format daty nie może zawierać znaków kontrolnych", "ErrDateControl"),
        ("Format daty kończy się niedokończonym znakiem ucieczki", "ErrDateEscape"),
        ("Format daty musi zawierać przynajmniej jeden token", "ErrDateToken"),
        ("Rozmiar czcionki musi być w zakresie", "ErrFontSize"),
        ("Kolor musi mieć postać 0xAARRGGBB", "ErrColor"),
        ("Nieprawidłowy sposób dopasowania tekstu: ", "ErrTextStyle"),
        ("Widget czasu musi mieścić się w ekranie", "ErrBounds"),
        ("projekt nie zawiera widgetu czasu", "ErrNoTimeWidget"),
        ("projekt jest nieprawidłowy:", "ErrInvalidProject"),
        ("Folder already exists: ", "ErrFolderExists"),
        ("Czcionka musi mieć rozszerzenie .ttf", "ErrFontExt"),
        ("To nie jest prawidłowy plik TTF: ", "ErrFontInvalid"),
        ("Nie można zastąpić domyślnej czcionki", "ErrFontDefaultReplace"),
        ("Czcionka nie jest zainstalowana: ", "ErrFontNotInstalled"),
        ("Domyślnej czcionki nie można usunąć", "ErrFontDefaultRemove"),
        ("Nieprawidłowa nazwa pliku czcionki", "ErrFontBadName"),
        ("Obrazek musi mieć rozszerzenie", "ErrImageExt"),
        ("To nie jest prawidłowy obrazek: ", "ErrImageInvalid"),
        ("jest już w magazynie", "ErrImageExists"),
        ("Obrazek nie jest zainstalowany: ", "ErrImageNotInstalled"),
        ("Nieprawidłowa nazwa pliku obrazka", "ErrImageBadName"),
        ("Nie można odczytać katalogu obrazków: ", "ErrImageDir"),
        ("W folderze brakuje pliku ", "ErrHandsMissing"),
        ("Wskazówki godzin i minut muszą mieć ten sam rozmiar", "ErrHandsSize"),
        ("Wskazówki muszą być kwadratowymi plikami PNG", "ErrHandsSquare"),
        ("jest już zainstalowany", "ErrHandsExists"),
        ("Zestaw wskazówek nie jest zainstalowany: ", "ErrHandsNotInstalled"),
        ("Nieprawidłowa nazwa zestawu wskazówek", "ErrHandsBadName"),
        ("Wbudowanego zestawu wskazówek nie można usunąć", "ErrHandsDefaultRemove"),
        ("Nie znaleziono wbudowanego zestawu wskazówek: ", "ErrHandsBundledMissing"),
        ("Nie można odczytać katalogu wskazówek: ", "ErrHandsDir"),
    };

    public static string Error(string raw)
    {
        if (string.IsNullOrWhiteSpace(raw)) return raw;
        if (_current.Code == "pl") return raw;

        foreach (var (source, key) in ErrorPatterns)
        {
            if (raw.StartsWith(source, StringComparison.Ordinal))
                return Get(key) + raw[source.Length..];
        }
        return raw;
    }

    public static string Get(string key) => _current[key];

    public static string Format(string key, params object[] args)
    {
        var pattern = _current[key];
        try
        {
            return string.Format(CultureInfo.CurrentCulture, pattern, args);
        }
        catch (FormatException)
        {
            return pattern;
        }
    }
}
