using System.Collections.Generic;
using System.ComponentModel;

namespace RustyWatch.Localization;

/// <summary>
/// Teksty jednego języka. Wiązania XAML używają indeksera: <c>{Binding Strings[Key]}</c>.
/// </summary>
public sealed class StringCatalog : INotifyPropertyChanged
{
    private readonly Dictionary<string, string> _strings;

    public StringCatalog(string code, string displayName, Dictionary<string, string> strings)
    {
        Code = code;
        DisplayName = displayName;
        _strings = strings;
    }

    public string Code { get; }

    /// <summary>Nazwa języka wyświetlana w selektorze (zawsze w tym języku).</summary>
    public string DisplayName { get; }

    /// <summary>
    /// Katalog zapasowy dla niepełnych tłumaczeń tworzonych przez użytkownika.
    /// Wiązania XAML używają indeksera, więc zapas musi być częścią katalogu —
    /// inaczej brakujące klucze wyświetlałyby się jako nazwy kluczy.
    /// </summary>
    internal StringCatalog? Fallback { get; set; }

    public string this[string key]
    {
        get
        {
            if (_strings.TryGetValue(key, out var value)) return value;
            if (Fallback is not null && !ReferenceEquals(Fallback, this)) return Fallback[key];
            return key;
        }
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    /// <summary>Powiadamia wiązania o zmianie języka (nowy katalog). Nie mutujemy tekstów.</summary>
    public void Refresh() => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs("Item[]"));
}
