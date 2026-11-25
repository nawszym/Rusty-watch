using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Text;
using Windows.ApplicationModel.DataTransfer;
using Windows.Foundation;
using Windows.Graphics.Imaging;
using Windows.Storage;
using Windows.Storage.Pickers;
using Windows.System;
using Windows.UI;
using WinRT.Interop;
using RustyWatch.Localization;

namespace RustyWatch;

/// <summary>Pojedynczy wpis na liście ostatnich projektów (strona HOME).</summary>
public sealed class RecentProjectView
{
    public string Name { get; set; } = "";
    public string Folder { get; set; } = "";
    public string File { get; set; } = "";
    public string OpenedText { get; set; } = "";
}

/// <summary>Warstwa edytora — dane z Rusta, gotowe do narysowania w podglądzie.</summary>
public sealed class LayerView
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("name")] public string Name { get; set; } = "";
    [JsonPropertyName("kind")] public string Kind { get; set; } = "";
    [JsonPropertyName("x")] public int X { get; set; }
    [JsonPropertyName("y")] public int Y { get; set; }
    [JsonPropertyName("w")] public int W { get; set; }
    [JsonPropertyName("h")] public int H { get; set; }
    [JsonPropertyName("text")] public string? Text { get; set; }
    [JsonPropertyName("timeFormat")] public string? TimeFormat { get; set; }
    [JsonPropertyName("textSize")] public uint? TextSize { get; set; }
    [JsonPropertyName("font")] public string? Font { get; set; }
    [JsonPropertyName("color")] public string? Color { get; set; }
    [JsonPropertyName("align")] public string? Align { get; set; }
    [JsonPropertyName("vAlign")] public string? VAlign { get; set; }
    /// <summary>Czy widget zdrowotny pokazuje jednostkę (bpm, °C, kcal, %).</summary>
    [JsonPropertyName("showUnit")] public bool ShowUnit { get; set; } = true;
    /// <summary>Krycie widgetu 0–255 (255 = pełna nieprzezroczystość).</summary>
    [JsonPropertyName("alpha")] public byte Alpha { get; set; } = 255;
    /// <summary>Wybrana paczka ikon pogody (tylko weather-icon).</summary>
    [JsonPropertyName("weatherIconSet")] public string? WeatherIconSet { get; set; }
    [JsonPropertyName("src")] public string? Src { get; set; }
    [JsonPropertyName("imageFit")] public string ImageFit { get; set; } = "contain";
    [JsonPropertyName("autoScale")] public bool AutoScale { get; set; }
    [JsonPropertyName("textStyle")] public string? TextStyle { get; set; }
    [JsonPropertyName("visible")] public bool Visible { get; set; } = true;
    [JsonPropertyName("handSet")] public string? HandSet { get; set; }
    [JsonPropertyName("hourSrc")] public string? HourSrc { get; set; }
    [JsonPropertyName("minuteSrc")] public string? MinuteSrc { get; set; }
    [JsonPropertyName("secondSrc")] public string? SecondSrc { get; set; }
    [JsonPropertyName("capSrc")] public string? CapSrc { get; set; }
    [JsonPropertyName("statusIconSet")] public string? StatusIconSet { get; set; }
    [JsonPropertyName("statusType")] public string? StatusType { get; set; }
    /// <summary>Własny system jednostek widgetu dystansu; null = domyślny z projektu.</summary>
    [JsonPropertyName("measurementSystem")] public string? MeasurementSystem { get; set; }
    /// <summary>Dzień prognozy pogody: 0 = dziś, 4 = +4 dni.</summary>
    [JsonPropertyName("weatherDayOffset")] public byte WeatherDayOffset { get; set; }
    /// <summary>Faza snu w minutach albo jako procent całego snu.</summary>
    [JsonPropertyName("sleepStageMode")] public string SleepStageMode { get; set; } = "duration";
    /// <summary>Ustawienia łuku/paska postępu (z Rusta, snake_case).</summary>
    [JsonPropertyName("progress")] public ProgressSettingsView? Progress { get; set; }
}

/// <summary>Lustro `ProgressSettings` z rusty-core — nazwy JSON są snake_case.</summary>
public sealed class ProgressSettingsView
{
    [JsonPropertyName("source")] public string Source { get; set; } = "battery";
    [JsonPropertyName("min")] public double Min { get; set; }
    [JsonPropertyName("max")] public double Max { get; set; } = 100;
    [JsonPropertyName("value")] public double Value { get; set; } = 50;
    [JsonPropertyName("color")] public string Color { get; set; } = "0xFF4FC3F7";
    [JsonPropertyName("track_color")] public string TrackColor { get; set; } = "0xFF3A3A42";
    [JsonPropertyName("thickness")] public uint Thickness { get; set; } = 16;
    [JsonPropertyName("start_angle")] public int StartAngle { get; set; } = -135;
    [JsonPropertyName("end_angle")] public int EndAngle { get; set; } = 135;
    [JsonPropertyName("corner")] public string Corner { get; set; } = "round";
    [JsonPropertyName("radius")] public uint Radius { get; set; } = 8;
    [JsonPropertyName("segments")] public uint Segments { get; set; } = 1;
    [JsonPropertyName("gap")] public uint Gap { get; set; } = 4;
    [JsonPropertyName("orientation")] public string Orientation { get; set; } = "horizontal";
    [JsonPropertyName("invert")] public bool Invert { get; set; }
    [JsonPropertyName("show_track")] public bool ShowTrack { get; set; } = true;
}

/// <summary>Wiersz listy warstw — panel pokazuje warstwę najwyższą na górze.</summary>
public sealed class LayerRow
{
    public LayerView Layer { get; set; } = new();
    public string Id => Layer.Id;
    public string Title => Layer.Name;
    public string Subtitle => Layer.Kind switch
    {
        "time" => L("LayerTime"),
        "date" => L("LayerDate"),
        "weekday-number" => L("LayerWeekdayNumber"),
        "week-number" => L("LayerWeekNumber"),
        "day-of-year" => L("LayerDayOfYear"),
        "lunar-date" => L("LayerLunarDate"),
        "festival" => L("LayerFestival"),
        "world-clock" => L("LayerWorldClock"),
        "steps" => L("LayerSteps"),
        "distance" => L("LayerDistance"),
        "heart" => L("LayerHeart"),
        "stress" => L("LayerStress"),
        "pai" => L("LayerPai"),
        "skin-temp" => L("LayerSkinTemp"),
        "sleep" => L("LayerSleep"),
        "calories" => L("LayerCalories"),
        "spo2" => L("LayerSpo2"),
        "sleep-target" => L("LayerSleepTarget"),
        "weather-condition" => L("LayerWeatherCondition"),
        "weather-city" => L("LayerWeatherCity"),
        "weather-high" => L("LayerWeatherHigh"),
        "weather-low" => L("LayerWeatherLow"),
        "sunrise" => L("LayerSunrise"),
        "sunset" => L("LayerSunset"),
        "weather-icon" => L("LayerWeatherIcon"),
        "battery" => L("LayerBattery"),
        "status-alarm" => L("LayerStatusAlarm"),
        "status-dnd" => L("LayerStatusDnd"),
        "status-bluetooth" => L("LayerStatusBluetooth"),
        "status-lock" => L("LayerStatusLock"),
        "arc-progress" => L("LayerArcProgress"),
        "bar-progress" => L("LayerBarProgress"),
        "analog-time" => L("LayerAnalog"),
        "image" => L("LayerImage"),
        "images" => L("LayerImages"),
        _ => L("LayerText"),
    };
    public string Glyph => Layer.Kind is "image" or "images" ? "\uE8B7" : "\uE8B2";
    public bool CanMoveUp { get; set; }
    public bool CanMoveDown { get; set; }
    public string MoveUpText => L("MoveUp");
    public string MoveDownText => L("MoveDown");
    public string RemoveText => L("RemoveWidget");

    /// <summary><c>E890</c> = View, <c>ED1A</c> = Hide (Segoe Fluent Icons na Windows 11).</summary>
    public string VisibilityGlyph => Layer.Visible ? "\uE890" : "\uED1A";
    public string VisibilityText => L(Layer.Visible ? "HideLayer" : "ShowLayer");
    /// <summary>Ukryta warstwa zostaje na liście, ale jest wyszarzona.</summary>
    public double ContentOpacity => Layer.Visible ? 1.0 : 0.45;

    private static string L(string key) => LocalizationService.Get(key);
}

/// <summary>Pozycja katalogu widgetów — zakładka Widgets i flyout „Dodaj widget".</summary>
public sealed class WidgetCatalogItem
{
    public string Kind { get; set; } = "";
    public string Category { get; set; } = "";
    /// <summary>Tylko time i date mają wzorzec — źródło: katalog z Rusta.</summary>
    public bool HasPattern { get; set; }
    public string Title { get; set; } = "";
    public string Description { get; set; } = "";
    public string Glyph { get; set; } = "\uE8B2";
}

/// <summary>
/// Wspólny stan kafelka Assets. ProgressRing w jego własnym miejscu nie blokuje
/// reszty widoku — zmieniamy tylko właściwości tego jednego wpisu.
/// </summary>
public abstract class AssetItemBase : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    private bool _isLoading;
    public bool IsLoading
    {
        get => _isLoading;
        set
        {
            if (_isLoading == value) return;
            _isLoading = value;
            OnChanged(nameof(IsLoading), nameof(LoadingVisibility), nameof(PreviewVisibility));
        }
    }

    public Visibility LoadingVisibility => IsLoading ? Visibility.Visible : Visibility.Collapsed;
    public Visibility PreviewVisibility => IsLoading ? Visibility.Collapsed : Visibility.Visible;

    protected void OnChanged(params string[] names)
    {
        foreach (var name in names)
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }
}

/// <summary>
/// Tani znacznik zmiany plików paczki: nazwa, rozmiar i czas modyfikacji.
/// Rust ma własny fingerprint; tutaj wykrywamy tylko „coś się zmieniło", żeby
/// od razu włączyć pierścień ładowania w danym kafelku.
/// </summary>
internal static class AssetStamp
{
    public static string Compute(string directory, params string[] files)
    {
        var parts = new List<string>();
        foreach (var file in files)
        {
            var path = Path.Combine(directory, file);
            if (!File.Exists(path)) continue;
            var info = new FileInfo(path);
            parts.Add($"{file}:{info.Length}:{info.LastWriteTimeUtc.Ticks}");
        }
        return string.Join("|", parts);
    }

    public static string Status(string id) => Compute(
        Path.Combine(AppPaths.StatusDirectory, id),
        "alarm.png", "dnd.png", "bluetooth.png", "lock.png");

    public static string Hands(string id) => Compute(
        Path.Combine(AppPaths.HandsDirectory, id),
        "hour.png", "minute.png", "second.png", "cap.png");

    public static string Image(string directory, string file) => Compute(directory, file);
}

/// <summary>Miniaturki współdzielone przez kafelki — bez ponownego dekodowania przy wejściu na Assets.</summary>
internal static class AssetPreview
{
    private static readonly Dictionary<string, BitmapImage> Cache = new(StringComparer.Ordinal);
    private static readonly HashSet<BitmapImage> Decoded = new();
    private static readonly object Gate = new();

    public static BitmapImage Load(string directory, string id, string fallbackFile, string? stamp = null)
    {
        var folder = Path.Combine(directory, id);
        var preview = Path.Combine(folder, "preview.png");
        var source = File.Exists(preview) ? preview : Path.Combine(folder, fallbackFile);
        return LoadFile(source, stamp ?? "", 128);
    }

    public static BitmapImage LoadFile(string path, string stamp, int decodeWidth)
    {
        var key = $"{path}|{stamp}|{decodeWidth}";
        lock (Gate)
        {
            if (Cache.TryGetValue(key, out var cached)) return cached;
        }

        var image = new BitmapImage { DecodePixelWidth = decodeWidth };
        void MarkDecoded(object? sender, object e) { lock (Gate) Decoded.Add(image); }
        void MarkFailed(object? sender, ExceptionRoutedEventArgs e) { lock (Gate) Decoded.Remove(image); }
        image.ImageOpened += MarkDecoded;
        image.ImageFailed += MarkFailed;
        if (File.Exists(path)) image.UriSource = new Uri(path);
        else lock (Gate) Decoded.Remove(image);
        lock (Gate) Cache[key] = image;
        return image;
    }

    /// <summary>Pierścień znika dopiero po faktycznym otwarciu obrazka (albo natychmiast dla cache).</summary>
    public static void Observe(BitmapImage image, Action onDone)
    {
        lock (Gate)
        {
            if (Decoded.Contains(image))
            {
                onDone();
                return;
            }
        }
        void Done(object? sender, object e)
        {
            image.ImageOpened -= Done;
            image.ImageFailed -= Failed;
            onDone();
        }
        void Failed(object? sender, ExceptionRoutedEventArgs e)
        {
            image.ImageOpened -= Done;
            image.ImageFailed -= Failed;
            onDone();
        }
        image.ImageOpened += Done;
        image.ImageFailed += Failed;
    }
}

/// <summary>Wpis katalogu paczek ikon STATUS w inspektorze i na stronie Assets.</summary>
public sealed class StatusPackItem : AssetItemBase
{
    private BitmapImage? _preview;

    public string Id { get; set; } = "";
    public uint Size { get; set; }
    /// <summary>Znacznik z Rusta — zmienia się tylko po faktycznej regeneracji miniaturki.</summary>
    public string Fingerprint { get; set; } = "";
    public string Stamp { get; set; } = "";
    public string Details => $"{Size} × {Size}";
    /// <summary>Tylko wbudowany `Classic` jest domyślny — tak jak czarne tło.</summary>
    public bool IsDefault => Id == "Classic";
    public string Badge => IsDefault ? L("AssetsDefaultBadge") : "";
    public BitmapImage? Preview
    {
        get => _preview;
        set { _preview = value; OnChanged(nameof(Preview)); }
    }

    public bool NeedsRefresh => Stamp != AssetStamp.Status(Id) || Preview is null;

    public void RefreshPreview(bool showProgress)
    {
        var stamp = AssetStamp.Status(Id);
        var image = AssetPreview.Load(AppPaths.StatusDirectory, Id, "dnd.png", stamp);
        if (showProgress || !ReferenceEquals(image, Preview))
        {
            IsLoading = true;
            AssetPreview.Observe(image, () => IsLoading = false);
        }
        Stamp = stamp;
        Preview = image;
    }

    private static string L(string key) => LocalizationService.Get(key);
}

/// <summary>Wpis katalogu paczek ikon pogody na stronie Assets.</summary>
public sealed class WeatherPackItem : AssetItemBase
{
    private BitmapImage? _preview;

    public string Id { get; set; } = "";
    public uint Size { get; set; }
    public string Fingerprint { get; set; } = "";
    public string Stamp { get; set; } = "";
    public string Details => $"{Size} × {Size}";
    public bool IsDefault => Id == "Classic";
    public string Badge => IsDefault ? L("AssetsDefaultBadge") : "";

    public BitmapImage? Preview
    {
        get => _preview;
        set { _preview = value; OnChanged(nameof(Preview)); }
    }

    public bool NeedsRefresh => Stamp != AssetStamp.Status("weather-" + Id) || Preview is null;

    public void RefreshPreview()
    {
        var stamp = AssetStamp.Status("weather-" + Id);
        var image = AssetPreview.Load(AppPaths.WeatherDirectory, Id, "preview.png", stamp);
        AssetPreview.Observe(image, () => { });
        Stamp = stamp;
        Preview = image;
    }

    private static string L(string key) => LocalizationService.Get(key);
}

/// <summary>Wpis katalogu zestawów wskazówek na stronie Assets.</summary>
public sealed class HandSetItem : AssetItemBase
{
    private BitmapImage? _preview;

    public string Id { get; set; } = "";
    public uint Size { get; set; }
    public bool HasSecond { get; set; }
    public bool HasCap { get; set; }
    public string Fingerprint { get; set; } = "";
    public string Stamp { get; set; } = "";
    public string Name => Id;
    public string Details => $"{Size} × {Size}"
        + (HasSecond ? $" · {L("HandSecond")}" : "")
        + (HasCap ? $" · {L("HandCap")}" : "");
    public string Badge => Id == "Classic" ? L("AssetsDefaultBadge") : "";
    public BitmapImage? HourPreview
    {
        get => _preview;
        set { _preview = value; OnChanged(nameof(HourPreview)); }
    }

    public bool NeedsRefresh => Stamp != AssetStamp.Hands(Id) || HourPreview is null;

    /// <summary>Złożona miniatura 21:37:11 (preview.png), fallback: hour.png.</summary>
    public void RefreshPreview(bool showProgress)
    {
        var stamp = AssetStamp.Hands(Id);
        var image = AssetPreview.Load(AppPaths.HandsDirectory, Id, "hour.png", stamp);
        if (showProgress || !ReferenceEquals(image, HourPreview))
        {
            IsLoading = true;
            AssetPreview.Observe(image, () => IsLoading = false);
        }
        Stamp = stamp;
        HourPreview = image;
    }

    private static string L(string key) => LocalizationService.Get(key);
}

/// <summary>Obrazki wskazówek w podglądzie tarczy analogowej.</summary>
public sealed class AnalogHandsView
{
    public Image Hour { get; set; } = new();
    public Image Minute { get; set; } = new();
    public Image? Second { get; set; }
    public Image? Cap { get; set; }
}

/// <summary>Grupa katalogu widgetów (np. Czas) — zwijalna sekcja w flyoucie i na zakładce Widgets.</summary>
public sealed class WidgetCatalogGroup
{
    public string Category { get; set; } = "";
    public string Title { get; set; } = "";
    public List<WidgetCatalogItem> Items { get; set; } = new();
    public Expander? FlyoutExpander { get; set; }
    public ListView? FlyoutList { get; set; }
}

/// <summary>Ustawienia wyświetlacza czasu (widgetu cyfrowego).</summary>
public sealed class TimeSettings
{
    public string Pattern { get; set; } = "HH:MM:SS";
    public uint TextSize { get; set; } = 64;
    /// <summary>Pusta wartość = czcionka systemowa zegarka.</summary>
    public string Font { get; set; } = "";
    public string Color { get; set; } = "0xFFFFFFFF";
    public int X { get; set; } = 23;
    public int Y { get; set; } = 183;
    public int W { get; set; } = 420;
    public int H { get; set; } = 100;
    public string Align { get; set; } = "center";
    public string VAlign { get; set; } = "center";
    public bool AutoScale { get; set; }
    public string TextStyle { get; set; } = "ellipsis";
    /// <summary>Treść widgetu tekstowego wpisanego przez użytkownika.</summary>
    public string UserText { get; set; } = "";
    /// <summary>Czy widget dystansu używa własnego systemu jednostek.</summary>
    public bool OwnMeasurementSystem { get; set; }
    /// <summary>System jednostek wybrany w inspektorze: metric albo imperial.</summary>
    public string MeasurementSystem { get; set; } = "watch";
    /// <summary>Czy pokazywać jednostkę w widgetach zdrowotnych.</summary>
    public bool ShowUnit { get; set; } = true;
    /// <summary>Krycie widgetu w zakresie 0–255.</summary>
    public byte Alpha { get; set; } = 255;
    /// <summary>Tryb dopasowania obrazka widgetu IMAGES: cover, contain, stretch.</summary>
    public string ImageFit { get; set; } = "contain";
    /// <summary>Wybrana paczka ikon pogody (Classic albo paczka użytkownika).</summary>
    public string WeatherIconSet { get; set; } = "Classic";
    /// <summary>Dzień prognozy: 0 = dziś, 4 = +4 dni.</summary>
    public byte WeatherDayOffset { get; set; }
    /// <summary>Faza snu: duration (minuty) albo percent (%).</summary>
    public string SleepStageMode { get; set; } = "duration";
    /// <summary>Ustawienia łuku/paska postępu.</summary>
    public ProgressSettingsView Progress { get; set; } = new();
}

/// <summary>Ramka tekstowego widgetu w podglądzie wraz z jego blokiem tekstu.</summary>
public sealed class FrameView
{
    public LayerView Layer { get; set; } = new();
    /// <summary>Kontener o wymiarach ramki — dopiero w nim VerticalAlignment tekstu ma sens.</summary>
    public Grid Host { get; set; } = new();
    public Border Frame { get; set; } = new();
    public TextBlock Text { get; set; } = new();
    /// <summary>Obraz widgetu IMAGES — rozmiar i źródło odświeżane z geometrią ramki.</summary>
    public Microsoft.UI.Xaml.Controls.Image? Picture { get; set; }
    /// <summary>Elementy rysujące łuk/pasek postępu w edytorze.</summary>
    public ProgressVisual? Progress { get; set; }
    public List<Border> Handles { get; } = new();
}

/// <summary>Żywotne elementy widgetu postępu w podglądzie edytora.</summary>
public sealed class ProgressVisual
{
    public Microsoft.UI.Xaml.Shapes.Shape? Track { get; set; }
    public Microsoft.UI.Xaml.Shapes.Shape? Fill { get; set; }
    public List<Microsoft.UI.Xaml.Shapes.Shape> Segments { get; } = new();
}

/// <summary>Widok tarczy zbudowany z tych samych danych co podgląd edytora.</summary>
public sealed class WatchfaceVisual
{
    public Dictionary<string, TextBlock> Texts { get; } = new();
    public Dictionary<string, AnalogHandsView> Hands { get; } = new();
    public Dictionary<string, FrameView> Frames { get; } = new();
}

/// <summary>Wybór z listy w inspektorze (dopasowanie tekstu, pozycja pionowa).</summary>
public sealed class ChoiceItem
{
    public string Value { get; set; } = "";
    public string Name { get; set; } = "";
}

/// <summary>Wpis katalogu czcionek na stronie Assets.</summary>
public sealed class FontItem
{
    public string File { get; set; } = "";
    public string Family { get; set; } = "";
    public bool IsDefault { get; set; }
    public string Badge => IsDefault ? L("AssetsDefaultBadge") : "";
    /// <summary>Podgląd próbki rysowany tą samą czcionką, której użyje tarcza.</summary>
    public FontFamily PreviewFontFamily => PreviewFonts.FromStore(File);

    private static string L(string key) => LocalizationService.Get(key);
}

/// <summary>Wpis katalogu obrazków na stronie Assets.</summary>
public sealed class ImageItem : AssetItemBase
{
    public string File { get; set; } = "";
    public uint Width { get; set; }
    public uint Height { get; set; }
    public ulong Bytes { get; set; }
    /// <summary>Magazyn, z którego pochodzi plik (Images).</summary>
    public string Directory { get; set; } = AppPaths.ImagesDirectory;
    public string Name => Path.GetFileNameWithoutExtension(File);
    public string DisplayName => Name;
    public string Badge => "";
    public string Dimensions => $"{Width} × {Height}";
    public string Details => $"{Dimensions} · {SizeText}";
    public string SizeText => Bytes >= 1024 * 1024
        ? $"{Bytes / (1024.0 * 1024.0):0.0} MB"
        : $"{Bytes / 1024.0:0} kB";
    public string Stamp { get; set; } = "";
    private BitmapImage? _preview;
    public BitmapImage? Preview
    {
        get => _preview;
        set { _preview = value; OnChanged(nameof(Preview)); }
    }

    public bool NeedsRefresh => Stamp != AssetStamp.Image(Directory, File) || Preview is null;

    public void RefreshPreview(bool showProgress)
    {
        var stamp = AssetStamp.Image(Directory, File);
        var image = AssetPreview.LoadFile(Path.Combine(Directory, File), stamp, 152);
        if (showProgress || !ReferenceEquals(image, Preview))
        {
            IsLoading = true;
            AssetPreview.Observe(image, () => IsLoading = false);
        }
        Stamp = stamp;
        Preview = image;
    }
}

/// <summary>Stan gestu przeciągania ramki — wartości z początku gestu idą do Rusta.</summary>
public sealed class FrameDragState
{
    public string LayerId { get; set; } = "";
    public string Handle { get; set; } = "move";
    public Windows.Foundation.Point StartPointer { get; set; }
    public int StartX { get; set; }
    public int StartY { get; set; }
    public int StartW { get; set; }
    public int StartH { get; set; }
    public uint StartTextSize { get; set; }
    public uint StartThickness { get; set; }
    public bool AutoScale { get; set; }
}

public sealed partial class MainWindow : Window
{
    /// <summary>Kafelki paczek ikon pogody na stronie Assets (x:Bind w XAML).</summary>
    public ObservableCollection<WeatherPackItem> WeatherPackItems => _weatherPackItems;

    private const int ScreenSize = 466;
    /// <summary>Musi być zgodne z `rusty_core::date_format::DEFAULT_PATTERN`.</summary>
    private const string DefaultDatePattern = "MMM D";
    /// <summary>Musi być zgodne z `rusty_core::widgets::SNAP_GRID`.</summary>
    private const int GridStep = 16;
    /// <summary>Uchwyty są na rogach ramki, wycentrowane na jej krawędziach.</summary>
    private const double HandleSize = 10;

    private string? _projectFile;
    private string _projectName = "";
    private RecentProjectView? _selectedRecent;
    private TimeSettings? _settings;
    private List<LayerView> _layers = new();
    private readonly Dictionary<string, FrameView> _frames = new();
    private FrameDragState? _drag;
    private string? _lastFramePressId;
    private long _lastFramePressAt;
    private List<WidgetCatalogItem> _widgetCatalog = new();
    private List<WidgetCatalogGroup> _widgetGroups = new();
    private readonly ObservableCollection<HandSetItem> _handItems = new();
    private readonly ObservableCollection<StatusPackItem> _statusItems = new();
    private readonly ObservableCollection<WeatherPackItem> _weatherPackItems = new();
    private string? _selectedWeatherPackId;
    private bool _handsBound;
    private bool _statusBound;
    private string _fontSignature = "";
    private int _assetsRefreshVersion;
    private List<StatusPackDto> _statusPacks = new();
    private List<WeatherPackDto> _weatherPacks = new();
    private List<FontAssetDto> _fonts = new();
    private string? _selectedFontFile;
    private List<ImageAssetDto> _images = new();
    private string? _selectedImageFile;
    private List<HandSetDto> _handSets = new();
    private string? _selectedHandSetId;
    private string? _selectedStatusPackId;
    private readonly Dictionary<string, AnalogHandsView> _analogHands = new();
    private string? _selectedLayerId;
    /// <summary>Użytkownik świadomie odznaczył warstwę — nie wybieraj automatycznie pierwszej.</summary>
    private bool _selectionCleared;
    private readonly DispatcherTimer _clockTimer;
    // Predefiniowany moment pobierany raz z Rusta — na żywo tick go nie dotyka.
    private PreviewDateTimeDto? _predefinedDateTime;
    private readonly LocalizationBridge _localizationBridge;
    private bool _initialized;
    private bool _loadingSettings;
    private bool _loadingFitOptions;
    private bool _loadingFontChoices;
    private bool _loadingWatchfaceLanguage;
    private bool _loadingUnits;
    private bool _loadingFirstDay;
    private bool _loadingWidgetOptions;
    private bool _loadingProgress;
    private bool _loadingImages;
    private bool _loadingHands;
    private bool _loadingStatusPacks;
    private bool _loadingWeatherPacks;
    // Skalowanie zasobów (29 ikon pogody, wskazówki, STATUS, obrazki) odbywa się
    // w tle z debounce — żadny ruch myszy ani suwaka nie blokuje wątku UI.
    private CancellationTokenSource? _assetJobCts;
    private Task _assetJob = Task.CompletedTask;
    // Przełączanie paczki pogody: podgląd zmieniamy natychmiast (jedna ikona z
    // magazynu), a kopiowanie 29 plików do projektu idzie w tle. Bramka daje
    // "ostatni wybór wygrywa" — szybkie przełączania nie kolejkują starych paczek.
    private CancellationTokenSource? _weatherPackCts;
    // Wspólna bramka dla wszystkich zapisów projektu z wątka tła (skalowanie
    // zasobów i przełączanie paczek) — zapobiega zgubieniu jednej z zmian.
    private readonly SemaphoreSlim _assetGate = new(1, 1);
    private int _weatherPackVersion;
    private Task _weatherPackJob = Task.CompletedTask;
    // Wartość X/Y: jedno kliknięcie kopiuje (z małym opóźnieniem, żeby podwójne
    // kliknięcie zdążyło je anulować), podwójne otwiera pole do wpisania liczby.
    private readonly DispatcherTimer _positionTapTimer = new()
    {
        Interval = TimeSpan.FromMilliseconds(250),
    };
    private TextBlock? _pendingPositionValue;
    // Wybór użytkownika zapamiętujemy także wtedy, gdy wybrany zestaw nie ma
    // danego pliku — powrót do zestawu z pełnym zestawem go przywróci.
    private bool _hourHandEnabled = true;
    private bool _minuteHandEnabled = true;
    private bool _secondHandEnabled = true;
    private bool _capHandEnabled = true;
    private string? _projectSettingsError;
    private Process? _emulatorProcess;

    private static string L(string key) => LocalizationService.Get(key);
    private static string LF(string key, params object[] args) => LocalizationService.Format(key, args);

    public MainWindow()
    {
        AppPaths.Initialize();
        // Pełna struktura %APPDATA% powstaje przy pierwszym starcie — bez czekania
        // aż użytkownik wejdzie w Assets. Błąd nie blokuje startu (seed jest idempotentny).
        RustNative.EnsureDefaults(out _);
        LocalizationService.Initialize();
        _localizationBridge = new LocalizationBridge();

        InitializeComponent();
        Closed += MainWindow_Closed;

        // Przeciąganie ramki: chwytamy wskaźnik na warstwie ramek (poza klipem tarczy),
        // nie na przesuwanej ramce. Dzięki temu uchwyty na rogach zawsze działają.
        OutlineCanvas.PointerMoved += OutlineCanvas_PointerMoved;
        OutlineCanvas.PointerReleased += OutlineCanvas_PointerReleased;
        OutlineCanvas.PointerCanceled += OutlineCanvas_PointerCanceled;

        // Elementy w PaneFooter nie biorą udziału w SelectionChanged NavigationView —
        // obsługujemy ich kliknięcie bezpośrednio (także gdy kontrolka oznaczy Tapped jako handled).
        AppSettingsNavItem.AddHandler(UIElement.TappedEvent, new TappedEventHandler(FooterItem_Tapped), true);
        ProjectSettingsNavItem.AddHandler(UIElement.TappedEvent, new TappedEventHandler(FooterItem_Tapped), true);

        RootGrid.DataContext = _localizationBridge;
        NewProjectDialog.DataContext = _localizationBridge;
        WidgetFlyoutRoot.DataContext = _localizationBridge;
        Title = L("AppTitle");
        ProjectNameText.Text = L("NavHome");
        LanguageCombo.ItemsSource = LocalizationService.Languages;
        LanguageCombo.SelectedItem = LocalizationService.Current;
        DefaultAuthorBox.Text = AppSettingsStore.DefaultAuthor;
        DefaultAuthorBox.PlaceholderText = LF("DefaultAuthorPlaceholder", Environment.UserName);
        BuildGridLines();
        PreviewGridToggle.IsChecked = AppSettingsStore.SnapToGrid;
        UnselectedOutlineToggle.IsChecked = AppSettingsStore.ShowUnselectedOutline;
        LivePreviewToggle.IsChecked = AppSettingsStore.LiveTimeDateEnabled;
        GridLinesCanvas.Visibility = AppSettingsStore.SnapToGrid ? Visibility.Visible : Visibility.Collapsed;
        RefreshPreviewOptionLabels();
        EditorStatusText.Text = L("AutoSave");

        AppWindow.Resize(new Windows.Graphics.SizeInt32(1280, 800));
        // Ikona okna — ten sam znak co ikona exe (Assets\AppIcon.ico, kopiowana do wyjścia).
        var appIcon = Path.Combine(AppContext.BaseDirectory, "Assets", "AppIcon.ico");
        if (File.Exists(appIcon)) AppWindow.SetIcon(appIcon);
        SystemBackdrop = new MicaBackdrop();

        var titleBar = AppWindow.TitleBar;
        titleBar.BackgroundColor = Color.FromArgb(255, 28, 28, 32);
        titleBar.ButtonBackgroundColor = Color.FromArgb(255, 28, 28, 32);
        titleBar.ButtonHoverBackgroundColor = Color.FromArgb(255, 48, 48, 56);
        titleBar.ButtonPressedBackgroundColor = Color.FromArgb(255, 60, 60, 70);
        titleBar.ForegroundColor = Color.FromArgb(255, 255, 255, 255);
        titleBar.ButtonForegroundColor = Color.FromArgb(255, 255, 255, 255);

        _clockTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
        _clockTimer.Tick += ClockTimer_Tick;
        _clockTimer.Start();

        ProjectsFolderBox.Text = AppSettingsStore.ProjectsDirectory;

        var version = RustNative.Version(out var error);
        if (version is not null)
        {
            using var document = JsonDocument.Parse(version);
            var root = document.RootElement;
            var versionText = root.GetProperty("version").GetString() ?? "?";
            HomeVersionText.Text = $"Rusty Watch {versionText}";
        }
        else
        {
            ShowStatus(error is null ? L("ErrorVersion") : LocalizationService.Error(error), InfoBarSeverity.Error);
        }

        RefreshRecent();
        LoadTextFitOptions();
        LoadWatchfaceLanguageOptions();
        LoadMeasurementSystemOptions();
        LoadFirstDayOfWeekOptions();
        LoadDistanceUnitOptions();
        LoadWeatherDayOptions();
        LoadSleepStageOptions();
        LoadWidgetCatalog();
        LoadFonts();
        LoadImages();
        LoadHandSets(); LoadStatusPacks(); LoadWeatherPacks();
        ShowView(editor: false);
        _initialized = true;
    }

    private IntPtr WindowHandle => WindowNative.GetWindowHandle(this);

    private void ShowStatus(string message, InfoBarSeverity severity = InfoBarSeverity.Success)
    {
        StatusInfoBar.Message = message;
        StatusInfoBar.Severity = severity;
        StatusInfoBar.IsOpen = true;
    }

    /// <summary>Przełącza między stroną Home, edytorem, widgetami i ustawieniami.</summary>
    private void ShowView(bool editor, bool settings = false, bool projectSettings = false,
        bool widgets = false, bool assets = false)
    {
        HomeView.Visibility = (editor || settings || projectSettings || widgets || assets)
            ? Visibility.Collapsed : Visibility.Visible;
        EditorView.Visibility = editor ? Visibility.Visible : Visibility.Collapsed;
        WidgetsView.Visibility = widgets ? Visibility.Visible : Visibility.Collapsed;
        AssetsView.Visibility = assets ? Visibility.Visible : Visibility.Collapsed;
        SettingsView.Visibility = settings ? Visibility.Visible : Visibility.Collapsed;
        ProjectSettingsView.Visibility = projectSettings ? Visibility.Visible : Visibility.Collapsed;
        var projectOpen = _projectFile is not null;
        EditorNavItem.Visibility = projectOpen ? Visibility.Visible : Visibility.Collapsed;
        ProjectSettingsNavItem.Visibility = projectOpen ? Visibility.Visible : Visibility.Collapsed;
        // Widgets jest widoczny zawsze — katalog można przeglądać też bez otwartego projektu.
        SaveButton.IsEnabled = projectOpen;
        SaveAsButton.IsEnabled = projectOpen;
        ExportButton.IsEnabled = editor && projectOpen;
        TestEmulatorButton.IsEnabled = editor && projectOpen;
    }

    private void RefreshRecent()
    {
        var raw = RustNative.RecentList(AppPaths.RecentStorePath, out var error);
        var items = new ObservableCollection<RecentProjectView>();

        if (raw is not null)
        {
            using var document = JsonDocument.Parse(raw);
            foreach (var element in document.RootElement.EnumerateArray())
            {
                var file = element.GetProperty("file").GetString() ?? "";
                items.Add(new RecentProjectView
                {
                    Name = element.GetProperty("name").GetString() ?? Path.GetFileName(file),
                    Folder = Path.GetDirectoryName(file) ?? file,
                    File = file,
                    OpenedText = FormatOpened(element.GetProperty("openedAt").GetInt64()),
                });
            }
        }
        else
        {
            ShowStatus(error is null ? L("ErrorRecentList") : LocalizationService.Error(error), InfoBarSeverity.Error);
        }

        RecentList.ItemsSource = items;
        RecentEmptyState.Visibility = items.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        RecentList.Visibility = items.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
        ClearRecentButton.IsEnabled = items.Count > 0;
        _selectedRecent = null;
    }

    private static string FormatOpened(long millis)
    {
        var time = DateTimeOffset.FromUnixTimeMilliseconds(millis).LocalDateTime;
        var today = DateTime.Today;
        if (time.Date == today) return LF("TodayAt", time.ToString("HH:mm"));
        if (time.Date == today.AddDays(-1)) return LF("YesterdayAt", time.ToString("HH:mm"));
        var days = (today - time.Date).Days;
        if (days > 0 && days < 7) return LF("DaysAgo", days);
        return time.ToString("d MMM yyyy", CultureInfo.CurrentCulture);
    }

    /// <summary>Wczytuje projekt przez Rust i dopisuje go do listy ostatnich.</summary>
    private bool OpenProjectFile(string file)
    {
        var data = RustNative.OpenProject(AppPaths.RecentStorePath, file, out var error);
        if (data is null)
        {
            ShowStatus(error is null ? L("ErrorOpenProject") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return false;
        }

        using var document = JsonDocument.Parse(data);
        var project = document.RootElement.GetProperty("project");
        var name = project.GetProperty("metadata").GetProperty("name").GetString()
                   ?? Path.GetFileName(Path.GetDirectoryName(file)) ?? L("NavHome");

        _projectFile = file;
        _projectName = name;
        _selectionCleared = false;
        // Starsze projekty mogą mieć widgety STATUS bez przypisanej paczki.
        RustNative.AttachStatusPackToMissing(file, out _);
        ProjectNameText.Text = name;
        LoadLayers();
        // Ustawienia aplikacji obowiązują w każdym otwartym projekcie (wpływają na JS).
        ApplyAppMeasurementSystemToProject();
        ApplyAppFirstDayOfWeekToProject();
        ApplyDefaultFontToMissing();
        LoadImages();
        LoadHandSets(); LoadStatusPacks(); LoadWeatherPacks();
        LoadProjectSettings();
        ShowView(editor: true);
        RefreshRecent();
        if (_initialized) NavView.SelectedItem = EditorNavItem;
        return true;
    }

    // --- Edytor: inspektor zaznaczonego widgetu -----------------------------

    private void ApplyPreview()
    {
        if (_settings is null || _selectedLayerId is null) return;
        if (_frames.TryGetValue(_selectedLayerId, out var view))
        {
            var layer = view.Layer;
            layer.X = _settings.X;
            layer.Y = _settings.Y;
            layer.W = _settings.W;
            layer.H = _settings.H;
            layer.TextSize = _settings.TextSize;
            layer.Font = _settings.Font;
            layer.Color = _settings.Color;
            layer.Align = _settings.Align;
            layer.VAlign = _settings.VAlign;
            layer.AutoScale = _settings.AutoScale;
            layer.TextStyle = _settings.TextStyle;
            layer.Alpha = _settings.Alpha;
            view.Host.Opacity = _settings.Alpha / 255.0;

            ApplyFrameGeometry(view, _settings.X, _settings.Y, _settings.W, _settings.H, _settings.TextSize);
            // Zegar odświeża tylko tekst czasu/daty — stały tekst użytkownika
            // aktualizujemy od razu przy każdej zmianie w inspektorze.
            if (layer.Kind == "text") view.Text.Text = _settings.UserText;
            view.Text.Foreground = new SolidColorBrush(ParseArgb(_settings.Color));
            ApplyTextFit(view, _settings.TextStyle);
            ApplyFont(view);
        }
        PreviewPatternText.Text = LF("FormatLabel", _settings.Pattern);
        UpdateFrameVisuals();
        UpdateClockText();
    }

    /// <summary>Podgląd używa dokładnie tego samego pliku TTF co eksportowana tarcza.</summary>
    private string ProjectFontsDirectory => _projectFile is null
        ? ""
        : Path.Combine(Path.GetDirectoryName(_projectFile) ?? "", "assets", "fonts");

    private void ApplyFont(FrameView view) => ApplyFont(view.Text, view.Layer.Font);

    private void ApplyFont(TextBlock text, string? font)
    {
        text.FontFamily = string.IsNullOrWhiteSpace(font)
            ? new FontFamily("Segoe UI")
            : PreviewFonts.Get(font, ProjectFontsDirectory);
    }

    private static TextAlignment AlignOf(string? align) => align switch
    {
        "left" => TextAlignment.Left,
        "right" => TextAlignment.Right,
        _ => TextAlignment.Center,
    };

    private static VerticalAlignment VerticalOf(string? align) => align switch
    {
        "top" => VerticalAlignment.Top,
        "bottom" => VerticalAlignment.Bottom,
        _ => VerticalAlignment.Center,
    };

    /// <summary>
    /// Jedyne miejsce ustawiające kotwicę tekstu. Wołane razem z geometrią, żeby
    /// wyrównanie nie mogło zostać z poprzedniego stanu ramki / inspektora.
    /// </summary>
    private static void ApplyTextLayout(FrameView view)
    {
        var layer = view.Layer;
        var text = view.Text;
        // TextAlignment działa wewnątrz Width ramki; VerticalAlignment dopiero
        // wtedy, gdy pole tekstu ma wysokość treści (Height = NaN) i MaxHeight = h.
        text.HorizontalAlignment = HorizontalAlignment.Left;
        text.Width = view.Host.Width;
        text.Height = double.NaN;
        text.MaxHeight = view.Host.Height;
        text.TextAlignment = AlignOf(layer.Align);
        text.VerticalAlignment = VerticalOf(layer.VAlign);
    }

    /// <summary>
    /// Warstwy oparte na obrazach (wskazówki zegara, przyszłe widgety graficzne).
    /// W niebieskim trybie można je tylko przesuwać — skalowanie wymaga czerwonego trybu.
    /// </summary>
    private static bool IsImageLayer(LayerView layer) => layer.Kind is "analog-time" or "image" or "images"
        || IsStatusKind(layer.Kind) || IsWeatherIcon(layer);

    /// <summary>
    /// Host i ramka mają dokładnie w/h; tekst ma szerokość ramki i automatyczną
    /// wysokość (ograniczoną MaxHeight), żeby wyrównanie liczyło się od aktualnej ramki.
    /// Bez UI-only marginesów i bez sztywnego Height.
    /// </summary>
    /// <summary>Promień linii środkowej — nie zależy od grubości (center stays).</summary>
    private static double ArcCenterRadius(int w, int h)
        => Math.Max(4, Math.Min(w, h) / 2.0 - 10);

    /// <summary>Margines na grubą linię, żeby Path miał miejsce poza ramką.</summary>
    private static double ArcStrokeMargin(double thickness)
        => Math.Max(0, thickness / 2.0 - 8) + 6;

    private static Point ArcPoint(double cx, double cy, double radius, double degrees)
    {
        var radians = degrees * Math.PI / 180.0;
        // 0° = góra, zgodnie z ARC_PROGRESS w tarczy.
        return new Point(cx + radius * Math.Sin(radians), cy - radius * Math.Cos(radians));
    }

    private static Geometry ArcGeometry(double cx, double cy, double radius, double start, double end)
    {
        var sweep = end - start;
        // ArcSegment nie potrafi narysować pełnego okręgu (start == koniec) —
        // wtedy ZeppOS i tak rysuje ARC_PROGRESS 0→360 jako zamknięty łuk.
        if (Math.Abs(sweep) >= 359.999)
            return new EllipseGeometry
            {
                Center = new Point(cx, cy),
                RadiusX = radius,
                RadiusY = radius,
            };
        var figure = new PathFigure { StartPoint = ArcPoint(cx, cy, radius, start) };
        figure.Segments.Add(new ArcSegment
        {
            Point = ArcPoint(cx, cy, radius, end),
            Size = new Size(radius, radius),
            IsLargeArc = Math.Abs(sweep) > 180.0,
            SweepDirection = sweep >= 0 ? SweepDirection.Clockwise : SweepDirection.Counterclockwise,
        });
        return new PathGeometry { Figures = new PathFigureCollection { figure } };
    }

    /// <summary>"Zaokrąglone końce" wyłączone = płaskie kapki i radius 0.</summary>
    /// <summary>Zaokrąglenia są włączone lub wyłączone — kwadrat staje się kołem.</summary>
    private static double EffectiveProgressRadius(int w, int h, ProgressSettingsView p)
        => p.Corner == "flat" ? 0.0 : Math.Min(w, h) / 2.0;

    private static PenLineCap ProgressLineCap(ProgressSettingsView p)
        => p.Corner == "flat" ? PenLineCap.Flat : PenLineCap.Round;

    private static Microsoft.UI.Xaml.Shapes.Shape CreateProgressShape(bool arc, LayerView layer, ProgressSettingsView p, bool track)
    {
        var color = ParseArgb(track ? p.TrackColor : p.Color);
        var brush = new SolidColorBrush(color);
        if (arc)
        {
            var radius = ArcCenterRadius(layer.W, layer.H);
            var start = track ? p.StartAngle : p.StartAngle;
            var end = track ? p.EndAngle : p.StartAngle;
            var offset = ArcStrokeMargin(p.Thickness);
            return new Microsoft.UI.Xaml.Shapes.Path
            {
                Stroke = brush,
                StrokeThickness = p.Thickness,
                StrokeStartLineCap = ProgressLineCap(p),
                StrokeEndLineCap = ProgressLineCap(p),
                Stretch = Stretch.None,
                Width = layer.W + 2 * offset,
                Height = layer.H + 2 * offset,
                Margin = new Thickness(-offset),
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Top,
                Clip = null,
                Data = ArcGeometry(layer.W / 2.0 + offset, layer.H / 2.0 + offset, radius, start, end),
            };
        }
        var cornerRadius = EffectiveProgressRadius(layer.W, layer.H, p);
        return new Microsoft.UI.Xaml.Shapes.Rectangle
        {
            Fill = brush,
            RadiusX = cornerRadius,
            RadiusY = cornerRadius,
            // Rodzicem jest Grid — Canvas.Left/Top nie ma tu żadnego efektu.
            // Wypełnienie kotwimy alignmentem: poziomo do lewej, pionowo do dołu.
            HorizontalAlignment = track || p.Orientation == "vertical"
                ? HorizontalAlignment.Stretch : HorizontalAlignment.Left,
            VerticalAlignment = track || p.Orientation != "vertical"
                ? VerticalAlignment.Stretch : VerticalAlignment.Bottom,
        };
    }

    /// <summary>Predefiniowana wartość czujnika do podglądu postępu na PC.</summary>
    private static double ProgressPreviewValue(ProgressSettingsView p)
    {
        var custom = AppSettingsStore.GetCustomPreviewValue(p.Source);
        if (custom is not null && double.TryParse(custom, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed))
            return parsed;
        var fallback = p.Source switch
        {
            "steps" => 16000.0,
            "distance" => 12.3,
            "heart" => 72.0,
            "calories" => 430.0,
            "stand" => 8.0,
            "fat-burning" => 32.0,
            "pai" => 42.0,
            "spo2" => 97.0,
            "sleep" => 425.0,
            "fixed" => p.Value,
            // Bateria domyślnie 73% — realistyczny stan, nie pełne 100%.
            "battery" => 73.0,
            _ => 73.0,
        };
        return fallback;
    }

    private static double ProgressFraction(LayerView layer)
    {
        var p = layer.Progress ?? new ProgressSettingsView();
        var raw = ProgressPreviewValue(p);
        var hi = p.Max > p.Min ? p.Max : p.Source switch
        {
            "steps" => 20000.0,
            "calories" => 600.0,
            "stand" => 12.0,
            "fat-burning" => 60.0,
            "sleep" => 480.0,
            _ => 100.0,
        };
        var value = hi > p.Min ? (raw - p.Min) / (hi - p.Min) : 0.0;
        if (p.Invert) value = 1.0 - value;
        return Math.Max(0.0, Math.Min(1.0, value));
    }

    private void UpdateProgressVisual(FrameView view)
    {
        var visual = view.Progress;
        if (visual is null) return;
        var layer = view.Layer;
        var p = layer.Progress ?? new ProgressSettingsView();
        var value = ProgressFraction(layer);
        var arc = layer.Kind == "arc-progress";
        var radius = ArcCenterRadius(layer.W, layer.H);
        var offset = arc ? ArcStrokeMargin(p.Thickness) : 0;
        var cx = layer.W / 2.0 + offset;
        var cy = layer.H / 2.0 + offset;
        var sweep = p.EndAngle - p.StartAngle;
        if (arc)
        {
            var arcTrack = visual.Track as Microsoft.UI.Xaml.Shapes.Path;
            var arcFill = visual.Fill as Microsoft.UI.Xaml.Shapes.Path;
            foreach (var path in new[] { arcTrack, arcFill })
            {
                if (path is null) continue;
                path.Width = layer.W + 2 * offset;
                path.Height = layer.H + 2 * offset;
                path.Margin = new Thickness(-offset);
                path.Clip = null;
            }
        }

        if (visual.Track is not null)
        {
            // ShowTrack musi działać na żywo w edytorze — nie tylko po eksporcie.
            visual.Track.Visibility = p.ShowTrack ? Visibility.Visible : Visibility.Collapsed;
            if (arc && visual.Track is Microsoft.UI.Xaml.Shapes.Path trackPath)
            {
                trackPath.Data = ArcGeometry(cx, cy, radius, p.StartAngle, p.EndAngle);
                trackPath.StrokeThickness = p.Thickness;
            }
            else if (!arc && visual.Track is Microsoft.UI.Xaml.Shapes.Rectangle trackRect)
            {
                trackRect.Width = layer.W;
                trackRect.Height = layer.H;
                trackRect.RadiusX = EffectiveProgressRadius(layer.W, layer.H, p);
                trackRect.RadiusY = trackRect.RadiusX;
            }
        }
        // Zawsze jedno ciągłe wypełnienie — tak samo jak w eksportowanym JS.
        if (visual.Fill is Microsoft.UI.Xaml.Shapes.Path fillPath)
        {
            fillPath.Data = ArcGeometry(cx, cy, radius, p.StartAngle, p.StartAngle + sweep * value);
            fillPath.StrokeThickness = p.Thickness;
            fillPath.StrokeStartLineCap = ProgressLineCap(p);
            fillPath.StrokeEndLineCap = ProgressLineCap(p);
        }
        else if (visual.Fill is Microsoft.UI.Xaml.Shapes.Rectangle fillRect)
        {
            var rounded = EffectiveProgressRadius(layer.W, layer.H, p);
            fillRect.RadiusX = Math.Min(rounded, Math.Max(1, Math.Min(layer.W, layer.H) / 2));
            fillRect.RadiusY = fillRect.RadiusX;
            if (p.Orientation == "vertical")
            {
                // Pełny pasek idzie od dołu do góry.
                fillRect.Width = layer.W;
                fillRect.Height = Math.Max(0, layer.H * value);
                fillRect.HorizontalAlignment = HorizontalAlignment.Stretch;
                fillRect.VerticalAlignment = VerticalAlignment.Bottom;
            }
            else
            {
                // Pusty = lewo, pełny = prawo. Lewa krawędź jest zawsze zakotwiczona
                // (Canvas.Left nie działa w Grid — stąd HorizontalAlignment.Left).
                fillRect.Width = Math.Max(0, layer.W * value);
                fillRect.Height = layer.H;
                fillRect.HorizontalAlignment = HorizontalAlignment.Left;
                fillRect.VerticalAlignment = VerticalAlignment.Top;
            }
        }
    }

    private void ApplyFrameGeometry(FrameView view, int x, int y, int w, int h, uint textSize)
        => ApplyFrameGeometry(view, x, y, w, h, textSize, _analogHands);

    private void ApplyFrameGeometry(FrameView view, int x, int y, int w, int h, uint textSize,
        IReadOnlyDictionary<string, AnalogHandsView> hands)
    {
        view.Host.Width = w;
        view.Host.Height = h;
        view.Frame.Width = w;
        view.Frame.Height = h;
        // Pole tekstu ma zawsze szerokość ramki (w niej działa TextAlignment),
        // ale wysokość automatyczną — wtedy VerticalAlignment pozycjonuje treść
        // względem AKTUALNEJ wysokości ramki, a nie względem starego bounding
        // boxa sprzed skalowania. MaxHeight domyka tekst w ramce, więc nawet gdy
        // czcionka jest większa niż ramka, tekst nie ucieknie poza Canvas/Outline.
        view.Text.Width = w;
        view.Text.Height = double.NaN;
        view.Text.MaxHeight = h;
        view.Text.FontSize = textSize;
        if (view.Picture is not null)
        {
            view.Picture.Width = w;
            view.Picture.Height = h;
            if (!string.IsNullOrEmpty(view.Layer.Src) && _projectFile is not null)
            {
                var picturePath = Path.Combine(
                    Path.GetDirectoryName(_projectFile) ?? "", "assets", view.Layer.Src);
                // Bitmapa jest dekodowana raz. Podczas przeciągania zmieniamy tylko
                // Width/Height i Stretch.Fill — dokładnie jedna widoczna ikona jest
                // skalowana przez kontrolkę, bez odczytu pliku w każdej klatce.
                // Źródłem ikony pogody jest oryginał z magazynu wskazywany przez
                // `rw_format_weather_field` — nie nadpisujemy go ścieżką projektu.
                if (view.Layer.Kind != "weather-icon"
                    && !Equals(view.Picture.Tag as string, picturePath) && File.Exists(picturePath))
                {
                    view.Picture.Source = CachedBitmap(picturePath);
                    view.Picture.Tag = picturePath;
                }
            }
        }
        Canvas.SetLeft(view.Host, x);
        Canvas.SetTop(view.Host, y);
        Canvas.SetLeft(view.Frame, x);
        Canvas.SetTop(view.Frame, y);

        // Wskazówki w podglądzie mają rozmiar ramki — przez to skalowanie jest 1:1
        // z `w/h` zapisanym w projekcie i wysyłanym do widgetów `IMG` na tarczy.
        if (hands.TryGetValue(view.Layer.Id, out var handSet))
        {
            foreach (var hand in new[] { handSet.Hour, handSet.Minute, handSet.Second, handSet.Cap })
            {
                if (hand is null) continue;
                hand.Width = w;
                hand.Height = h;
            }
        }

        // Uchwyty siedzą dokładnie na rogach ramki (nie w jej wnętrzu). Ramka jest
        // rysowana poza klipem tarczy, więc nawet rogi dużych wskazówek można złapać.
        var half = HandleSize / 2;
        var arcRadius = ArcCenterRadius(w, h);
        foreach (var handle in view.Handles)
        {
            if (handle.Name == "thick")
            {
                // Punkt na łuku (0° = góra) — przeciągnięcie zmienia grubość.
                Canvas.SetLeft(handle, x + w / 2.0 - half);
                Canvas.SetTop(handle, y + h / 2.0 - arcRadius - half);
                continue;
            }
            var west = handle.Name.Contains('w');
            var north = handle.Name.Contains('n');
            Canvas.SetLeft(handle, (west ? x : x + w) - half);
            Canvas.SetTop(handle, (north ? y : y + h) - half);
        }

        // Geometria i wyrównanie to jedna operacja — bez tego ramka mogła się
        // przesunąć, a tekst zachował kotwicę z poprzedniego stanu (np. po
        // skalowaniu lub zmianie justowania w inspektorze).
        ApplyTextLayout(view);
        UpdateProgressVisual(view);
    }

    private static void ApplyTextFit(FrameView view, string? fit) => ApplyTextFit(view.Text, fit);

    private static void ApplyTextFit(TextBlock text, string? fit)
    {
        switch (fit)
        {
            case "none":
                text.TextTrimming = TextTrimming.None;
                text.TextWrapping = TextWrapping.NoWrap;
                break;
            case "wrap":
                text.TextTrimming = TextTrimming.None;
                text.TextWrapping = TextWrapping.Wrap;
                break;
            default:
                text.TextTrimming = TextTrimming.CharacterEllipsis;
                text.TextWrapping = TextWrapping.NoWrap;
                break;
        }
    }

    /// <summary>Zaznaczona ramka jest niebieska, w trybie skalowania czerwona.</summary>
    private void UpdateFrameVisuals()
    {
        var accent = new SolidColorBrush(Color.FromArgb(255, 96, 205, 255));
        var red = new SolidColorBrush(Color.FromArgb(255, 255, 59, 48));
        var subtle = new SolidColorBrush(Color.FromArgb(140, 150, 160, 180));
        var showUnselected = AppSettingsStore.ShowUnselectedOutline;
        foreach (var view in _frames.Values)
        {
            var selected = view.Layer.Id == _selectedLayerId;
            var brush = selected ? (view.Layer.AutoScale ? red : accent) : subtle;
            view.Frame.BorderBrush = brush;
            view.Frame.BorderThickness = new Thickness(selected ? 2 : showUnselected ? 1 : 0);
            // Zaznaczona ramka i jej uchwyty są nad pozostałymi — wybrana na liście
            // warstwa wygrywa hit-testing z warstwą aktualnie najwyższą na tarczy.
            Canvas.SetZIndex(view.Frame, selected ? 100 : 1);
            // Niebieska ramka obrazu służy wyłącznie do przesuwania — nie pokazujemy
            // uchwytów, dopóki dwuklik nie przełączy trybu rozciągania (czerwona).
            var handlesEnabled = view.Layer.Kind != "images" || view.Layer.AutoScale;
            foreach (var handle in view.Handles)
            {
                handle.Background = brush;
                handle.Visibility = (selected || showUnselected) && handlesEnabled
                    ? Visibility.Visible
                    : Visibility.Collapsed;
                Canvas.SetZIndex(handle, selected ? 101 : 2);
            }
        }
    }

    private void SyncPositionSliders()
    {
        if (_settings is null) return;
        _loadingSettings = true;
        SizeSlider.Value = Math.Clamp(_settings.TextSize, SizeSlider.Minimum, SizeSlider.Maximum);
        FontSizeValue.Text = _settings.TextSize.ToString();
        XSlider.Maximum = Math.Max(0, ScreenSize - _settings.W);
        _settings.X = Math.Clamp(_settings.X, 0, (int)XSlider.Maximum);
        XSlider.Value = _settings.X;
        YSlider.Maximum = Math.Max(0, ScreenSize - _settings.H);
        _settings.Y = Math.Clamp(_settings.Y, 0, (int)YSlider.Maximum);
        YSlider.Value = _settings.Y;
        UpdatePositionValues();
        _loadingSettings = false;
    }

    /// <summary>Aktualne wartości X/Y wyświetlane obok suwaków.</summary>
    private void UpdatePositionValues()
    {
        if (_settings is null) return;
        XPositionValue.Text = _settings.X.ToString(CultureInfo.InvariantCulture);
        YPositionValue.Text = _settings.Y.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// Odświeża cały podgląd. Live Preview dotyczy wyłącznie czasu i daty —
    /// wszystkie pozostałe pola zawsze czytają dane predefiniowane.
    /// </summary>
    private void UpdateClockText(DateTime? at = null)
    {
        if (_projectFile is null) return;
        var live = at ?? DateTime.Now;
        var previewNow = AppSettingsStore.LiveTimeDateEnabled ? live : PredefinedDateTime();
        var customFirstDay = AppSettingsStore.GetCustomPreviewValue("firstDay");
        foreach (var view in _frames.Values)
        {
            var layer = view.Layer;
            var fallback = layer.Kind == "date" ? DefaultDatePattern : "HH:MM:SS";
            var pattern = layer.Id == _selectedLayerId && _settings is not null
                ? _settings.Pattern
                : layer.TimeFormat ?? fallback;
            FormatTextForTime(layer.Kind, pattern, view.Text, previewNow, customFirstDay);
            if (IsWeatherKind(layer.Kind))
                ApplyWeatherPreview(layer, view, previewNow);
            else ApplyCustomSensorOverride(layer.Kind, view.Text);
            // Wartość progressu pochodzi z Custom Data — odśwież kształt razem z tekstem.
            if (IsProgressKind(layer.Kind)) UpdateProgressVisual(view);
        }

        // Wskazówki obracamy tym samym tyknięciem zegara, który odświeża tekst.
        foreach (var hands in _analogHands.Values) RotateHandSet(hands, previewNow);
    }

    /// <summary>
    /// Lekkie tyknięcie Live Preview: tylko TIME/DATE, pola pochodne i wskazówki.
    /// Nie dotykamy pogody, czujników, FFI dla danych ani przebudowy canvasu.
    /// </summary>
    private void UpdateLiveTimeDateOnly()
    {
        if (_projectFile is null) return;
        var now = DateTime.Now;
        var customFirstDay = AppSettingsStore.GetCustomPreviewValue("firstDay");
        foreach (var view in _frames.Values)
        {
            var kind = view.Layer.Kind;
            if (kind is not ("time" or "analog-time" or "date") && !IsDateFieldKind(kind)) continue;
            var fallback = kind == "date" ? DefaultDatePattern : "HH:MM:SS";
            var pattern = view.Layer.Id == _selectedLayerId && _settings is not null
                ? _settings.Pattern
                : view.Layer.TimeFormat ?? fallback;
            FormatTextForTime(kind, pattern, view.Text, now, customFirstDay);
        }
        foreach (var hands in _analogHands.Values) RotateHandSet(hands, now);
    }

    /// <summary>Sam moment z Rusta (bez nadpisów z Custom Data) — do placeholderów.</summary>
    private static DateTime DefaultPreviewMoment()
    {
        var preset = RustNative.PreviewDefaultDateTime(out _);
        if (preset is null) return DateTime.Now;
        var year = Math.Clamp(preset.Year, 1, 9999);
        var month = Math.Clamp(preset.Month, 1, 12);
        return new DateTime(year, month,
            Math.Clamp(preset.Day, 1, DateTime.DaysInMonth(year, month)),
            Math.Clamp(preset.Hour, 0, 23),
            Math.Clamp(preset.Minute, 0, 59),
            Math.Clamp(preset.Second, 0, 59));
    }

    /// <summary>Predefiniowany moment z Rusta z nałożonymi polami z Custom Data.</summary>
    private DateTime PredefinedDateTime()
    {
        var preset = _predefinedDateTime ??= RustNative.PreviewDefaultDateTime(out _);
        if (preset is null) return DateTime.Now;
        var year = ClampCustomInt("year", preset.Year, 1, 9999);
        var month = ClampCustomInt("month", preset.Month, 1, 12);
        var day = ClampCustomInt("day", preset.Day, 1, DateTime.DaysInMonth(year, month));
        return new DateTime(year, month, day,
            ClampCustomInt("hour", preset.Hour, 0, 23),
            ClampCustomInt("minute", preset.Minute, 0, 59),
            ClampCustomInt("second", preset.Second, 0, 59));
    }

    private static int ClampCustomInt(string key, int fallback, int min, int max)
    {
        var raw = AppSettingsStore.GetCustomPreviewValue(key);
        return int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? Math.Clamp(value, min, max)
            : fallback;
    }

    /// <summary>Ręczne wartości czujników i pól-placeholderów wyświetlamy dosłownie.</summary>
    private static void ApplyCustomSensorOverride(string kind, TextBlock text)
    {
        var key = CustomValueKeyForKind(kind);
        if (key is null) return;
        var value = AppSettingsStore.GetCustomPreviewValue(key);
        if (value is not null) text.Text = value;
    }

    /// <summary>
    /// Podgląd pogody na żywo. Rust formatuje tekst i podaje ścieżkę ikony z
    /// magazynu; brakujące dane Custom Data wracają do domyślnej, pochmurnej
    /// Warszawy. Tylko widoczna ikona jest skalowana (przez Stretch.Fill kontrolki).
    /// </summary>
    private void ApplyWeatherPreview(LayerView layer, FrameView view, DateTime now)
    {
        // Pogoda zawsze pochodzi z danych predefiniowanych — Live Preview nie
        // przełącza żadnych czujników ani pogody na dane systemowe.
        // `sun-current` dostaje aktualną minutę, żeby liczyć czas do najbliższego
        // wschodu/zachodu zgodnie z tym, co pokazuje tarcza na zegarku. Dla
        // `weather-current` Rust zwraca wartość zastępczą (PC nie ma wiązania
        // firmware'u z hmUI.data_type.WEATHER_CURRENT).
        var day = layer.Kind is "sun-current" or "weather-current" ? (byte)0 : layer.WeatherDayOffset;
        var overrides = WeatherOverridesJson(day, now);
        var raw = RustNative.FormatWeatherField(
            layer.Kind, overrides, EffectiveWatchfaceLanguage(),
            layer.WeatherIconSet ?? "Classic", out var error);
        if (raw is null) return;
        using var document = JsonDocument.Parse(raw);
        var data = document.RootElement;
        // Ikona nie ma tekstu — Rust zwraca dla niej pusty string; nie wpisujemy
        // go do TextBlocku, żeby nawet chwilowy artifact nie pojawił się w ramce.
        if (layer.Kind != "weather-icon" && data.TryGetProperty("text", out var textElement))
            view.Text.Text = textElement.GetString() ?? view.Text.Text;
        if (layer.Kind != "weather-icon" || view.Picture is null
            || !data.TryGetProperty("iconPath", out var iconElement)) return;
        var iconPath = iconElement.GetString();
        if (string.IsNullOrEmpty(iconPath) || !File.Exists(iconPath)) return;
        if (!Equals(view.Picture.Tag as string, iconPath))
        {
            view.Picture.Source = CachedBitmap(iconPath);
            view.Picture.Tag = iconPath;
        }
    }

    /// <summary>
    /// Natychmiastowe przełączenie podglądu po wyborze paczki: dekodujemy tylko
    /// aktualnie widoczną ikonę z magazynu i skalujemy ją przez Stretch.Fill.
    /// Nie czekamy na kopiowanie pozostałych 28 plików do projektu.
    /// </summary>
    private void RefreshWeatherIconPreview(string packId)
    {
        var locale = EffectiveWatchfaceLanguage();
        foreach (var view in _frames.Values)
        {
            if (!IsWeatherIcon(view.Layer) || view.Picture is null) continue;
            var overrides = WeatherOverridesJson(view.Layer.WeatherDayOffset);
            var raw = RustNative.FormatWeatherField(
                "weather-icon", overrides, locale, packId, out _);
            if (raw is null) continue;
            using var document = JsonDocument.Parse(raw);
            if (!document.RootElement.TryGetProperty("iconPath", out var iconElement)) continue;
            var iconPath = iconElement.GetString();
            if (string.IsNullOrEmpty(iconPath) || !File.Exists(iconPath)) continue;
            if (Equals(view.Picture.Tag as string, iconPath)) continue;
            view.Picture.Source = CachedBitmap(iconPath);
            view.Picture.Tag = iconPath;
        }
    }

    /// <summary>Tylko ustawione pola są wysyłane do Rusta — reszta dostaje wartości Warszawy.</summary>
    private static string WeatherOverridesJson(byte dayOffset = 0, DateTime? now = null)
    {
        var moment = now ?? DefaultPreviewMoment();
        var values = new Dictionary<string, string>
        {
            ["dayOffset"] = dayOffset.ToString(),
            ["nowMinute"] = (moment.Hour * 60 + moment.Minute).ToString(CultureInfo.InvariantCulture),
        };
        foreach (var key in new[]
        {
            "city", "index", "high", "low", "current", "sunrise", "sunset",
        })
        {
            var value = AppSettingsStore.GetCustomPreviewValue(key);
            if (value is not null) values[key] = value;
        }
        return JsonSerializer.Serialize(values);
    }

    /// <summary>Wspólny dla podglądu i klatek eksportu formatter tekstu czasu/daty.</summary>
    private void FormatTextForTime(string layerKind, string? pattern, TextBlock text, DateTime now,
        string? firstDayOverride = null)
    {
        var locale = EffectiveWatchfaceLanguage();
        if (layerKind == "date")
        {
            var dateRaw = RustNative.FormatDate(
                string.IsNullOrEmpty(pattern) ? DefaultDatePattern : pattern,
                now.Day, now.Month - 1, now.Year, locale, out _);
            if (dateRaw is null) return;
            using var dateDocument = JsonDocument.Parse(dateRaw);
            SetTextIfChanged(text, dateDocument.RootElement.GetProperty("text").GetString() ?? text.Text);
            return;
        }
        if (IsDateFieldKind(layerKind))
        {
            // `month` jest 1-based (Rust odejmuje 1 we własnych funkcjach).
            var fieldRaw = RustNative.FormatDateField(layerKind, now.Day, now.Month, now.Year,
                firstDayOverride ?? AppSettingsStore.FirstDayOfWeek, out _);
            if (fieldRaw is null) return;
            using var fieldDocument = JsonDocument.Parse(fieldRaw);
            SetTextIfChanged(text, fieldDocument.RootElement.GetProperty("text").GetString() ?? text.Text);
            return;
        }
        if (layerKind == "battery")
        {
            // Na PC nie ma czujnika baterii — trzymamy neutralny stan początkowy,
            // który może nadpisać Custom Data. Na zegarku JS czyta BATTERY.current.
            SetTextIfChanged(text, "100%");
            return;
        }
        if (layerKind != "time") return;
        var raw = RustNative.FormatTime(pattern ?? "HH:MM:SS", now.Hour, now.Minute, now.Second,
            locale, out _);
        if (raw is null) return;
        using var document = JsonDocument.Parse(raw);
        SetTextIfChanged(text, document.RootElement.GetProperty("text").GetString() ?? text.Text);
    }

    /// <summary>Tyk Live Preview nie może bez powodu nadpisywać identycznych tekstów.</summary>
    private static void SetTextIfChanged(TextBlock text, string value)
    {
        if (!string.Equals(text.Text, value, StringComparison.Ordinal)) text.Text = value;
    }

    private static void RotateHandSet(AnalogHandsView hands, DateTime now)
    {
        SetHandAngle(hands.Hour, (now.Hour % 12) * 30.0 + now.Minute * 0.5);
        SetHandAngle(hands.Minute, now.Minute * 6.0 + now.Second * 0.1);
        if (hands.Second is not null) SetHandAngle(hands.Second, now.Second * 6.0);
    }

    private static void SetHandAngle(Image image, double angle)
    {
        if (image.RenderTransform is RotateTransform transform) transform.Angle = angle;
    }

    // --- Warstwy edytora ----------------------------------------------------

    private void LoadLayers()
    {
        if (_projectFile is null) return;
        var raw = RustNative.EditorLayers(_projectFile, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorLayers") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        _layers = JsonSerializer.Deserialize<List<LayerView>>(raw) ?? new();
        RestoreLayerSelection(_layers);
        RefreshLayersList();
        RebuildPreview();
        LoadWidgetSettings();
    }

    /// <summary>
    /// Przywraca zaznaczenie po przeładowaniu warstw. Jeśli użytkownik świadomie
    /// odznaczył warstwę, puste zaznaczenie jest zachowane.
    /// </summary>
    private void RestoreLayerSelection(List<LayerView> layers)
    {
        if (_selectedLayerId is not null && !layers.Any(l => l.Id == _selectedLayerId))
        {
            _selectedLayerId = layers.FirstOrDefault()?.Id;
            _selectionCleared = false;
        }
        else if (_selectedLayerId is null && !_selectionCleared)
        {
            _selectedLayerId = layers.FirstOrDefault()?.Id;
        }
    }

    private void RefreshLayersList()
    {
        var count = _layers.Count;
        // Warstwy z Rusta są od spodu — w panelu pokazujemy najwyższą na górze.
        var rows = _layers
            .AsEnumerable()
            .Reverse()
            .Select((layer, index) => new LayerRow
            {
                Layer = layer,
                CanMoveUp = index > 0,
                CanMoveDown = index < count - 1,
            })
            .ToList();
        LayersList.ItemsSource = rows;
        LayersList.SelectedItem = rows.FirstOrDefault(row => row.Id == _selectedLayerId);
    }

    private void LayersList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || e.AddedItems.FirstOrDefault() is not LayerRow row) return;
        _selectedLayerId = row.Id;
        _selectionCleared = false;
        LoadWidgetSettings();
        UpdateFrameVisuals();
    }

    /// <summary>Podwójne kliknięcie w puste pole odznacza aktualnie wybraną warstwę.</summary>
    private void RootGrid_DoubleTapped(object sender, DoubleTappedRoutedEventArgs e)
    {
        if (!_initialized || _projectFile is null) return;
        if (IsInspectorOrInteractiveTarget(e.OriginalSource)) return;
        ClearLayerSelection();
    }

    private void ClearLayerSelection()
    {
        if (_selectedLayerId is null) return;
        _selectedLayerId = null;
        _selectionCleared = true;
        _drag = null;
        HideGuides();
        LayersList.SelectedItem = null;
        LoadWidgetSettings();
        UpdateFrameVisuals();
    }

    /// <summary>
    /// Nie odznaczamy warstwy, gdy podwójne kliknięcie trafia w kontrolkę, ramkę widgetu
    /// (tam przełączamy tryb skalowania) albo w panel inspektora (podwójne kliknięcie
    /// służy tam do edycji wartości X/Y).
    /// </summary>
    private static bool IsInspectorOrInteractiveTarget(object? source)
    {
        var element = source as DependencyObject;
        while (element is not null)
        {
            if (element is FrameworkElement { Tag: LayerView }) return true;
            if (element is TextBox || element is ButtonBase || element is ComboBox
                || element is Slider || element is CheckBox || element is ToggleSwitch
                || element is ListViewItem || element is NumberBox || element is ContentDialog)
                return true;
            if (element is FrameworkElement named && named.Name is "WidgetInspector"
                or "AnalogSettingsPanel" or "DigitalAlignPanel")
                return true;
            element = VisualTreeHelper.GetParent(element);
        }
        return false;
    }

    /// <summary>Wczytuje ustawienia zaznaczonej warstwy do panelu inspektora.</summary>
    private void LoadWidgetSettings()
    {
        var layer = _layers.FirstOrDefault(l => l.Id == _selectedLayerId);
        if (layer is null)
        {
            // Podwójny kliknięcie w puste pole odznacza warstwę — inspektor znika.
            _settings = null;
            WidgetInspector.Visibility = Visibility.Collapsed;
            DigitalSettingsPanel.Visibility = Visibility.Collapsed;
            DigitalAlignPanel.Visibility = Visibility.Collapsed;
            StepInfoPanel.Visibility = Visibility.Collapsed;
            SensorInfoPanel.Visibility = Visibility.Collapsed;
            GoalInfoPanel.Visibility = Visibility.Collapsed;
            DistanceUnitPanel.Visibility = Visibility.Collapsed;
            ShowUnitPanel.Visibility = Visibility.Collapsed;
            AnalogSettingsPanel.Visibility = Visibility.Collapsed;
            ImagesSettingsPanel.Visibility = Visibility.Collapsed;
            StatusSettingsPanel.Visibility = Visibility.Collapsed;
            WeatherSettingsPanel.Visibility = Visibility.Collapsed;
            WeatherDayPanel.Visibility = Visibility.Collapsed;
            SleepStageModePanel.Visibility = Visibility.Collapsed;
            ProgressSettingsPanel.Visibility = Visibility.Collapsed;
            InspectorTitle.Text = "";
            return;
        }
        var isAnalog = layer.Kind == "analog-time";
        var isImages = layer.Kind == "images";
        var isStatus = IsStatusKind(layer.Kind);
        var isWeatherIcon = IsWeatherIcon(layer);
        var isDate = layer.Kind == "date";
        var isUserText = layer.Kind == "text";
        var isStep = layer.Kind == "steps";
        var isDistance = layer.Kind == "distance";
        var isHealth = IsHealthKind(layer.Kind);
        var isGoal = IsGoalKind(layer.Kind);
        var isWeather = IsWeatherKind(layer.Kind);
        var isSleep = IsSleepKind(layer.Kind);
        var isProgress = IsProgressKind(layer.Kind);
        WidgetInspector.Visibility = Visibility.Visible;
        DigitalSettingsPanel.Visibility = isAnalog || isImages || isStatus || isWeatherIcon || isProgress ? Visibility.Collapsed : Visibility.Visible;
        DigitalAlignPanel.Visibility = isAnalog || isImages || isStatus || isWeatherIcon || isProgress ? Visibility.Collapsed : Visibility.Visible;
        // Wzorzec czasu/daty ma tylko to, co katalog z Rusta oznaczył flagą
        // hasPattern (time i date). Bateria i pola pogody czytają czujnik —
        // pokazywanie im presetu HH:MM:SS było mylące i zapisywało timeFormat.
        FormatPanel.Visibility = HasPatternKind(layer.Kind) ? Visibility.Visible : Visibility.Collapsed;
        UserTextPanel.Visibility = isUserText ? Visibility.Visible : Visibility.Collapsed;
        StepInfoPanel.Visibility = isStep ? Visibility.Visible : Visibility.Collapsed;
        SensorInfoPanel.Visibility = IsSensorTextKind(layer.Kind) || isSleep
            ? Visibility.Visible
            : Visibility.Collapsed;
        if (SensorInfoPanel.Visibility == Visibility.Visible)
            SensorInfoHint.Text = layer.Kind == "battery"
                ? L("SensorInfoBattery")
                : isSleep ? L("SensorInfoSleep") : L("SensorInfoWeather");
        GoalInfoPanel.Visibility = isGoal ? Visibility.Visible : Visibility.Collapsed;
        if (isGoal)
        {
            // STAND i FAT_BURRING nie są gwarantowane na Active 2 — użytkownik
            // musi wiedzieć, że pole może zostać na wartości domyślnej.
            GoalInfoHint.Text = layer.Kind is "stand-target" or "fat-burning-target"
                ? L("GoalSensorWarning")
                : L("GoalSensorHint");
        }
        DistanceUnitPanel.Visibility = isDistance ? Visibility.Visible : Visibility.Collapsed;
        ShowUnitPanel.Visibility = HasUnitKind(layer.Kind) ? Visibility.Visible : Visibility.Collapsed;
        AnalogSettingsPanel.Visibility = isAnalog ? Visibility.Visible : Visibility.Collapsed;
        ImagesSettingsPanel.Visibility = isImages ? Visibility.Visible : Visibility.Collapsed;
        StatusSettingsPanel.Visibility = isStatus ? Visibility.Visible : Visibility.Collapsed;
        WeatherSettingsPanel.Visibility = isWeatherIcon ? Visibility.Visible : Visibility.Collapsed;
        // Miasto, temperatura bieżąca i „czas do" nie zależą od dnia prognozy.
        WeatherDayPanel.Visibility = isWeather && layer.Kind is not ("weather-city" or "weather-current" or "sun-current")
            ? Visibility.Visible
            : Visibility.Collapsed;
        SleepStageModePanel.Visibility = layer.Kind is "sleep-deep" or "sleep-light" or "sleep-rem" or "sleep-wake"
            ? Visibility.Visible
            : Visibility.Collapsed;
        ProgressSettingsPanel.Visibility = isProgress ? Visibility.Visible : Visibility.Collapsed;
        InspectorTitle.Text = isImages
            ? L("ImagesWidgetCardTitle")
            : isAnalog
                ? L("AnalogWidgetCardTitle")
                : LF("WidgetSettingsFor", layer.Name);

        // Geometria jest wspólna dla wszystkich widgetów — suwaki X/Y i przyciski
        // wyśrodkowania są widoczne zarówno dla czasu cyfrowego, jak i analogowego.
        _settings = new TimeSettings
        {
            Pattern = HasPatternKind(layer.Kind)
                ? layer.TimeFormat ?? (isDate ? DefaultDatePattern : "HH:MM:SS")
                : "",
            TextSize = layer.TextSize ?? 64,
            Font = layer.Font ?? "",
            Color = layer.Color ?? "0xFFFFFFFF",
            X = layer.X,
            Y = layer.Y,
            W = layer.W,
            H = layer.H,
            Align = layer.Align ?? "center",
            VAlign = layer.VAlign ?? "center",
            AutoScale = layer.AutoScale,
            TextStyle = layer.TextStyle ?? "ellipsis",
            UserText = layer.Text ?? "",
            OwnMeasurementSystem = layer.MeasurementSystem is not null,
            MeasurementSystem = layer.MeasurementSystem ?? AppSettingsStore.MeasurementSystem,
            ShowUnit = layer.ShowUnit,
            Alpha = layer.Alpha,
            ImageFit = string.IsNullOrEmpty(layer.ImageFit) ? "contain" : layer.ImageFit,
            WeatherIconSet = layer.WeatherIconSet ?? "Classic",
            WeatherDayOffset = layer.WeatherDayOffset,
            SleepStageMode = string.IsNullOrEmpty(layer.SleepStageMode)
                ? "duration"
                : layer.SleepStageMode,
            Progress = layer.Progress ?? new ProgressSettingsView(),
        };
        if (isProgress) LoadProgressSettings();

        if (isImages)
        {
            var selectedFile = string.IsNullOrEmpty(layer.Src)
                ? ""
                : Path.GetFileName(layer.Src);
            _loadingImages = true;
            WidgetImageComboBox.SelectedItem = (WidgetImageComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
                .FirstOrDefault(item => item.Value == selectedFile);
            ImageFitComboBox.SelectedItem = (ImageFitComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
                .FirstOrDefault(item => item.Value == _settings.ImageFit);
            _loadingImages = false;
        }

        if (isStatus)
        {
            _loadingStatusPacks = true;
            StatusPackComboBox.SelectedItem = _statusPacks
                .FirstOrDefault(pack => pack.Id == layer!.StatusIconSet);
            _loadingStatusPacks = false;
        }
        if (isWeatherIcon)
        {
            _loadingWeatherPacks = true;
            WeatherPackComboBox.SelectedItem = _weatherPacks
                .FirstOrDefault(pack => pack.Id == layer!.WeatherIconSet);
            WeatherScaleModeButtonText.Text = _settings.AutoScale
                ? L("ScaleModeOnLabel") : L("ScaleModeOffLabel");
            _loadingWeatherPacks = false;
        }

        if (isAnalog)
        {
            var selectedHands = layer!.HandSet ?? "";
            var handSet = _handSets.FirstOrDefault(set => set.Id == selectedHands);
            _loadingHands = true;
            HandsComboBox.SelectedItem = (HandsComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
                .FirstOrDefault(item => item.Value == selectedHands);
            // Checkboxy opcjonalnych wskazówek pojawiają się tylko wtedy, gdy wybrany
            // zestaw faktycznie zawiera te pliki.
            SecondHandCheckBox.Visibility = handSet is { HasSecond: true }
                ? Visibility.Visible : Visibility.Collapsed;
            CapHandCheckBox.Visibility = handSet is { HasCap: true }
                ? Visibility.Visible : Visibility.Collapsed;
            var hasRequired = handSet is not null;
            var hasSecond = handSet is { HasSecond: true };
            var hasCap = handSet is { HasCap: true };
            HourHandCheckBox.Visibility = hasRequired ? Visibility.Visible : Visibility.Collapsed;
            MinuteHandCheckBox.Visibility = hasRequired ? Visibility.Visible : Visibility.Collapsed;
            if (hasRequired)
            {
                _hourHandEnabled = !string.IsNullOrEmpty(layer.HourSrc);
                _minuteHandEnabled = !string.IsNullOrEmpty(layer.MinuteSrc);
            }
            if (hasSecond) _secondHandEnabled = !string.IsNullOrEmpty(layer.SecondSrc);
            if (hasCap) _capHandEnabled = !string.IsNullOrEmpty(layer.CapSrc);
            HourHandCheckBox.IsChecked = _hourHandEnabled;
            MinuteHandCheckBox.IsChecked = _minuteHandEnabled;
            SecondHandCheckBox.IsChecked = _secondHandEnabled;
            CapHandCheckBox.IsChecked = _capHandEnabled;
            _loadingHands = false;
        }

        _loadingSettings = true;
        // Presety i podpowiedź tokenów są inne dla daty i dla czasu — oba wzorce
        // trafiają do tego samego pola `timeFormat` w projekcie.
        FormatPreset.ItemsSource = isDate
            ? new[]
            {
                "MMM D", "DD.MM", "DD.MM.YYYY", "D MMM YYYY", "YYYY-MM-DD",
                "DDD, D MMM YYYY", "dddd, d mmmm yyyy",
            }
            : new[] { "HH:MM:SS", "HH:MM", "H:MM:SS", "hh:mm:ss A", "h:mm a" };
        FormatHintText.Text = isDate ? L("DateFormatHint") : L("FormatHint");
        FormatBox.Text = _settings.Pattern;
        WeatherDayCombo.SelectedItem = (WeatherDayCombo.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == _settings.WeatherDayOffset.ToString());
        SleepStageModeCombo.SelectedItem = (SleepStageModeCombo.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == _settings.SleepStageMode);
        UserTextBox.Text = _settings.UserText;
        if (HasUnitKind(layer.Kind))
        {
            ShowUnitCheckBox.IsChecked = _settings.ShowUnit;
        }
        OpacitySlider.Value = _settings.Alpha;
        OpacityValue.Text = _settings.Alpha.ToString();
        if (isDistance)
        {
            _loadingUnits = true;
            OwnMeasurementSystemCheckBox.IsChecked = _settings.OwnMeasurementSystem;
            DistanceUnitCombo.IsEnabled = _settings.OwnMeasurementSystem;
            DistanceUnitCombo.SelectedItem = (DistanceUnitCombo.ItemsSource as IEnumerable<ChoiceItem>)?
                .FirstOrDefault(item => item.Value == _settings.MeasurementSystem);
            _loadingUnits = false;
        }
        FitComboBox.SelectedItem = (FitComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == _settings.TextStyle);
        foreach (var item in FormatPreset.Items)
        {
            var value = item switch
            {
                ComboBoxItem combo => combo.Content as string,
                string text => text,
                _ => null,
            };
            if (value == _settings.Pattern)
            {
                FormatPreset.SelectedItem = item;
                break;
            }
        }
        FontComboBox.SelectedItem = (FontComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == _settings.Font);
        SizeSlider.Value = _settings.TextSize;
        // ValueChanged jest wyciszony przez `_loadingSettings`, więc etykietę
        // trzeba odświeżyć samemu — inaczej zostaje wartość poprzedniego widgetu.
        FontSizeValue.Text = _settings.TextSize.ToString();
        XSlider.Maximum = Math.Max(0, ScreenSize - _settings.W);
        XSlider.Value = _settings.X;
        YSlider.Maximum = Math.Max(0, ScreenSize - _settings.H);
        YSlider.Value = _settings.Y;
        UpdatePositionValues();
        _loadingSettings = false;

        UpdateColorUI();
        UpdateScaleModeUI();
        ApplyPreview();
    }

    /// <summary>
    /// Wczytuje bitmapę ze strumienia, a nie z URI — WinUI cache'uje obrazy pod tym samym
    /// URI, więc po nadpisaniu pliku obrazu podgląd pokazywałby starą wersję.
    /// </summary>
    private static BitmapImage LoadFreshBitmap(string path)
    {
        var image = new BitmapImage();
        using var stream = File.OpenRead(path);
        image.SetSource(stream.AsRandomAccessStream());
        return image;
    }

    /// <summary>
    /// Dekoduje bitmapę tylko raz na ścieżkę. WinUI skaluje ją przez Stretch.Fill
    /// przy zmianie Width/Height, więc przeciąganie ramki nie czyta pliku ani nie
    /// dekoduje PNG w każdym ruchu wskaźnika.
    /// </summary>
    private static readonly Dictionary<string, BitmapImage> _bitmapCache =
        new(StringComparer.OrdinalIgnoreCase);

    private static BitmapImage CachedBitmap(string path)
    {
        if (_bitmapCache.TryGetValue(path, out var cached)) return cached;
        var image = LoadFreshBitmap(path);
        _bitmapCache[path] = image;
        return image;
    }

    private static void InvalidateBitmapCache(string pathPrefix)
    {
        foreach (var key in _bitmapCache.Keys.Where(k => k.StartsWith(pathPrefix, StringComparison.OrdinalIgnoreCase)).ToList())
            _bitmapCache.Remove(key);
    }

    /// <summary>Tworzy obraz wskazówki o rozmiarze ramki z obrotem wokół jej środka.</summary>
    private Image? CreateHandImage(LayerView layer, string? src)
    {
        if (string.IsNullOrEmpty(src) || _projectFile is null) return null;
        var image = new Image
        {
            Width = layer.W,
            Height = layer.H,
            Stretch = Stretch.Fill,
            IsHitTestVisible = false,
            RenderTransformOrigin = new Point(0.5, 0.5),
            RenderTransform = new RotateTransform(),
        };
        var path = Path.Combine(Path.GetDirectoryName(_projectFile) ?? "", "assets", src);
        if (File.Exists(path)) image.Source = LoadFreshBitmap(path);
        return image;
    }

    private void RebuildPreview()
    {
        WatchCanvas.Children.Clear();
        OutlineCanvas.Children.Clear();
        var visual = PopulateWatchface(WatchCanvas, OutlineCanvas);
        _frames.Clear();
        foreach (var pair in visual.Frames) _frames[pair.Key] = pair.Value;
        _analogHands.Clear();
        foreach (var pair in visual.Hands) _analogHands[pair.Key] = pair.Value;
        UpdateFrameVisuals();
        UpdateClockText();
    }

    /// <summary>Jedyna ścieżka tworząca widok tarczy w podglądzie edytora.</summary>
    private WatchfaceVisual PopulateWatchface(Canvas watchCanvas, Canvas? outlineCanvas)
    {
        var visual = new WatchfaceVisual();
        foreach (var layer in _layers)
        {
            // Ukryta warstwa zostaje na liście i w inspektorze, ale nie jest rysowana
            // — dokładnie tak samo jak w wygenerowanym watchface/index.js.
            if (!layer.Visible) continue;
            if (layer.Kind is "image")
            {
                var image = new Microsoft.UI.Xaml.Controls.Image
                {
                    Width = layer.W,
                    Height = layer.H,
                    Stretch = Stretch.Fill,
                    HorizontalAlignment = HorizontalAlignment.Stretch,
                    VerticalAlignment = VerticalAlignment.Stretch,
                    IsHitTestVisible = false,
                    Opacity = layer.Alpha / 255.0,
                };
                if (!string.IsNullOrEmpty(layer.Src) && _projectFile is not null)
                {
                    var path = Path.Combine(Path.GetDirectoryName(_projectFile) ?? "", "assets", layer.Src);
                    if (File.Exists(path)) image.Source = LoadFreshBitmap(path);
                }
                Canvas.SetLeft(image, layer.X);
                Canvas.SetTop(image, layer.Y);
                watchCanvas.Children.Add(image);
                continue;
            }

            var view = new FrameView { Layer = layer };
            if (layer.Kind == "images" || IsStatusKind(layer.Kind) || IsWeatherIcon(layer))
            {
                // PNG w projekcie ma fizycznie rozmiar ramki — Podgląd rysuje go
                // bez skalowania, dokładnie jak hmUI.widget.IMG na tarczy.
                var picture = new Microsoft.UI.Xaml.Controls.Image
                {
                    Width = layer.W,
                    Height = layer.H,
                    Stretch = Stretch.Fill,
                    IsHitTestVisible = false,
                };
                if (!string.IsNullOrEmpty(layer.Src) && _projectFile is not null)
                {
                    var picturePath = Path.Combine(
                        Path.GetDirectoryName(_projectFile) ?? "", "assets", layer.Src);
                    if (File.Exists(picturePath)) picture.Source = LoadFreshBitmap(picturePath);
                }
                view.Picture = picture;
                view.Host.Children.Add(picture);
            }
            else if (layer.Kind == "analog-time")
            {
                var hands = new AnalogHandsView();
                if (CreateHandImage(layer, layer.HourSrc) is { } hour) { hands.Hour = hour; view.Host.Children.Add(hour); }
                if (CreateHandImage(layer, layer.MinuteSrc) is { } minute) { hands.Minute = minute; view.Host.Children.Add(minute); }
                if (CreateHandImage(layer, layer.SecondSrc) is { } second) { hands.Second = second; view.Host.Children.Add(second); }
                if (CreateHandImage(layer, layer.CapSrc) is { } cap) { hands.Cap = cap; view.Host.Children.Add(cap); }
                visual.Hands[layer.Id] = hands;
            }
            else if (layer.Kind is "arc-progress" or "bar-progress")
            {
                var settings = layer.Progress ?? new ProgressSettingsView();
                var progress = new ProgressVisual();
                var arc = layer.Kind == "arc-progress";
                // Podkład jest zawsze tworzony — przełącznik ShowTrack zmienia
                // tylko jego widoczność w UpdateProgressVisual.
                var track = CreateProgressShape(arc, layer, settings, true);
                progress.Track = track;
                view.Host.Children.Add(track);
                var fill = CreateProgressShape(arc, layer, settings, false);
                progress.Fill = fill;
                view.Host.Children.Add(fill);
                view.Progress = progress;
            }

            view.Text = new TextBlock
            {
                Width = layer.W,
                // Wysokość musi zostać automatyczna: `VerticalAlignment` ma
                // pozycjonować pole tekstu (o wysokości treści) wewnątrz ramki.
                // Sztywne Height = layer.H sprawiało, że po skalowaniu ramki tekst
                // zachowywał stary bounding box i uciekał poza Canvas/Outline.
                HorizontalAlignment = HorizontalAlignment.Left,
                FontSize = layer.TextSize ?? 64,
                Text = layer.Text ?? "",
                TextAlignment = AlignOf(layer.Align),
                VerticalAlignment = VerticalOf(layer.VAlign),
                Foreground = new SolidColorBrush(ParseArgb(layer.Color ?? "0xFFFFFFFF")),
                IsHitTestVisible = false,
                Tag = layer,
            };
            ApplyTextFit(view.Text, layer.TextStyle);
            ApplyFont(view.Text, layer.Font);
            visual.Texts[layer.Id] = view.Text;

            if (outlineCanvas is not null)
            {
                // Ramka i uchwyty trafiają na osobną warstwę poza klipem tarczy.
                // Dzięki temu obramowanie może wystawać poza obszar roboczy i zawsze
                // da się je złapać — także wtedy, gdy rogi wskazówek są poza okręgiem.
                var frame = new Border
                {
                    Width = layer.W,
                    Height = layer.H,
                    Background = new SolidColorBrush(Color.FromArgb(0, 0, 0, 0)),
                    Tag = layer,
                };
                AutomationProperties.SetName(frame, "WidgetFrame_" + layer.Id);
                Canvas.SetZIndex(frame, layer.Id == _selectedLayerId ? 100 : 1);
                frame.PointerPressed += Frame_PointerPressed;
                view.Frame = frame;

                foreach (var name in new[] { "nw", "ne", "sw", "se" })
                {
                    var handle = new Border
                    {
                        Name = name,
                        Width = HandleSize,
                        Height = HandleSize,
                        Tag = layer,
                        Background = new SolidColorBrush(Color.FromArgb(255, 255, 255, 255)),
                        BorderBrush = new SolidColorBrush(Color.FromArgb(255, 32, 32, 40)),
                        BorderThickness = new Thickness(1),
                    };
                    AutomationProperties.SetName(handle, "Resize_" + name);
                    Canvas.SetZIndex(handle, layer.Id == _selectedLayerId ? 101 : 2);
                    handle.PointerPressed += Frame_PointerPressed;
                    outlineCanvas.Children.Add(handle);
                    view.Handles.Add(handle);
                }

                if (layer.Kind == "arc-progress")
                {
                    // Dodatkowy punkt na łuku — przeciąganie zmienia `Progress.Thickness`.
                    var thick = new Border
                    {
                        Name = "thick",
                        Width = HandleSize + 4,
                        Height = HandleSize + 4,
                        CornerRadius = new CornerRadius(8),
                        Tag = layer,
                        Background = new SolidColorBrush(Color.FromArgb(255, 255, 255, 255)),
                        BorderBrush = new SolidColorBrush(Color.FromArgb(255, 32, 32, 40)),
                        BorderThickness = new Thickness(1),
                    };
                    AutomationProperties.SetName(thick, "ArcThickness_" + layer.Id);
                    Canvas.SetZIndex(thick, layer.Id == _selectedLayerId ? 102 : 3);
                    thick.PointerPressed += Frame_PointerPressed;
                    outlineCanvas.Children.Add(thick);
                    view.Handles.Add(thick);
                }
            }

            // Ikona pogody jest czystym obrazem — żaden TextBlock nie trafia do
            // drzewa wizualnego, więc nie może wylądować w nim tekst "--".
            if (layer.Kind != "images" && !IsWeatherIcon(layer) && view.Progress is null)
                view.Host.Children.Add(view.Text);
            view.Host.Opacity = layer.Alpha / 255.0;
            ApplyFrameGeometry(view, layer.X, layer.Y, layer.W, layer.H, layer.TextSize ?? 64, visual.Hands);
            watchCanvas.Children.Add(view.Host);
            if (outlineCanvas is not null) outlineCanvas.Children.Add(view.Frame);
            visual.Frames[layer.Id] = view;
        }
        return visual;
    }

    // --- Ramka: przeciąganie, skalowanie i zmiana rozmiaru -------------------

    private void SelectFrameLayer(LayerView layer)
    {
        if (_selectedLayerId == layer.Id) return;
        _selectedLayerId = layer.Id;
        _selectionCleared = false;
        LayersList.SelectedItem = (LayersList.ItemsSource as IEnumerable<LayerRow>)?
            .FirstOrDefault(row => row.Id == layer.Id);
        LoadWidgetSettings();
    }

    private void Frame_PointerPressed(object sender, PointerRoutedEventArgs e)
    {
        if (!_initialized || sender is not FrameworkElement element || element.Tag is not LayerView layer) return;
        e.Handled = true;
        if (layer.Kind is "image") return;
        SelectFrameLayer(layer);

        // Grafika (wskazówki, przyszłe widgety) w niebieskim trybie ma tylko
        // przesuwać się — dopiero czerwony tryb pozwala ją skalować.
        var handle = element.Name switch
        {
            "nw" or "ne" or "sw" or "se" => element.Name,
            "thick" => "thick",
            _ => "move",
        };
        if (IsImageLayer(layer) && !layer.AutoScale) handle = "move";

        // Podwójne kliknięcie ramki przełącza tryb skalowania (tekstu albo wskazówek).
        // Punkt grubości łuku ma własny gest — nie przełączamy tam trybu.
        var now = Environment.TickCount64;
        if (handle != "thick" && _lastFramePressId == layer.Id && now - _lastFramePressAt < 450)
        {
            _lastFramePressAt = 0;
            ToggleScaleMode(layer);
            return;
        }
        _lastFramePressId = layer.Id;
        _lastFramePressAt = now;
        if (_settings is null) return;

        _drag = new FrameDragState
        {
            LayerId = layer.Id,
            Handle = handle,
            StartPointer = e.GetCurrentPoint(OutlineCanvas).Position,
            StartX = layer.X,
            StartY = layer.Y,
            StartW = layer.W,
            StartH = layer.H,
            StartTextSize = layer.TextSize ?? 64,
            StartThickness = layer.Progress?.Thickness ?? 16,
            AutoScale = layer.AutoScale,
        };
        OutlineCanvas.CapturePointer(e.Pointer);
    }

    private void OutlineCanvas_PointerMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_drag is null || !_frames.TryGetValue(_drag.LayerId, out var view)) return;
        var point = e.GetCurrentPoint(OutlineCanvas).Position;
        var dx = (int)Math.Round(point.X - _drag.StartPointer.X);
        var dy = (int)Math.Round(point.Y - _drag.StartPointer.Y);

        if (_drag.Handle == "thick")
        {
            // Punkt na łuku: środek pierścienia pozostaje na miejscu, a przeciągnięcie
            // na zewnątrz pogrubia łuk (do środka odchudza). Wszystkie zmiany idą
            // przez ProgressSettings i rw_set_widget
            // dopiero przy puszczeniu pointera (EndFrameDrag).
            var cx = view.Layer.X + view.Layer.W / 2.0;
            var cy = view.Layer.Y + view.Layer.H / 2.0;
            var startDistance = Math.Sqrt(
                (_drag.StartPointer.X - cx) * (_drag.StartPointer.X - cx)
                + (_drag.StartPointer.Y - cy) * (_drag.StartPointer.Y - cy));
            var distance = Math.Sqrt(
                (point.X - cx) * (point.X - cx) + (point.Y - cy) * (point.Y - cy));
            var thickness = Math.Clamp(
                _drag.StartThickness + 2.0 * (distance - startDistance), 1, 233);
            var roundedThickness = (uint)Math.Round(thickness);
            var layerProgress = view.Layer.Progress ?? new ProgressSettingsView();
            layerProgress.Thickness = roundedThickness;
            view.Layer.Progress = layerProgress;
            if (_settings is not null)
            {
                _settings.Progress.Thickness = roundedThickness;
                if (!_loadingProgress) ProgressThicknessBox.Value = roundedThickness;
            }
            // Ramka pozostaje bez zmian — grubszy łuk jest rysowany z marginesem
            // poza jej obrys, więc nie jest przycinany.
            ApplyFrameGeometry(view, view.Layer.X, view.Layer.Y, view.Layer.W, view.Layer.H,
                view.Layer.TextSize ?? 64);
            e.Handled = true;
            return;
        }

        // Kandydaci do wyrównania: środek i krawędzie płótna oraz krawędzie/środki
        // pozostałych widgetów. Matematykę przyciągania robi rusty-core.
        var alignX = new List<int> { 0, ScreenSize / 2, ScreenSize };
        var alignY = new List<int> { 0, ScreenSize / 2, ScreenSize };
        foreach (var other in _layers)
        {
            if (other.Id == _drag.LayerId || !other.Visible) continue;
            alignX.Add(other.X);
            alignX.Add(other.X + other.W / 2);
            alignX.Add(other.X + other.W);
            alignY.Add(other.Y);
            alignY.Add(other.Y + other.H / 2);
            alignY.Add(other.Y + other.H);
        }

        var input = JsonSerializer.Serialize(new
        {
            x = _drag.StartX,
            y = _drag.StartY,
            w = _drag.StartW,
            h = _drag.StartH,
            textSize = _drag.StartTextSize,
            handle = _drag.Handle,
            dx,
            dy,
            autoScale = _drag.AutoScale,
            square = view.Layer.Kind is "analog-time" or "arc-progress",
            snap = PreviewGridToggle.IsChecked == true,
            alignX,
            alignY,
        });
        var raw = RustNative.FrameGeometry(input, out _);
        if (raw is null) return;
        using var document = JsonDocument.Parse(raw);
        var data = document.RootElement;
        var x = data.GetProperty("x").GetInt32();
        var y = data.GetProperty("y").GetInt32();
        var w = data.GetProperty("w").GetInt32();
        var h = data.GetProperty("h").GetInt32();
        var textSize = data.GetProperty("textSize").GetUInt32();
        UpdateGuides(data);

        ApplyFrameGeometry(view, x, y, w, h, textSize);
        view.Layer.X = x;
        view.Layer.Y = y;
        view.Layer.W = w;
        view.Layer.H = h;
        view.Layer.TextSize = textSize;
        if (_settings is not null)
        {
            _settings.X = x;
            _settings.Y = y;
            _settings.W = w;
            _settings.H = h;
            _settings.TextSize = textSize;
        }
        e.Handled = true;
    }

    /// <summary>Przerywane czerwone prowadnice wyrównania zwracane przez rw_frame_geometry.</summary>
    private void UpdateGuides(JsonElement data)
    {
        var gx = data.GetProperty("guideX");
        if (gx.ValueKind == JsonValueKind.Number)
        {
            var value = gx.GetInt32();
            GuideVertical.X1 = GuideVertical.X2 = value;
            GuideVertical.Y1 = 0;
            GuideVertical.Y2 = ScreenSize;
            GuideVertical.Visibility = Visibility.Visible;
        }
        else GuideVertical.Visibility = Visibility.Collapsed;

        var gy = data.GetProperty("guideY");
        if (gy.ValueKind == JsonValueKind.Number)
        {
            var value = gy.GetInt32();
            GuideHorizontal.Y1 = GuideHorizontal.Y2 = value;
            GuideHorizontal.X1 = 0;
            GuideHorizontal.X2 = ScreenSize;
            GuideHorizontal.Visibility = Visibility.Visible;
        }
        else GuideHorizontal.Visibility = Visibility.Collapsed;
    }

    private void HideGuides()
    {
        GuideVertical.Visibility = Visibility.Collapsed;
        GuideHorizontal.Visibility = Visibility.Collapsed;
    }

    private void OutlineCanvas_PointerReleased(object sender, PointerRoutedEventArgs e)
        => EndFrameDrag(e, persist: true);

    private void OutlineCanvas_PointerCanceled(object sender, PointerRoutedEventArgs e)
        => EndFrameDrag(e, persist: true);

    private void EndFrameDrag(PointerRoutedEventArgs e, bool persist)
    {
        if (_drag is null) return;
        _drag = null;
        HideGuides();
        try { OutlineCanvas.ReleasePointerCapture(e.Pointer); } catch { /* pointer już zwolniony */ }
        if (persist)
        {
            SyncPositionSliders();
            PersistSettings();
        }
        e.Handled = true;
    }

    private void ToggleScaleMode(LayerView layer)
    {
        SelectFrameLayer(layer);
        if (_settings is null) return;
        _settings.AutoScale = !_settings.AutoScale;
        if (layer.Kind == "images" && _settings.AutoScale)
        {
            // Czerwona ramka obrazu zawsze rozciąga grafikę do nowego rozmiaru.
            _settings.ImageFit = "stretch";
            _loadingImages = true;
            ImageFitComboBox.SelectedItem = (ImageFitComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
                .FirstOrDefault(item => item.Value == "stretch");
            _loadingImages = false;
        }
        PersistSettings();
        UpdateScaleModeUI();
        if (IsWeatherIcon(layer))
        {
            WeatherScaleModeButtonText.Text = _settings.AutoScale
                ? L("ScaleModeOnLabel") : L("ScaleModeOffLabel");
        }
        // Odświeża kolor ramki i widoczność uchwytów (obraz w niebieskim trybie
        // ma tylko przesuwanie, w czerwonym — uchwyty rozciągania).
        UpdateFrameVisuals();
        var analog = layer.Kind == "analog-time";
        var key = analog
            ? (_settings.AutoScale ? "ScaleModeOnHands" : "ScaleModeOffHands")
            : (_settings.AutoScale ? "ScaleModeOn" : "ScaleModeOff");
        ShowStatus(L(key));
    }

    private void MoveLayer(string id, int delta)
    {
        if (_projectFile is null) return;
        var raw = RustNative.MoveLayer(_projectFile, id, delta, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorMoveLayer") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        ShowStatus(L("LayerMoved"));
    }

    private void LayerUp_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string id }) MoveLayer(id, 1);
    }

    private void LayerDown_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string id }) MoveLayer(id, -1);
    }

    /// <summary>Oczko na liście warstw: ukrywa widget w podglądzie i w eksportowanej tarczy.</summary>
    private void ToggleLayerVisibility_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null || sender is not Button { Tag: string id }) return;
        var layer = _layers.FirstOrDefault(l => l.Id == id);
        if (layer is null) return;

        var raw = RustNative.SetWidgetVisibility(_projectFile, id, !layer.Visible, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorLayerVisibility") : LocalizationService.Error(error),
                InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        LoadWidgetSettings();
        ShowStatus(L(layer.Visible ? "LayerHidden" : "LayerShown"));
    }

    private void RemoveWidget_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null || sender is not Button { Tag: string id }) return;
        var raw = RustNative.RemoveWidget(_projectFile, id, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorRemoveWidget") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        LoadWidgetSettings();
        ShowStatus(L("WidgetRemoved"));
    }

    /// <summary>
    /// Język podglądu tarczy wynika wyłącznie z ustawienia „Język podglądu tarczy"
    /// (domyślnie angielski) — nie od języka Windows ani od języka interfejsu.
    /// Sama tarcza wykrywa język zegarka (pl-PL = 9) w runtime; to ustawienie mówi
    /// edytorowi, który wariant pokazać w podglądzie i w plikach preview.png/gif.
    /// </summary>
    private static string EffectiveWatchfaceLanguage()
        => AppSettingsStore.WatchfacePreviewLanguage == "pl" ? "pl" : "en";

    private void LoadWatchfaceLanguageOptions()
    {
        _loadingWatchfaceLanguage = true;
        var items = new[]
        {
            new ChoiceItem { Value = "pl", Name = L("WatchfaceLanguagePolish") },
            new ChoiceItem { Value = "en", Name = L("WatchfaceLanguageEnglish") },
        };
        var selected = AppSettingsStore.WatchfacePreviewLanguage;
        WatchfaceLanguageCombo.ItemsSource = items;
        WatchfaceLanguageCombo.SelectedItem = items.FirstOrDefault(item => item.Value == selected);
        _loadingWatchfaceLanguage = false;
    }

    private void WatchfaceLanguageCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingWatchfaceLanguage) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        AppSettingsStore.SetWatchfacePreviewLanguage(item.Value);
        UpdateClockText();
    }

    private static ChoiceItem[] UnitChoices() => new[]
    {
        new ChoiceItem { Value = "watch", Name = L("MeasurementSystemWatch") },
        new ChoiceItem { Value = "metric", Name = L("MeasurementSystemMetric") },
        new ChoiceItem { Value = "imperial", Name = L("MeasurementSystemImperial") },
    };

    private void LoadMeasurementSystemOptions()
    {
        _loadingUnits = true;
        MeasurementSystemCombo.ItemsSource = UnitChoices();
        MeasurementSystemCombo.SelectedItem = (MeasurementSystemCombo.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == AppSettingsStore.MeasurementSystem);
        _loadingUnits = false;
    }

    private void LoadDistanceUnitOptions()
    {
        _loadingUnits = true;
        DistanceUnitCombo.ItemsSource = UnitChoices();
        _loadingUnits = false;
    }

    private static ChoiceItem[] WeatherDayChoices() => new[]
    {
        new ChoiceItem { Value = "0", Name = L("WeatherDayToday") },
        new ChoiceItem { Value = "1", Name = L("WeatherDayTomorrow") },
        new ChoiceItem { Value = "2", Name = L("WeatherDayDayAfter") },
        new ChoiceItem { Value = "3", Name = L("WeatherDayPlus3") },
        new ChoiceItem { Value = "4", Name = L("WeatherDayPlus4") },
    };

    private void LoadWeatherDayOptions()
    {
        _loadingWidgetOptions = true;
        WeatherDayCombo.ItemsSource = WeatherDayChoices();
        _loadingWidgetOptions = false;
    }

    private void LoadSleepStageOptions()
    {
        _loadingWidgetOptions = true;
        SleepStageModeCombo.ItemsSource = new[]
        {
            new ChoiceItem { Value = "duration", Name = L("SleepStageModeDuration") },
            new ChoiceItem { Value = "percent", Name = L("SleepStageModePercent") },
        };
        _loadingWidgetOptions = false;
    }

    /// <summary>Stosuje system z ustawień aplikacji w otwartym projekcie (wpływa na JS).</summary>
    private void ApplyAppMeasurementSystemToProject()
    {
        if (_projectFile is null) return;
        var raw = RustNative.SetMeasurementSystem(
            _projectFile, AppSettingsStore.MeasurementSystem, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorSaveChanges") : LocalizationService.Error(error),
                InfoBarSeverity.Error);
            return;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
    }

    /// <summary>
    /// Starsze projekty i nowe widgety mają `font: null` — wtedy edytor używał
    /// Segoe UI, a tarcza czcionki systemowej ZeppOS. Uzupełniamy dołączony TTF,
    /// żeby podgląd i eksport miały identyczne metryki tekstu.
    /// </summary>
    private void ApplyDefaultFontToMissing()
    {
        if (_projectFile is null) return;
        var raw = RustNative.ApplyDefaultFontToMissing(_projectFile, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorSaveChanges") : LocalizationService.Error(error),
                InfoBarSeverity.Error);
            return;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
    }

    private void MeasurementSystemCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingUnits || !_initialized) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        AppSettingsStore.SetMeasurementSystem(item.Value);
        ApplyAppMeasurementSystemToProject();
        LoadWidgetSettings();
    }

    private static ChoiceItem[] FirstDayChoices() => new[]
    {
        new ChoiceItem { Value = "monday", Name = L("FirstDayMonday") },
        new ChoiceItem { Value = "sunday", Name = L("FirstDaySunday") },
    };

    private void LoadFirstDayOfWeekOptions()
    {
        _loadingFirstDay = true;
        FirstDayOfWeekCombo.ItemsSource = FirstDayChoices();
        FirstDayOfWeekCombo.SelectedItem = (FirstDayOfWeekCombo.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == AppSettingsStore.FirstDayOfWeek);
        _loadingFirstDay = false;
    }

    /// <summary>Stosuje pierwszy dzień tygodnia z ustawień w otwartym projekcie (wpływa na JS).</summary>
    private void ApplyAppFirstDayOfWeekToProject()
    {
        if (_projectFile is null) return;
        var raw = RustNative.SetFirstDayOfWeek(
            _projectFile, AppSettingsStore.FirstDayOfWeek, out var error);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorSaveChanges") : LocalizationService.Error(error),
                InfoBarSeverity.Error);
            return;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
    }

    private void FirstDayOfWeekCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingFirstDay || !_initialized) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        AppSettingsStore.SetFirstDayOfWeek(item.Value);
        ApplyAppFirstDayOfWeekToProject();
        UpdateClockText();
        LoadWidgetSettings();
    }

    private void ShowUnitCheckBox_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || _loadingSettings || _loadingUnits) return;
        _settings.ShowUnit = ShowUnitCheckBox.IsChecked == true;
        PersistSettings();
    }

    private void WeatherDayCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_settings is null || _loadingSettings || _loadingWidgetOptions) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        if (!byte.TryParse(item.Value, out var day)) return;
        _settings.WeatherDayOffset = day;
        PersistSettings();
        UpdateClockText();
    }

    private void SleepStageModeCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_settings is null || _loadingSettings || _loadingWidgetOptions) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        _settings.SleepStageMode = item.Value;
        PersistSettings();
        UpdateClockText();
    }

    private void OwnMeasurementSystemCheckBox_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || _loadingUnits || _loadingSettings) return;
        _settings.OwnMeasurementSystem = OwnMeasurementSystemCheckBox.IsChecked == true;
        DistanceUnitCombo.IsEnabled = _settings.OwnMeasurementSystem;
        PersistSettings();
    }

    private void DistanceUnitCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_settings is null || _loadingUnits || _loadingSettings) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        _settings.MeasurementSystem = item.Value;
        PersistSettings();
    }

    private void LoadTextFitOptions()
    {
        _loadingFitOptions = true;
        LoadChoices(FitComboBox, new[]
        {
            new ChoiceItem { Value = "ellipsis", Name = L("FitCut") },
            new ChoiceItem { Value = "none", Name = L("FitSlide") },
            new ChoiceItem { Value = "wrap", Name = L("FitWrap") },
        });
        _loadingFitOptions = false;
    }

    // --- Assets: wczytywanie i odświeżanie bez blokowania wejścia na zakładkę ---

    private void LoadFonts()
    {
        _assetsRefreshVersion++;
        _fonts = RustNative.FontsList(out var error);
        ApplyFonts(_fonts, error);
    }

    private void ApplyFonts(List<FontAssetDto> fonts, string? error)
    {
        if (error is not null) AssetsStatusText.Text = LocalizationService.Error(error);

        var signature = string.Join("|", fonts.Select(font => $"{font.File}:{font.Family}:{font.IsDefault}"));
        if (signature != _fontSignature)
        {
            _fontSignature = signature;
            PreviewFonts.RefreshCatalog(fonts);
        }

        var fontItems = new List<ChoiceItem>
        {
            new() { Value = "", Name = L("WidgetFontSystem") },
        };
        fontItems.AddRange(fonts.Select(font => new ChoiceItem
        {
            Value = font.File,
            Name = font.IsDefault ? $"{font.Family} ({L("AssetsDefaultBadge")})" : font.Family,
        }));
        _loadingFontChoices = true;
        LoadChoices(FontComboBox, fontItems.ToArray());
        _loadingFontChoices = false;

        var fontRows = fonts.Select(font => new FontItem
        {
            File = font.File,
            Family = font.Family,
            IsDefault = font.IsDefault,
        }).ToList();
        FontsList.ItemsSource = fontRows;
        FontsList.SelectedItem = fontRows.FirstOrDefault(item => item.File == _selectedFontFile);
        RemoveFontButton.IsEnabled = FontsList.SelectedItem is FontItem { IsDefault: false };
    }

    private void LoadImages()
    {
        _assetsRefreshVersion++;
        _images = RustNative.ImagesList(out var error);
        ApplyImages(_images, error);
    }

    private void ApplyImages(List<ImageAssetDto> images, string? error)
    {
        if (error is not null) ImagesStatusText.Text = LocalizationService.Error(error);

        var choices = images.Select(image => new ChoiceItem
        {
            Value = image.File,
            Name = Path.GetFileNameWithoutExtension(image.File),
        }).ToArray();
        _loadingImages = true;
        LoadChoices(WidgetImageComboBox, choices);
        LoadChoices(ImageFitComboBox, ImageFitChoices());
        var selectedLayer = _layers.FirstOrDefault(layer => layer.Id == _selectedLayerId);
        var current = string.IsNullOrEmpty(selectedLayer?.Src) ? "" : Path.GetFileName(selectedLayer.Src);
        WidgetImageComboBox.SelectedItem = choices.FirstOrDefault(item => item.Value == current);
        ImageFitComboBox.SelectedItem = (ImageFitComboBox.ItemsSource as IEnumerable<ChoiceItem>)?
            .FirstOrDefault(item => item.Value == (selectedLayer?.ImageFit ?? "contain"));
        _loadingImages = false;

        var rows = images.Select(image => new ImageItem
        {
            File = image.File,
            Width = image.Width,
            Height = image.Height,
            Bytes = image.Bytes,
            Directory = AppPaths.ImagesDirectory,
        }).ToList();
        foreach (var row in rows) row.RefreshPreview(false);
        ImagesList.ItemsSource = rows;
        ImagesList.SelectedItem = rows.FirstOrDefault(row => row.File == _selectedImageFile);
        RemoveImageButton.IsEnabled = ImagesList.SelectedItem is not null;
    }

    private static ChoiceItem[] ImageFitChoices() => new[]
    {
        new ChoiceItem { Value = "contain", Name = L("ImageFitContain") },
        new ChoiceItem { Value = "cover", Name = L("ImageFitCover") },
        new ChoiceItem { Value = "stretch", Name = L("ImageFitStretch") },
    };

    private void LoadHandSets()
    {
        _assetsRefreshVersion++;
        _handSets = RustNative.HandsList(out var error);
        ApplyHandSets(_handSets, error);
    }

    private void ApplyHandSets(List<HandSetDto> sets, string? error)
    {
        if (error is not null) HandsStatusText.Text = LocalizationService.Error(error);

        var choices = sets.Select(set => new ChoiceItem { Value = set.Id, Name = set.Name }).ToList();
        _loadingHands = true;
        HandsComboBox.ItemsSource = choices;
        var current = _layers.FirstOrDefault(l => l.Kind == "analog-time")?.HandSet ?? "";
        HandsComboBox.SelectedItem = choices.FirstOrDefault(item => item.Value == current);
        _loadingHands = false;

        var byId = _handItems.ToDictionary(item => item.Id, StringComparer.OrdinalIgnoreCase);
        foreach (var set in sets)
        {
            if (!byId.TryGetValue(set.Id, out var item))
            {
                item = new HandSetItem { Id = set.Id, IsLoading = true };
                byId[set.Id] = item;
                _handItems.Add(item);
            }
            var fingerprint = set.Fingerprint ?? "";
            var changed = item.Fingerprint != fingerprint || item.NeedsRefresh;
            item.Size = set.Size;
            item.HasSecond = set.HasSecond;
            item.HasCap = set.HasCap;
            item.Fingerprint = fingerprint;
            item.RefreshPreview(changed);
        }
        foreach (var stale in _handItems.Where(item => sets.All(set => set.Id != item.Id)).ToList())
            _handItems.Remove(stale);

        if (!_handsBound)
        {
            HandsList.ItemsSource = _handItems;
            _handsBound = true;
        }
        HandsList.SelectedItem = _handItems.FirstOrDefault(row => row.Id == _selectedHandSetId);
        RemoveHandsButton.IsEnabled = HandsList.SelectedItem is HandSetItem { Id: not "Classic" };
    }

    private void LoadWeatherPacks()
    {
        _weatherPacks = RustNative.WeatherPacksList(out var error);
        if (error is not null) WeatherStatusText.Text = LocalizationService.Error(error);
        _loadingWeatherPacks = true;
        WeatherPackComboBox.ItemsSource = _weatherPacks;
        var current = _layers.FirstOrDefault(IsWeatherIcon)?.WeatherIconSet ?? "Classic";
        WeatherPackComboBox.SelectedItem = _weatherPacks.FirstOrDefault(pack => pack.Id == current);
        _loadingWeatherPacks = false;
        ApplyWeatherPacks(_weatherPacks);
    }

    private void ApplyWeatherPacks(List<WeatherPackDto> packs)
    {
        foreach (var pack in packs)
        {
            var item = _weatherPackItems.FirstOrDefault(p => p.Id == pack.Id);
            if (item is null)
            {
                item = new WeatherPackItem { Id = pack.Id };
                _weatherPackItems.Add(item);
            }
            item.Size = pack.Size;
            item.Fingerprint = pack.Fingerprint ?? "";
            if (item.NeedsRefresh) item.RefreshPreview();
        }
        var ids = packs.Select(p => p.Id).ToHashSet(StringComparer.OrdinalIgnoreCase);
        foreach (var stale in _weatherPackItems.Where(p => !ids.Contains(p.Id)).ToList())
            _weatherPackItems.Remove(stale);
        WeatherPacksGrid.SelectedItem = _weatherPackItems.FirstOrDefault(p => p.Id == (_selectedWeatherPackId ?? "Classic"));
        RemoveWeatherButton.IsEnabled = WeatherPacksGrid.SelectedItem is WeatherPackItem { IsDefault: false };
    }

    private void WeatherPacksGrid_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if ((WeatherPacksGrid.SelectedItem as WeatherPackItem) is not { } item) return;
        _selectedWeatherPackId = item.Id;
        RemoveWeatherButton.IsEnabled = !item.IsDefault;
    }

    private async void AddWeatherButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker { SuggestedStartLocation = PickerLocationId.DocumentsLibrary };
        picker.FileTypeFilter.Add("*");
        var folder = await picker.PickSingleFolderAsync();
        if (folder is null) return;
        WeatherPackDto? pack = null;
        string? error = null;
        WeatherStatusText.Text = L("StatusPreparingAssets");
        await Task.Run(() => pack = RustNative.WeatherPacksAdd(folder.Path, out error))
            .ConfigureAwait(true);
        if (pack is null)
        {
            WeatherStatusText.Text = error is null ? L("StatusApplyFailed") : LocalizationService.Error(error);
            return;
        }
        _selectedWeatherPackId = pack.Id;
        LoadWeatherPacks();
        WeatherStatusText.Text = LF("StatusApplied", pack.Id);
    }

    private void RemoveWeatherButton_Click(object sender, RoutedEventArgs e)
    {
        if ((WeatherPacksGrid.SelectedItem as WeatherPackItem) is not { } item || item.IsDefault) return;
        var raw = RustNative.WeatherPacksRemove(item.Id, out var error);
        if (raw is null)
        {
            WeatherStatusText.Text = error is null ? L("StatusApplyFailed") : LocalizationService.Error(error);
            return;
        }
        if (_selectedWeatherPackId == item.Id) _selectedWeatherPackId = null;
        LoadWeatherPacks();
        WeatherStatusText.Text = L("AssetsStatusRemoved");
    }

    private async void OpenWeatherFolderButton_Click(object sender, RoutedEventArgs e)
    {
        await Launcher.LaunchFolderAsync(
            await StorageFolder.GetFolderFromPathAsync(AppPaths.WeatherDirectory));
    }

    private void LoadStatusPacks()
    {
        _assetsRefreshVersion++;
        _statusPacks = RustNative.StatusPacksList(out var error);
        ApplyStatusPacks(_statusPacks, error);
    }

    private void ApplyStatusPacks(List<StatusPackDto> packs, string? error)
    {
        if (error is not null) StatusStatusText.Text = LocalizationService.Error(error);

        _loadingStatusPacks = true;
        StatusPackComboBox.ItemsSource = packs;
        var current = _layers.FirstOrDefault(l => IsStatusKind(l.Kind))?.StatusIconSet
                      ?? _selectedStatusPackId
                      ?? "Classic";
        StatusPackComboBox.SelectedItem = packs.FirstOrDefault(pack => pack.Id == current);
        _loadingStatusPacks = false;

        var byId = _statusItems.ToDictionary(item => item.Id, StringComparer.OrdinalIgnoreCase);
        foreach (var pack in packs)
        {
            if (!byId.TryGetValue(pack.Id, out var item))
            {
                item = new StatusPackItem { Id = pack.Id, IsLoading = true };
                byId[pack.Id] = item;
                _statusItems.Add(item);
            }
            var fingerprint = pack.Fingerprint ?? "";
            var changed = item.Fingerprint != fingerprint || item.NeedsRefresh;
            item.Size = pack.Size;
            item.Fingerprint = fingerprint;
            item.RefreshPreview(changed);
        }
        foreach (var stale in _statusItems.Where(item => packs.All(pack => pack.Id != item.Id)).ToList())
            _statusItems.Remove(stale);

        if (!_statusBound)
        {
            StatusList.ItemsSource = _statusItems;
            _statusBound = true;
        }
        StatusList.SelectedItem = _statusItems.FirstOrDefault(row => row.Id == _selectedStatusPackId);
        RemoveStatusButton.IsEnabled =
            StatusList.SelectedItem is StatusPackItem { Id: not "Classic" };
    }

    /// <summary>Nowe/edytowane paczki dostają pierścień zanim ruszyGenerated w tle.</summary>
    private void MarkChangedPacksLoading()
    {
        foreach (var item in _handItems.Where(item => item.NeedsRefresh)) item.IsLoading = true;
        foreach (var item in _statusItems.Where(item => item.NeedsRefresh)) item.IsLoading = true;
    }

    /// <summary>
    /// Wejście na Assets nie czeka na generowanie miniatur — widok pojawia się z
    /// cache'owanymi obrazkami, a odświeżanie magazynów idzie w tle.
    /// </summary>
    private async Task RefreshAssetsAsync()
    {
        var version = ++_assetsRefreshVersion;
        var dispatcher = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
        MarkChangedPacksLoading();

        List<HandSetDto>? hands = null;
        List<StatusPackDto>? packs = null;
        List<FontAssetDto>? fonts = null;
        List<ImageAssetDto>? images = null;
        string? handError = null, packError = null, fontError = null, imageError = null;

        await Task.WhenAll(
            Task.Run(() => { hands = RustNative.HandsList(out handError); }),
            Task.Run(() => { packs = RustNative.StatusPacksList(out packError); }),
            Task.Run(() => { fonts = RustNative.FontsList(out fontError); }),
            Task.Run(() => { images = RustNative.ImagesList(out imageError); })
        ).ConfigureAwait(false);

        dispatcher?.TryEnqueue(() =>
        {
            if (version != _assetsRefreshVersion) return;
            _handSets = hands ?? _handSets;
            _statusPacks = packs ?? _statusPacks;
            _fonts = fonts ?? _fonts;
            _images = images ?? _images;
            if (fonts is not null) ApplyFonts(fonts, fontError);
            if (images is not null) ApplyImages(images, imageError);
            if (hands is not null) ApplyHandSets(hands, handError);
            if (packs is not null) ApplyStatusPacks(packs, packError);
        });
    }

    /// <summary>Assets-siatka: zawsze maksymalnie trzy kafelki w jednym rzędzie.</summary>
    private void AssetGrid_SizeChanged(object sender, SizeChangedEventArgs e)
    {
        if (sender is GridView grid && e.NewSize.Width > 0
            && grid.ItemsPanelRoot is ItemsWrapGrid panel)
        {
            panel.ItemWidth = Math.Max(160, (e.NewSize.Width - 16) / 3);
        }
    }

    private static void LoadChoices(ComboBox combo, ChoiceItem[] items)
    {
        var selected = (combo.SelectedItem as ChoiceItem)?.Value;
        combo.ItemsSource = items;
        if (selected is not null)
            combo.SelectedItem = items.FirstOrDefault(item => item.Value == selected);
    }

    private void LoadWidgetCatalog()
    {
        var items = RustNative.SupportedWidgetItems(out var error);
        if (error is not null)
            ShowStatus(LocalizationService.Error(error), InfoBarSeverity.Error);

        _widgetCatalog = items.Select(item => new WidgetCatalogItem
        {
            Kind = item.Kind,
            Category = item.Category,
            HasPattern = item.HasPattern,
            Title = WidgetTitle(item.Kind),
            Description = WidgetDescription(item.Kind),
            Glyph = item.Kind switch
            {
                "analog-time" => "\uE823",
                "arc-progress" => "\uE823",
                "bar-progress" => "\uE8B2",
                "world-clock" => "\uE823",
                "date" => "\uE787",
                "text" => "\uE8A5",
                "images" => "\uE8B7",
                _ => "\uE8B2",
            },
        }).ToList();

        // Kategorie z Rusta są źródłem prawdy dla obu miejsc: flyoutu „Dodaj widget"
        // oraz zakładki Widgets. Kolejność wynika z kolejności wpisów w katalogu.
        _widgetGroups = _widgetCatalog
            .GroupBy(item => item.Category)
            .Select(group => new WidgetCatalogGroup
            {
                Category = group.Key,
                Title = WidgetCategoryTitle(group.Key),
                Items = group.ToList(),
            })
            .ToList();

        RebuildWidgetGroups();
        FilterWidgetCatalog("");
    }

    private static string WidgetCategoryTitle(string category) => category switch
    {
        "time" => L("WidgetCategoryTime"),
        "date" => L("WidgetCategoryDate"),
        "text" => L("WidgetCategoryText"),
        "images" => L("WidgetCategoryImages"),
        "distance" => L("WidgetCategoryDistance"),
        "goals" => L("WidgetCategoryGoals"),
        "health" => L("WidgetCategoryHealth"),
        "sleep" => L("WidgetCategorySleep"),
        "system" => L("WidgetCategorySystem"),
        "weather" => L("WidgetCategoryWeather"),
        "progress" => L("WidgetCategoryProgress"),
        _ => category,
    };

    private static string WidgetTitle(string kind) => kind switch
    {
        "time" => L("TimeWidgetCardTitle"),
        "analog-time" => L("AnalogWidgetCardTitle"),
        "date" => L("DateWidgetCardTitle"),
        "weekday-number" => L("WeekdayNumberWidgetCardTitle"),
        "week-number" => L("WeekNumberWidgetCardTitle"),
        "day-of-year" => L("DayOfYearWidgetCardTitle"),
        "lunar-date" => L("LunarDateWidgetCardTitle"),
        "festival" => L("FestivalWidgetCardTitle"),
        "world-clock" => L("WorldClockWidgetCardTitle"),
        "text" => L("TextWidgetCardTitle"),
        "images" => L("ImagesWidgetCardTitle"),
        "steps" => L("StepWidgetCardTitle"),
        "distance" => L("DistanceWidgetCardTitle"),
        "steps-target" => L("StepTargetWidgetCardTitle"),
        "calories-target" => L("CaloriesTargetWidgetCardTitle"),
        "stand-target" => L("StandTargetWidgetCardTitle"),
        "fat-burning-target" => L("FatBurningTargetWidgetCardTitle"),
        "heart" => L("HeartWidgetCardTitle"),
        "heart-last" => L("HeartLastWidgetCardTitle"),
        "heart-min" => L("HeartMinWidgetCardTitle"),
        "heart-max" => L("HeartMaxWidgetCardTitle"),
        "heart-avg" => L("HeartAvgWidgetCardTitle"),
        "stress" => L("StressWidgetCardTitle"),
        "pai" => L("PaiWidgetCardTitle"),
        "skin-temp" => L("SkinTempWidgetCardTitle"),
        "sleep" => L("SleepWidgetCardTitle"),
        "sleep-score" => L("SleepScoreWidgetCardTitle"),
        "sleep-deep" => L("SleepDeepWidgetCardTitle"),
        "sleep-light" => L("SleepLightWidgetCardTitle"),
        "sleep-rem" => L("SleepRemWidgetCardTitle"),
        "sleep-wake" => L("SleepWakeWidgetCardTitle"),
        "sleep-start" => L("SleepStartWidgetCardTitle"),
        "sleep-end" => L("SleepEndWidgetCardTitle"),
        "calories" => L("CaloriesWidgetCardTitle"),
        "spo2" => L("Spo2WidgetCardTitle"),
        "spo2-hour-avg" => L("Spo2HourAvgWidgetCardTitle"),
        "stand" => L("StandWidgetCardTitle"),
        "fat-burning" => L("FatBurningWidgetCardTitle"),
        "pai-weekly" => L("PaiWeeklyWidgetCardTitle"),
        "pai-prepai" => L("PaiPrepaiWidgetCardTitle"),
        "sleep-target" => L("SleepTargetWidgetCardTitle"),
        "weather-condition" => L("WeatherConditionWidgetCardTitle"),
        "weather-city" => L("WeatherCityWidgetCardTitle"),
        "weather-current" => L("WeatherCurrentWidgetCardTitle"),
        "weather-high" => L("WeatherHighWidgetCardTitle"),
        "weather-low" => L("WeatherLowWidgetCardTitle"),
        "sunrise" => L("SunriseWidgetCardTitle"),
        "sunset" => L("SunsetWidgetCardTitle"),
        "sun-current" => L("SunCurrentWidgetCardTitle"),
        "weather-icon" => L("WeatherIconWidgetCardTitle"),
        "arc-progress" => L("ArcProgressWidgetCardTitle"),
        "bar-progress" => L("BarProgressWidgetCardTitle"),
        "battery" => L("BatteryWidgetCardTitle"),
        "status-alarm" => L("StatusAlarmWidgetCardTitle"),
        "status-dnd" => L("StatusDndWidgetCardTitle"),
        "status-bluetooth" => L("StatusBluetoothWidgetCardTitle"),
        "status-lock" => L("StatusLockWidgetCardTitle"),
        _ => kind,
    };

    private static string WidgetDescription(string kind) => kind switch
    {
        "time" => L("TimeWidgetCardDesc"),
        "analog-time" => L("AnalogWidgetCardDesc"),
        "date" => L("DateWidgetCardDesc"),
        "weekday-number" => L("WeekdayNumberWidgetCardDesc"),
        "week-number" => L("WeekNumberWidgetCardDesc"),
        "day-of-year" => L("DayOfYearWidgetCardDesc"),
        "lunar-date" => L("LunarDateWidgetCardDesc"),
        "festival" => L("FestivalWidgetCardDesc"),
        "world-clock" => L("WorldClockWidgetCardDesc"),
        "text" => L("TextWidgetCardDesc"),
        "images" => L("ImagesWidgetCardDesc"),
        "steps" => L("StepWidgetCardDesc"),
        "distance" => L("DistanceWidgetCardDesc"),
        "steps-target" => L("StepTargetWidgetCardDesc"),
        "calories-target" => L("CaloriesTargetWidgetCardDesc"),
        "stand-target" => L("StandTargetWidgetCardDesc"),
        "fat-burning-target" => L("FatBurningTargetWidgetCardDesc"),
        "heart" => L("HeartWidgetCardDesc"),
        "heart-last" => L("HeartLastWidgetCardDesc"),
        "heart-min" => L("HeartMinWidgetCardDesc"),
        "heart-max" => L("HeartMaxWidgetCardDesc"),
        "heart-avg" => L("HeartAvgWidgetCardDesc"),
        "stress" => L("StressWidgetCardDesc"),
        "pai" => L("PaiWidgetCardDesc"),
        "skin-temp" => L("SkinTempWidgetCardDesc"),
        "sleep" => L("SleepWidgetCardDesc"),
        "sleep-score" => L("SleepScoreWidgetCardDesc"),
        "sleep-deep" => L("SleepDeepWidgetCardDesc"),
        "sleep-light" => L("SleepLightWidgetCardDesc"),
        "sleep-rem" => L("SleepRemWidgetCardDesc"),
        "sleep-wake" => L("SleepWakeWidgetCardDesc"),
        "sleep-start" => L("SleepStartWidgetCardDesc"),
        "sleep-end" => L("SleepEndWidgetCardDesc"),
        "calories" => L("CaloriesWidgetCardDesc"),
        "spo2" => L("Spo2WidgetCardDesc"),
        "spo2-hour-avg" => L("Spo2HourAvgWidgetCardDesc"),
        "stand" => L("StandWidgetCardDesc"),
        "fat-burning" => L("FatBurningWidgetCardDesc"),
        "pai-weekly" => L("PaiWeeklyWidgetCardDesc"),
        "pai-prepai" => L("PaiPrepaiWidgetCardDesc"),
        "sleep-target" => L("SleepTargetWidgetCardDesc"),
        "weather-condition" => L("WeatherConditionWidgetCardDesc"),
        "weather-city" => L("WeatherCityWidgetCardDesc"),
        "weather-current" => L("WeatherCurrentWidgetCardDesc"),
        "weather-high" => L("WeatherHighWidgetCardDesc"),
        "weather-low" => L("WeatherLowWidgetCardDesc"),
        "sunrise" => L("SunriseWidgetCardDesc"),
        "sunset" => L("SunsetWidgetCardDesc"),
        "sun-current" => L("SunCurrentWidgetCardDesc"),
        "weather-icon" => L("WeatherIconWidgetCardDesc"),
        "arc-progress" => L("ArcProgressWidgetCardDesc"),
        "bar-progress" => L("BarProgressWidgetCardDesc"),
        "battery" => L("BatteryWidgetCardDesc"),
        "status-alarm" => L("StatusAlarmWidgetCardDesc"),
        "status-dnd" => L("StatusDndWidgetCardDesc"),
        "status-bluetooth" => L("StatusBluetoothWidgetCardDesc"),
        "status-lock" => L("StatusLockWidgetCardDesc"),
        _ => kind,
    };

    /// <summary>Wzorzec czasu/daty ma tylko to, co katalog z Rusta oznaczył flagą hasPattern.</summary>
    private bool HasPatternKind(string? kind)
        => _widgetCatalog.Any(item => item.Kind == kind && item.HasPattern);

    /// <summary>Kategoria SNU pochodzi z katalogu z Rusta — bez duplikowania listy typów.</summary>
    private bool IsSleepKind(string? kind)
        => _widgetCatalog.Any(item => item.Kind == kind && item.Category == "sleep");

    private static bool IsDateFieldKind(string? kind) => kind is
        "weekday-number" or "week-number" or "day-of-year" or "lunar-date" or "festival" or "world-clock";

    private static bool IsHealthKind(string? kind) => kind is
        "heart" or "heart-last" or "heart-min" or "heart-max" or "heart-avg"
        or "stress" or "pai" or "skin-temp" or "sleep" or "calories"
        or "spo2" or "spo2-hour-avg" or "stand" or "fat-burning"
        or "pai-weekly" or "pai-prepai";

    private static bool IsGoalKind(string? kind) => kind is
        "steps-target" or "calories-target" or "stand-target" or "fat-burning-target" or "sleep-target";

    private static bool IsWeatherKind(string? kind) => kind is
        "weather-condition" or "weather-city" or "weather-current" or "weather-high"
        or "weather-low" or "sunrise" or "sunset" or "sun-current" or "weather-icon";

    private static bool IsWeatherIcon(LayerView? layer) => layer?.Kind == "weather-icon";

    private static bool IsProgressKind(string? kind) => kind is "arc-progress" or "bar-progress";

    private static ChoiceItem[] ProgressSourceChoices() => new[]
    {
        new ChoiceItem { Value = "battery", Name = L("ProgressSourceBattery") },
        new ChoiceItem { Value = "steps", Name = L("ProgressSourceSteps") },
        new ChoiceItem { Value = "distance", Name = L("ProgressSourceDistance") },
        new ChoiceItem { Value = "heart", Name = L("ProgressSourceHeart") },
        new ChoiceItem { Value = "calories", Name = L("ProgressSourceCalories") },
        new ChoiceItem { Value = "stand", Name = L("ProgressSourceStand") },
        new ChoiceItem { Value = "fat-burning", Name = L("ProgressSourceFatBurning") },
        new ChoiceItem { Value = "pai", Name = L("ProgressSourcePai") },
        new ChoiceItem { Value = "spo2", Name = L("ProgressSourceSpo2") },
        new ChoiceItem { Value = "sleep", Name = L("ProgressSourceSleep") },
        new ChoiceItem { Value = "fixed", Name = L("ProgressSourceFixed") },
    };

    private static void SelectChoice(ComboBox combo, IEnumerable<ChoiceItem> items, string value)
    {
        combo.ItemsSource = items;
        combo.SelectedItem = items.FirstOrDefault(item => item.Value == value);
    }

    private void UpdateProgressColorUI()
    {
        if (_settings is null) return;
        var p = _settings.Progress;
        var fill = ParseArgb(p.Color);
        var track = ParseArgb(p.TrackColor);
        ProgressColorSwatch.Background = new SolidColorBrush(fill);
        ProgressTrackColorSwatch.Background = new SolidColorBrush(track);
        ProgressColorValue.Text = $"#{fill.A:X2}{fill.R:X2}{fill.G:X2}{fill.B:X2}";
        ProgressTrackColorValue.Text = $"#{track.A:X2}{track.R:X2}{track.G:X2}{track.B:X2}";
    }

    private void LoadProgressSettings()
    {
        if (_settings is null) return;
        var p = _settings.Progress;
        var kind = _layers.FirstOrDefault(layer => layer.Id == _selectedLayerId)?.Kind;
        _loadingProgress = true;
        SelectChoice(ProgressSourceCombo, ProgressSourceChoices(), p.Source);
        ProgressThicknessBox.Value = p.Thickness;
        ProgressArcSizeSlider.Value = Math.Clamp(Math.Min(_settings.W, _settings.H), 24, 466);
        ProgressArcSizeValue.Text = $"{Math.Round(ProgressArcSizeSlider.Value)} px";
        ProgressWidthBox.Value = _settings.W;
        ProgressHeightBox.Value = _settings.H;
        ProgressArcSizePanel.Visibility = kind == "arc-progress" ? Visibility.Visible : Visibility.Collapsed;
        ProgressBarSizePanel.Visibility = kind == "bar-progress" ? Visibility.Visible : Visibility.Collapsed;
        ProgressStartAngleBox.Value = p.StartAngle;
        ProgressEndAngleBox.Value = p.EndAngle;
        // Kąty dotyczą wyłącznie łuku — w pasku nie są pokazywane.
        ProgressAnglePanel.Visibility = kind == "arc-progress" ? Visibility.Visible : Visibility.Collapsed;
        ProgressRoundedCheckBox.IsChecked = p.Corner != "flat";
        ProgressTrackCheckBox.IsChecked = p.ShowTrack;
        UpdateProgressColorUI();
        _loadingProgress = false;
    }

    private void ProgressCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_settings is null || _loadingProgress || e.AddedItems.FirstOrDefault() is not ChoiceItem item) return;
        if (sender is not ComboBox combo || combo != ProgressSourceCombo) return;
        _settings.Progress.Source = item.Value;
        ApplyPreview();
        PersistSettings();
    }

    private void ProgressNumber_ValueChanged(NumberBox sender, NumberBoxValueChangedEventArgs args)
    {
        if (_settings is null || _loadingProgress || sender is not NumberBox box || double.IsNaN(box.Value)) return;
        var p = _settings.Progress;
        if (box == ProgressThicknessBox) p.Thickness = (uint)Math.Clamp(box.Value, 1, 233);
        else if (box == ProgressStartAngleBox) p.StartAngle = (int)Math.Clamp(box.Value, -360, 360);
        else if (box == ProgressEndAngleBox) p.EndAngle = (int)Math.Clamp(box.Value, -360, 360);
        else return;
        ApplyPreview();
        PersistSettings();
    }

    /// <summary>Suwak rozmiaru kwadratowego łuku — środek zostaje w miejscu.</summary>
    private void ProgressArcSizeSlider_ValueChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (_settings is null || _loadingProgress || sender is not Slider slider) return;
        var size = (int)Math.Clamp(Math.Round(slider.Value), 24, ScreenSize - 24);
        ProgressArcSizeValue.Text = $"{size} px";
        var cx = _settings.X + _settings.W / 2.0;
        var cy = _settings.Y + _settings.H / 2.0;
        var x = (int)Math.Clamp(cx - size / 2.0, 0, ScreenSize - size);
        var y = (int)Math.Clamp(cy - size / 2.0, 0, ScreenSize - size);
        if (x == _settings.X && y == _settings.Y && size == _settings.W && size == _settings.H) return;
        _settings.X = x;
        _settings.Y = y;
        _settings.W = size;
        _settings.H = size;
        if (_frames.TryGetValue(_selectedLayerId ?? "", out var view))
        {
            view.Layer.X = x;
            view.Layer.Y = y;
            view.Layer.W = size;
            view.Layer.H = size;
        }
        ApplyPreview();
        PersistSettings();
    }

    /// <summary>Wpisywanie wymiarów paska (szerokość i wysokość w px).</summary>
    private void ProgressSize_ValueChanged(NumberBox sender, NumberBoxValueChangedEventArgs args)
    {
        if (_settings is null || _loadingProgress || sender is not NumberBox box || double.IsNaN(box.Value)) return;
        var width = _settings.W;
        var height = _settings.H;
        var x = _settings.X;
        var y = _settings.Y;
        if (box == ProgressWidthBox)
            width = (int)Math.Clamp(box.Value, 24, ScreenSize - _settings.X);
        else if (box == ProgressHeightBox)
            height = (int)Math.Clamp(box.Value, 24, ScreenSize - _settings.Y);
        else return;
        if (width == _settings.W && height == _settings.H && x == _settings.X && y == _settings.Y) return;
        _settings.X = x;
        _settings.Y = y;
        _settings.W = width;
        _settings.H = height;
        if (_frames.TryGetValue(_selectedLayerId ?? "", out var view))
        {
            view.Layer.X = x;
            view.Layer.Y = y;
            view.Layer.W = width;
            view.Layer.H = height;
        }
        ApplyPreview();
        PersistSettings();
    }

    /// <summary>Wybór koloru identyczny jak dla zwykłego tekstu — ten sam ColorPicker i swatch.</summary>
    private void ProgressColorButton_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || sender is not Button button) return;
        var track = button == ProgressTrackColorButton;
        var p = _settings.Progress;
        var picker = new ColorPicker { Color = ParseArgb(track ? p.TrackColor : p.Color) };
        picker.ColorChanged += (_, args) =>
        {
            if (_settings is null) return;
            var c = args.NewColor;
            var hex = $"0x{c.A:X2}{c.R:X2}{c.G:X2}{c.B:X2}";
            if (track) _settings.Progress.TrackColor = hex;
            else _settings.Progress.Color = hex;
            UpdateProgressColorUI();
            ApplyPreview();
            PersistSettings();
        };
        new Flyout { Content = picker }.ShowAt(button);
    }

    private void ProgressCheck_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || _loadingProgress || sender is not CheckBox box) return;
        if (box == ProgressRoundedCheckBox)
        {
            _settings.Progress.Corner = box.IsChecked == true ? "round" : "flat";
        }
        else if (box == ProgressTrackCheckBox)
            _settings.Progress.ShowTrack = box.IsChecked == true;
        else return;
        ApplyPreview();
        PersistSettings();
    }

    /// <summary>Teksty czytane z czujnika/firmware'u: bateria i pola pogody (bez ikony).</summary>
    private static bool IsSensorTextKind(string? kind)
        => kind == "battery" || (IsWeatherKind(kind) && kind != "weather-icon");

    /// <summary>Tylko widgety z jednostką mają w inspektorze przełącznik jej pokazywania.</summary>
    private static bool HasUnitKind(string? kind) => kind is
        "heart" or "heart-last" or "heart-min" or "heart-max" or "heart-avg"
        or "skin-temp" or "calories" or "spo2" or "spo2-hour-avg"
        or "calories-target" or "stand-target" or "fat-burning-target"
        or "stand" or "fat-burning" or "weather-current";

    private static ListView CreateCatalogList(DataTemplate template, bool selectable,
        SelectionChangedEventHandler? selectionChanged)
    {
        var list = new ListView
        {
            SelectionMode = selectable ? ListViewSelectionMode.Single : ListViewSelectionMode.None,
            Padding = new Thickness(0),
            ItemTemplate = template,
        };
        ScrollViewer.SetVerticalScrollBarVisibility(list, ScrollBarVisibility.Disabled);
        if (selectionChanged is not null) list.SelectionChanged += selectionChanged;
        return list;
    }

    private void RebuildWidgetGroups()
    {
        WidgetFlyoutGroups.Children.Clear();
        WidgetsCatalogGroups.Children.Clear();
        var flyoutTemplate = (DataTemplate?)WidgetFlyoutRoot.Resources["WidgetFlyoutItemTemplate"];
        var tabTemplate = (DataTemplate?)WidgetsCatalogGroups.Resources["WidgetsCatalogItemTemplate"];

        foreach (var group in _widgetGroups)
        {
            var flyoutList = CreateCatalogList(flyoutTemplate!, selectable: true, WidgetFlyoutList_SelectionChanged);
            flyoutList.ItemsSource = group.Items;
            var flyoutExpander = new Expander
            {
                Header = new TextBlock { Text = group.Title, FontSize = 13, FontWeight = FontWeights.SemiBold },
                Content = flyoutList,
                IsExpanded = false,
                HorizontalAlignment = HorizontalAlignment.Stretch,
            };
            group.FlyoutList = flyoutList;
            group.FlyoutExpander = flyoutExpander;
            WidgetFlyoutGroups.Children.Add(flyoutExpander);

            // Zakładka Widgets: kategorie zawsze rozwinięte, bez strzałek.
            var tabList = CreateCatalogList(tabTemplate!, selectable: false, selectionChanged: null);
            tabList.ItemsSource = group.Items;
            WidgetsCatalogGroups.Children.Add(new StackPanel
            {
                Spacing = 4,
                HorizontalAlignment = HorizontalAlignment.Stretch,
                Children =
                {
                    new TextBlock
                    {
                        Text = group.Title,
                        FontSize = 15,
                        FontWeight = FontWeights.SemiBold,
                        Margin = new Thickness(0, 4, 0, 0),
                    },
                    tabList,
                },
            });
        }
    }

    private void FilterWidgetCatalog(string query)
    {
        var text = query.Trim();
        var total = 0;
        foreach (var group in _widgetGroups)
        {
            var matches = group.Items.Where(item =>
                text.Length == 0
                || item.Title.Contains(text, StringComparison.OrdinalIgnoreCase)
                || item.Description.Contains(text, StringComparison.OrdinalIgnoreCase)
                || item.Kind.Contains(text, StringComparison.OrdinalIgnoreCase)
                || group.Title.Contains(text, StringComparison.OrdinalIgnoreCase)).ToList();
            total += matches.Count;
            if (group.FlyoutList is not null) group.FlyoutList.ItemsSource = matches;
            if (group.FlyoutExpander is not null)
            {
                group.FlyoutExpander.Visibility = matches.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
                // Wyszukiwanie sięga także zwiniętych kategorii — trafienia same rozwijają sekcję.
                group.FlyoutExpander.IsExpanded = text.Length > 0 && matches.Count > 0;
            }
        }
        WidgetFlyoutEmpty.Visibility = total == 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void AddWidgetButton_Click(object sender, RoutedEventArgs e)
    {
        WidgetSearchBox.Text = "";
        FilterWidgetCatalog("");
    }

    private void WidgetSearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_initialized) return;
        FilterWidgetCatalog(WidgetSearchBox.Text);
    }

    private void WidgetFlyoutList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || e.AddedItems.FirstOrDefault() is not WidgetCatalogItem item) return;
        if (sender is ListView list) list.SelectedItem = null;
        AddWidget(item.Kind);
        AddWidgetFlyout.Hide();
    }

    private async void AddWidget(string kind)
    {
        if (_projectFile is null) return;
        var project = _projectFile;
        // Dodanie ikony pogody/wskazówek kopiuje i skaluje wiele plików — w tle.
        string? raw = null;
        string? error = null;
        await Task.Run(() => raw = RustNative.AddWidget(project, kind, out error)).ConfigureAwait(true);
        if (raw is null)
        {
            ShowStatus(error is null ? L("ErrorAddWidget") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(raw);
        _selectedLayerId = document.RootElement.GetProperty("id").GetString();
        _selectionCleared = false;
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        ApplyDefaultFontToMissing();
        LoadWidgetSettings();
        NavView.SelectedItem = EditorNavItem;
        ShowView(editor: true);
        ShowStatus(L("WidgetAdded"));
    }

    /// <summary>
    /// Zapisuje ustawienia widgetu. Domyślnie tylko geometrię — przeskalowanie
    /// zasobów jest kolejkowane w tle (patrz <see cref="ScheduleAssetRescale"/>).
    /// </summary>
    private void PersistSettings(bool deferAssets = true)
    {
        if (_projectFile is null || _settings is null || _loadingSettings || _selectedLayerId is null) return;
        var kind = _layers.FirstOrDefault(l => l.Id == _selectedLayerId)?.Kind;
        var isUserText = kind == "text";
        var isDistance = kind == "distance";
        var distanceUnits = isDistance && _settings.OwnMeasurementSystem
            ? _settings.MeasurementSystem
            : "";
        var payload = JsonSerializer.Serialize(new
        {
            id = _selectedLayerId,
            x = _settings.X,
            y = _settings.Y,
            w = _settings.W,
            h = _settings.H,
            textSize = _settings.TextSize,
            font = _settings.Font,
            color = _settings.Color,
            align = _settings.Align,
            vAlign = _settings.VAlign,
            // Tylko time i date mają wzorzec. Wysłanie timeFormat dla baterii
            // lub pogody nadpisałoby ich tekst na "00:00:00" (regresja).
            timeFormat = HasPatternKind(kind) ? _settings.Pattern : null,
            text = isUserText ? _settings.UserText : null,
            // Pusty string = użyj systemu z ustawień aplikacji (bez nadpisania).
            measurementSystem = distanceUnits,
            showUnit = _settings.ShowUnit,
            alpha = _settings.Alpha,
            autoScale = _settings.AutoScale,
            textStyle = _settings.TextStyle,
            imageFit = _settings.ImageFit,
            weatherDayOffset = _settings.WeatherDayOffset,
            sleepStageMode = _settings.SleepStageMode,
            progress = new
            {
                source = _settings.Progress.Source,
                min = _settings.Progress.Min,
                max = _settings.Progress.Max,
                value = _settings.Progress.Value,
                color = _settings.Progress.Color,
                trackColor = _settings.Progress.TrackColor,
                thickness = _settings.Progress.Thickness,
                startAngle = _settings.Progress.StartAngle,
                endAngle = _settings.Progress.EndAngle,
                corner = _settings.Progress.Corner,
                radius = _settings.Progress.Radius,
                segments = _settings.Progress.Segments,
                gap = _settings.Progress.Gap,
                orientation = _settings.Progress.Orientation,
                invert = _settings.Progress.Invert,
                showTrack = _settings.Progress.ShowTrack,
            },
            deferAssets,
        });
        var raw = RustNative.SetWidget(_projectFile, payload, out var error);
        if (raw is null)
        {
            EditorStatusText.Text = error is null ? L("ErrorSaveChanges") : LocalizationService.Error(error);
            return;
        }

        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        EditorStatusText.Text = L("SavedStatus");
        if (deferAssets) ScheduleAssetRescale(_selectedLayerId);
    }

    /// <summary>
    /// Kolejkuje przeskalowanie wszystkich zasobów widgetu w wątku tła.
    /// Szybkie zmiany (przeciąganie, suwaki) anulują poprzednie zadanie —
    /// wykonuje się tylko ostatnie, po 180 ms bez zmian.
    /// </summary>
    private void ScheduleAssetRescale(string widgetId)
    {
        var cts = new CancellationTokenSource();
        _assetJobCts?.Cancel();
        _assetJobCts = cts;
        var token = cts.Token;
        var project = _projectFile;
        if (project is null) return;
        _assetJob = Task.Run(async () =>
        {
            try
            {
                await Task.Delay(180, token).ConfigureAwait(false);
                if (token.IsCancellationRequested) return;
                await _assetGate.WaitAsync(token).ConfigureAwait(false);
                try
                {
                    if (token.IsCancellationRequested) return;
                    RustNative.RescaleWidgetAssets(project, widgetId, out var error);
                    if (error is not null && !token.IsCancellationRequested)
                    {
                        DispatcherQueue.TryEnqueue(() =>
                            EditorStatusText.Text = LocalizationService.Error(error));
                    }
                }
                finally
                {
                    _assetGate.Release();
                }
            }
            catch (OperationCanceledException) { }
        }, token);
    }

    /// <summary>Export czeka na zakończenie ostatniego zadania skalowania.</summary>
    private async Task WaitForAssetJobsAsync()
    {
        // Eksport musi widzieć ostateczny stan projektu także po przełączeniu
        // paczki ikon — w przeciwnym razie ZIP miałby poprzedni zestaw.
        foreach (var job in new[] { _assetJob, _weatherPackJob })
        {
            if (!job.IsCompleted) await job.ConfigureAwait(true);
        }
    }

    private void ApplyLayersPayload(string json)
    {
        var next = JsonSerializer.Deserialize<List<LayerView>>(json) ?? new();
        RestoreLayerSelection(next);

        // Zmiana geometrii nie zmienia struktury — odświeżamy istniejące obiekty zamiast
        // tworzyć nowe wiersze i kontrolki (to powodowało skakanie listy i mruganie podglądu).
        var rebuild = !SameLayerStructure(_layers, next);
        _layers = next;
        if (rebuild)
        {
            RefreshLayersList();
            RebuildPreview();
        }
        else
        {
            UpdateExistingLayerViews(next);
        }
    }

    private static bool SameLayerStructure(List<LayerView> current, List<LayerView> next)
    {
        if (current.Count != next.Count) return false;
        for (var i = 0; i < current.Count; i++)
        {
            if (current[i].Id != next[i].Id
                || current[i].Kind != next[i].Kind
                || current[i].Visible != next[i].Visible
                // Ikona pogody w edytorze zawsze pochodzi z magazynu (przez
                // rw_format_weather_field); zmiana Src w projekcie nie wymaga
                // przebudowy podglądu — wystarczy aktualizacja w miejscu.
                || (current[i].Src != next[i].Src && current[i].Kind != "weather-icon")
                || current[i].HandSet != next[i].HandSet
                || current[i].HourSrc != next[i].HourSrc
                || current[i].MinuteSrc != next[i].MinuteSrc
                || current[i].SecondSrc != next[i].SecondSrc
                || current[i].CapSrc != next[i].CapSrc)
                return false;
        }
        return true;
    }

    private void UpdateExistingLayerViews(List<LayerView> next)
    {
        foreach (var layer in next)
        {
            // Te same instancje są źródłem wiązań wiersza i ramki — aktualizujemy je w miejscu.
            var rowLayer = (LayersList.ItemsSource as IEnumerable<LayerRow>)?
                .FirstOrDefault(row => row.Id == layer.Id)?.Layer;
            if (rowLayer is not null) CopyLayerState(rowLayer, layer);

            if (!_frames.TryGetValue(layer.Id, out var view)) continue;
            CopyLayerState(view.Layer, layer);
            view.Host.Opacity = layer.Alpha / 255.0;
            ApplyFrameGeometry(view, layer.X, layer.Y, layer.W, layer.H, layer.TextSize ?? 64);
            view.Text.Foreground = new SolidColorBrush(ParseArgb(layer.Color ?? "0xFFFFFFFF"));
            if (!IsWeatherIcon(layer)) view.Text.Text = layer.Text ?? view.Text.Text;
            ApplyTextFit(view, layer.TextStyle);
            ApplyFont(view);
        }
        UpdateFrameVisuals();
        UpdateClockText();
    }

    private static void CopyLayerState(LayerView target, LayerView source)
    {
        target.Name = source.Name;
        target.X = source.X;
        target.Y = source.Y;
        target.W = source.W;
        target.H = source.H;
        target.TextSize = source.TextSize;
        target.Font = source.Font;
        target.Text = source.Text;
        target.TimeFormat = source.TimeFormat;
        target.Color = source.Color;
        target.Src = source.Src;
        target.ImageFit = source.ImageFit;
        target.Align = source.Align;
        target.VAlign = source.VAlign;
        target.ShowUnit = source.ShowUnit;
        target.Alpha = source.Alpha;
        target.WeatherIconSet = source.WeatherIconSet;
        target.AutoScale = source.AutoScale;
        target.TextStyle = source.TextStyle;
        target.Visible = source.Visible;
        target.HandSet = source.HandSet;
        target.HourSrc = source.HourSrc;
        target.MinuteSrc = source.MinuteSrc;
        target.SecondSrc = source.SecondSrc;
        target.CapSrc = source.CapSrc;
        target.WeatherDayOffset = source.WeatherDayOffset;
        target.SleepStageMode = source.SleepStageMode;
        target.Progress = source.Progress;
    }

    private static Color ParseArgb(string value)
    {
        var hex = value.StartsWith("0x", StringComparison.OrdinalIgnoreCase) ? value[2..] : value;
        if (hex.Length != 8) return Color.FromArgb(255, 255, 255, 255);
        byte Part(int start) => byte.Parse(hex.AsSpan(start, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
        return Color.FromArgb(Part(0), Part(2), Part(4), Part(6));
    }

    private void UpdateColorUI()
    {
        if (_settings is null) return;
        ColorSwatch.Background = new SolidColorBrush(ParseArgb(_settings.Color));
        ColorValue.Text = _settings.Color.Replace("0x", "#", StringComparison.OrdinalIgnoreCase);
    }

    private void UpdateScaleModeUI()
    {
        if (_settings is null) return;
        var label = _settings.AutoScale ? L("ScaleModeOnLabel") : L("ScaleModeOffLabel");
        ScaleModeButtonText.Text = label;
        StatusScaleModeButtonText.Text = label;
    }

    private void ClockTimer_Tick(object? sender, object e)
    {
        // Domyślnie podgląd jest statyczny — nie formatujemy nic co sekundę.
        if (!AppSettingsStore.LiveTimeDateEnabled) return;
        UpdateLiveTimeDateOnly();
    }

    private void FormatBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (_loadingSettings || _settings is null) return;
        _settings.Pattern = FormatBox.Text;
        ApplyPreview();
        PersistSettings();
    }

    private void UserTextBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (_loadingSettings || _settings is null) return;
        _settings.UserText = UserTextBox.Text;
        if (_selectedLayerId is not null
            && _frames.TryGetValue(_selectedLayerId, out var view)
            && view.Layer.Kind == "text")
        {
            view.Text.Text = _settings.UserText;
        }
        PersistSettings();
    }

    private void FormatPreset_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || _loadingSettings || sender is not ComboBox combo) return;
        var pattern = combo.SelectedItem switch
        {
            ComboBoxItem item => item.Content as string,
            string text => text,
            _ => null,
        };
        if (pattern is not null) FormatBox.Text = pattern;
    }

    private void SizeSlider_ValueChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (_loadingSettings || _settings is null) return;
        _settings.TextSize = (uint)e.NewValue;
        FontSizeValue.Text = _settings.TextSize.ToString();
        ApplyPreview();
        PersistSettings();
    }

    private void OpacitySlider_ValueChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (_loadingSettings || _settings is null) return;
        _settings.Alpha = (byte)e.NewValue;
        OpacityValue.Text = _settings.Alpha.ToString();
        ApplyPreview();
        PersistSettings();
    }

    private void PositionSlider_ValueChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (_loadingSettings || _settings is null) return;
        if (ReferenceEquals(sender, XSlider)) _settings.X = (int)e.NewValue;
        else if (ReferenceEquals(sender, YSlider)) _settings.Y = (int)e.NewValue;
        UpdatePositionValues();
        ApplyPreview();
        PersistSettings();
    }

    private void PositionValue_Tapped(object sender, TappedRoutedEventArgs e)
    {
        if (sender is not TextBlock block) return;
        // Opóźnienie pozwala DoubleTapped anulować kopiowanie.
        _pendingPositionValue = block;
        _positionTapTimer.Stop();
        _positionTapTimer.Tick -= PositionTapTimer_Tick;
        _positionTapTimer.Tick += PositionTapTimer_Tick;
        _positionTapTimer.Start();
    }

    private void PositionTapTimer_Tick(object? sender, object e)
    {
        _positionTapTimer.Stop();
        _positionTapTimer.Tick -= PositionTapTimer_Tick;
        var block = _pendingPositionValue;
        _pendingPositionValue = null;
        if (block is null) return;
        try
        {
            var package = new DataPackage();
            package.SetText(block.Text);
            Clipboard.SetContent(package);
            EditorStatusText.Text = LF("PositionCopied", block.Text);
        }
        catch (Exception)
        {
            // Schowek bywa zablokowany przez inną aplikację — nie może wywalić edytora.
            EditorStatusText.Text = L("PositionCopyFailed");
        }
    }

    private void PositionValue_DoubleTapped(object sender, DoubleTappedRoutedEventArgs e)
    {
        _positionTapTimer.Stop();
        _positionTapTimer.Tick -= PositionTapTimer_Tick;
        _pendingPositionValue = null;
        if (sender is not TextBlock block) return;
        var isX = ReferenceEquals(block, XPositionValue);
        var box = isX ? XPositionBox : YPositionBox;
        box.Text = block.Text;
        block.Visibility = Visibility.Collapsed;
        box.Visibility = Visibility.Visible;
        box.Focus(FocusState.Programmatic);
        box.SelectAll();
        e.Handled = true;
    }

    private void PositionBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == VirtualKey.Enter)
        {
            CommitPositionEdit(sender as TextBox);
            e.Handled = true;
        }
        else if (e.Key == VirtualKey.Escape)
        {
            CancelPositionEdit(sender as TextBox);
            e.Handled = true;
        }
    }

    private void PositionBox_LostFocus(object sender, RoutedEventArgs e)
        => CommitPositionEdit(sender as TextBox);

    private void CommitPositionEdit(TextBox? box)
    {
        if (box is null || box.Visibility != Visibility.Visible) return;
        var isX = ReferenceEquals(box, XPositionBox);
        var block = isX ? XPositionValue : YPositionValue;
        var maximum = (int)Math.Max(0, isX ? XSlider.Maximum : YSlider.Maximum);
        if (int.TryParse(box.Text.Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            && _settings is not null && !_loadingSettings)
        {
            var clamped = Math.Clamp(value, 0, maximum);
            if (clamped != (isX ? _settings.X : _settings.Y))
            {
                if (isX) _settings.X = clamped;
                else _settings.Y = clamped;
                if (isX) XSlider.Value = clamped;
                else YSlider.Value = clamped;
                UpdatePositionValues();
                ApplyPreview();
                PersistSettings();
            }
        }
        box.Visibility = Visibility.Collapsed;
        block.Visibility = Visibility.Visible;
    }

    private void CancelPositionEdit(TextBox? box)
    {
        if (box is null || box.Visibility != Visibility.Visible) return;
        var block = ReferenceEquals(box, XPositionBox) ? XPositionValue : YPositionValue;
        box.Visibility = Visibility.Collapsed;
        block.Visibility = Visibility.Visible;
    }

    /// <summary>Wyśrodkowuje cały widget na tarczy — niezależnie od jego typu.</summary>
    private void CenterOnCanvasButton_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || _loadingSettings) return;
        _settings.X = (ScreenSize - _settings.W) / 2;
        _settings.Y = (ScreenSize - _settings.H) / 2;
        SyncPositionSliders();
        ApplyPreview();
        PersistSettings();
    }

    /// <summary>Wyśrodkowuje tylko w poziomie — wysokość pozostaje bez zmian.</summary>
    private void CenterHorizontallyButton_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || _loadingSettings) return;
        _settings.X = Math.Max(0, (ScreenSize - _settings.W) / 2);
        SyncPositionSliders();
        ApplyPreview();
        PersistSettings();
    }

    /// <summary>Wyśrodkowuje tylko w pionie — pozycja X pozostaje bez zmian.</summary>
    private void CenterVerticallyButton_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || _loadingSettings) return;
        _settings.Y = Math.Max(0, (ScreenSize - _settings.H) / 2);
        SyncPositionSliders();
        ApplyPreview();
        PersistSettings();
    }

    private void AlignButton_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || sender is not Button button) return;
        _settings.Align = button.Tag?.ToString() ?? "center";
        ApplyPreview();
        PersistSettings();
    }

    private void FitComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingSettings || _loadingFitOptions || _settings is null) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        _settings.TextStyle = item.Value;
        ApplyPreview();
        PersistSettings();
    }

    private void VAlignButton_Click(object sender, RoutedEventArgs e)
    {
        if (_settings is null || sender is not Button button) return;
        _settings.VAlign = button.Tag?.ToString() ?? "center";
        ApplyPreview();
        PersistSettings();
    }

    private void ScaleModeButton_Click(object sender, RoutedEventArgs e)
    {
        var layer = _layers.FirstOrDefault(item => item.Id == _selectedLayerId);
        if (layer is not null) ToggleScaleMode(layer);
    }

    private void ColorButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new ColorPicker();
        var current = _settings is null ? Color.FromArgb(255, 255, 255, 255) : ParseArgb(_settings.Color);
        picker.Color = current;
        picker.ColorChanged += (_, args) =>
        {
            if (_settings is null) return;
            var c = args.NewColor;
            _settings.Color = $"0x{c.A:X2}{c.R:X2}{c.G:X2}{c.B:X2}";
            UpdateColorUI();
            ApplyPreview();
            PersistSettings();
        };
        new Flyout { Content = picker }.ShowAt(ColorButton);
    }

    // --- Zasoby: czcionki ----------------------------------------------------

    private void FontComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingSettings || _loadingFontChoices || _settings is null) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        _settings.Font = item.Value;
        ApplyPreview();
        PersistSettings();
    }

    private void FontsList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized) return;
        _selectedFontFile = (FontsList.SelectedItem as FontItem)?.File;
        RemoveFontButton.IsEnabled = FontsList.SelectedItem is FontItem { IsDefault: false };
    }

    private async void AddFontButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new Windows.Storage.Pickers.FileOpenPicker
        {
            SuggestedStartLocation = PickerLocationId.Desktop,
        };
        picker.FileTypeFilter.Add(".ttf");
        InitializeWithWindow.Initialize(picker, WindowHandle);
        var file = await picker.PickSingleFileAsync();
        if (file is null) return;

        var added = RustNative.FontsAdd(file.Path, out var error);
        if (added is null)
        {
            AssetsStatusText.Text = error is null ? L("AssetsFontAddFailed") : LocalizationService.Error(error);
            return;
        }
        _selectedFontFile = added.File;
        LoadFonts();
        AssetsStatusText.Text = LF("AssetsFontAdded", added.Family);
    }

    private void RemoveFontButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedFontFile is null) return;
        RustNative.FontsRemove(_selectedFontFile, out var error);
        if (error is not null)
        {
            AssetsStatusText.Text = LocalizationService.Error(error);
            return;
        }
        // Widgety w już istniejących projektach zachowują swoją kopię pliku w `assets/fonts`.
        _selectedFontFile = null;
        LoadFonts();
        AssetsStatusText.Text = L("AssetsFontRemoved");
    }

    private async void OpenFontsFolderButton_Click(object sender, RoutedEventArgs e)
    {
        await Launcher.LaunchFolderAsync(await StorageFolder.GetFolderFromPathAsync(AppPaths.FontsDirectory));
    }

    // --- Języki interfejsu (pliki w %APPDATA%\RustyWatch\Localization) --------

    private async void OpenLanguagesFolderButton_Click(object sender, RoutedEventArgs e)
    {
        await Launcher.LaunchFolderAsync(await StorageFolder.GetFolderFromPathAsync(AppPaths.LocalizationDirectory));
    }

    private void ReloadLanguagesButton_Click(object sender, RoutedEventArgs e)
    {
        LocalizationService.Reload();
        // To ten sam obiekt List, więc ComboBox nie zorientuje się sam — podmieniamy
        // ItemsSource, żeby nowe pliki pojawiły się w selektorze bez restartu.
        LanguageCombo.ItemsSource = null;
        LanguageCombo.ItemsSource = LocalizationService.Languages;
        LanguageCombo.SelectedItem = LocalizationService.Current;
        ApplyLocalization();
        ShowStatus(L("LanguagesReloaded"));
    }

    // --- Zasoby: obrazki tła -------------------------------------------------

    // --- Zasoby: obrazki widgetu IMAGES -------------------------------------

    private void ImagesList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized) return;
        _selectedImageFile = (ImagesList.SelectedItem as ImageItem)?.File;
        RemoveImageButton.IsEnabled = ImagesList.SelectedItem is not null;
    }

    private async void AddImageButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FileOpenPicker { SuggestedStartLocation = PickerLocationId.PicturesLibrary };
        picker.FileTypeFilter.Add(".png");
        picker.FileTypeFilter.Add(".jpg");
        picker.FileTypeFilter.Add(".jpeg");
        InitializeWithWindow.Initialize(picker, WindowHandle);
        var file = await picker.PickSingleFileAsync();
        if (file is null) return;

        var added = RustNative.ImagesAdd(file.Path, out var error);
        if (added is null)
        {
            ImagesStatusText.Text = error is null ? L("AssetsImageAddFailed") : LocalizationService.Error(error);
            return;
        }
        _selectedImageFile = added.File;
        LoadImages();
        ImagesStatusText.Text = LF("AssetsImageAdded", added.File);
    }

    private void RemoveImageButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedImageFile is null) return;
        RustNative.ImagesRemove(_selectedImageFile, out var error);
        if (error is not null)
        {
            ImagesStatusText.Text = LocalizationService.Error(error);
            return;
        }
        // Kopie w projektach pozostają — eksportowana tarcza jest samowystarczalna.
        _selectedImageFile = null;
        LoadImages();
        ImagesStatusText.Text = L("AssetsImageRemoved");
    }

    private async void OpenImagesFolderButton_Click(object sender, RoutedEventArgs e)
    {
        await Launcher.LaunchFolderAsync(await StorageFolder.GetFolderFromPathAsync(AppPaths.ImagesDirectory));
    }

    private void WidgetImageComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingImages || !_initialized || _projectFile is null || _settings is null) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        var layer = _layers.FirstOrDefault(l => l.Id == _selectedLayerId && l.Kind == "images");
        if (layer is null || string.IsNullOrEmpty(item.Value)) return;
        var raw = RustNative.SetWidgetImage(
            _projectFile, layer.Id, item.Value, _settings.ImageFit, out var error);
        if (raw is null)
        {
            ImagesStatusText.Text = error is null ? L("ImageApplyFailed") : LocalizationService.Error(error);
            return;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        RebuildPreview();
        LoadWidgetSettings();
        ImagesStatusText.Text = LF("ImageApplied", item.Value);
    }

    private void ImageFitComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingImages || _loadingSettings || _settings is null) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        _settings.ImageFit = item.Value;
        PersistSettings();
        var layer = _layers.FirstOrDefault(l => l.Id == _selectedLayerId && l.Kind == "images");
        if (layer is null || string.IsNullOrEmpty(layer.Src)) return;
        RebuildPreview();
    }

    // --- Zasoby: zestawy wskazówek zegara analogowego -----------------------

    private void HandsList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized) return;
        _selectedHandSetId = (HandsList.SelectedItem as HandSetItem)?.Id;
        RemoveHandsButton.IsEnabled = HandsList.SelectedItem is HandSetItem { Id: not "Classic" };
    }

    private async void AddHandsButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker { SuggestedStartLocation = PickerLocationId.PicturesLibrary };
        picker.FileTypeFilter.Add("*");
        InitializeWithWindow.Initialize(picker, WindowHandle);
        var folder = await picker.PickSingleFolderAsync();
        if (folder is null) return;

        var added = RustNative.HandsAdd(folder.Path, out var error);
        if (added is null)
        {
            HandsStatusText.Text = error is null ? L("AssetsHandsAddFailed") : LocalizationService.Error(error);
            return;
        }
        _selectedHandSetId = added.Id;
        LoadHandSets(); LoadStatusPacks(); LoadWeatherPacks();
        HandsStatusText.Text = LF("AssetsHandsAdded", added.Name);
    }

    private void RemoveHandsButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedHandSetId is null) return;
        RustNative.HandsRemove(_selectedHandSetId, out var error);
        if (error is not null)
        {
            HandsStatusText.Text = LocalizationService.Error(error);
            return;
        }
        // Projekty mają własne kopie — usunięcie z magazynu nic im nie robi.
        _selectedHandSetId = null;
        LoadHandSets(); LoadStatusPacks(); LoadWeatherPacks();
        HandsStatusText.Text = L("AssetsHandsRemoved");
    }

    private async void OpenHandsFolderButton_Click(object sender, RoutedEventArgs e)
    {
        await Launcher.LaunchFolderAsync(await StorageFolder.GetFolderFromPathAsync(AppPaths.HandsDirectory));
    }

    // --- Zasoby: pakiety ikon stanu systemu --------------------------------

    private static bool IsStatusKind(string? kind) => kind is
        "status-alarm" or "status-dnd" or "status-bluetooth" or "status-lock";

    private void StatusList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized) return;
        _selectedStatusPackId = (StatusList.SelectedItem as StatusPackItem)?.Id;
        RemoveStatusButton.IsEnabled =
            StatusList.SelectedItem is StatusPackItem { Id: not "Classic" };
    }

    private async void AddStatusButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker { SuggestedStartLocation = PickerLocationId.PicturesLibrary };
        picker.FileTypeFilter.Add("*");
        InitializeWithWindow.Initialize(picker, WindowHandle);
        var folder = await picker.PickSingleFolderAsync();
        if (folder is null) return;

        var added = RustNative.StatusPacksAdd(folder.Path, out var error);
        if (added is null)
        {
            StatusStatusText.Text = error is null ? L("AssetsStatusAddFailed") : LocalizationService.Error(error);
            return;
        }
        _selectedStatusPackId = added.Id;
        LoadStatusPacks();
        LoadWeatherPacks();
        StatusStatusText.Text = LF("AssetsStatusAdded", added.Id);
    }

    private void RemoveStatusButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedStatusPackId is null) return;
        RustNative.StatusPacksRemove(_selectedStatusPackId, out var error);
        if (error is not null)
        {
            StatusStatusText.Text = LocalizationService.Error(error);
            return;
        }
        _selectedStatusPackId = null;
        LoadStatusPacks();
        LoadWeatherPacks();
        StatusStatusText.Text = L("AssetsStatusRemoved");
    }

    private async void OpenStatusFolderButton_Click(object sender, RoutedEventArgs e)
    {
        await Launcher.LaunchFolderAsync(
            await StorageFolder.GetFolderFromPathAsync(AppPaths.StatusDirectory));
    }

    private async void StatusPackComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingStatusPacks || !_initialized || _projectFile is null) return;
        if ((sender as ComboBox)?.SelectedItem is not StatusPackDto pack) return;
        var layer = _layers.FirstOrDefault(l => IsStatusKind(l.Kind) && l.Id == _selectedLayerId);
        if (layer is null) return;

        var project = _projectFile;
        var widgetId = layer.Id;
        string? raw = null;
        string? error = null;
        StatusStatusText.Text = L("StatusPreparingAssets");
        await Task.Run(() => raw = RustNative.SetStatusPack(project, widgetId, pack.Id, out error))
            .ConfigureAwait(true);
        if (raw is null)
        {
            StatusStatusText.Text = error is null ? L("StatusApplyFailed") : LocalizationService.Error(error);
            EditorStatusText.Text = StatusStatusText.Text;
            return;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        StatusStatusText.Text = LF("StatusApplied", pack.Id);
    }

    private void WeatherPackComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingWeatherPacks || !_initialized || _projectFile is null) return;
        if ((sender as ComboBox)?.SelectedItem is not WeatherPackDto pack) return;
        var layer = _layers.FirstOrDefault(IsWeatherIcon);
        if (layer is null) return;

        // 1) Natychmiast: tylko widoczna ikona z magazynu. Żadnego odczytu 29
        // plików ani zapisu projektu na wątku UI — ramka zmienia się w tej klatce.
        layer.WeatherIconSet = pack.Id;
        foreach (var view in _frames.Values.Where(v => IsWeatherIcon(v.Layer)))
            view.Layer.WeatherIconSet = pack.Id;
        RefreshWeatherIconPreview(pack.Id);
        ImagesStatusText.Text = LF("StatusApplied", pack.Id);

        // 2) Reszta paczki do projektu w tle. Szybkie przełączania anulują
        // oczekujące zadanie; wynik starszy niż ostatni wybór jest odrzucany.
        var cts = new CancellationTokenSource();
        _weatherPackCts?.Cancel();
        _weatherPackCts = cts;
        var token = cts.Token;
        var version = ++_weatherPackVersion;
        var project = _projectFile;
        var widgetId = layer.Id;
        _weatherPackJob = Task.Run(async () =>
        {
            try
            {
                await Task.Delay(120, token).ConfigureAwait(false);
                await _assetGate.WaitAsync(token).ConfigureAwait(false);
                try
                {
                    token.ThrowIfCancellationRequested();
                    if (version != _weatherPackVersion) return;
                    var raw = RustNative.SetWeatherPack(project, widgetId, pack.Id, out var error);
                    if (token.IsCancellationRequested || version != _weatherPackVersion) return;
                    // Wyniki z wątka tła trafiają do UI tylko przez dispatcher
                    // i tylko gdy nadal są aktualne.
                    DispatcherQueue.TryEnqueue(() =>
                    {
                        if (token.IsCancellationRequested || version != _weatherPackVersion
                            || !ReferenceEquals(project, _projectFile)) return;
                        if (raw is null)
                        {
                            ImagesStatusText.Text = error is null
                                ? L("StatusApplyFailed") : LocalizationService.Error(error);
                            return;
                        }
                        using var document = JsonDocument.Parse(raw);
                        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
                    });
                }
                finally
                {
                    _assetGate.Release();
                }
            }
            catch (OperationCanceledException) { }
        }, token);
    }

    private async void HandsComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loadingHands || !_initialized || _projectFile is null) return;
        if ((sender as ComboBox)?.SelectedItem is not ChoiceItem item) return;
        var layer = _layers.FirstOrDefault(l => l.Kind == "analog-time" && l.Id == _selectedLayerId);
        if (layer is null) return;

        var newSet = _handSets.FirstOrDefault(set => set.Id == item.Value);
        var showHour = HourHandCheckBox.IsChecked == true;
        var showMinute = MinuteHandCheckBox.IsChecked == true;
        var showSecond = newSet is { HasSecond: true } && SecondHandCheckBox.IsChecked == true;
        var showCap = newSet is { HasCap: true } && CapHandCheckBox.IsChecked == true;
        var project = _projectFile;
        var widgetId = layer.Id;
        var name = item.Name;
        string? raw = null;
        string? error = null;
        HandsStatusText.Text = L("StatusPreparingAssets");
        await Task.Run(() => raw = RustNative.SetHands(project, widgetId, item.Value, out error))
            .ConfigureAwait(true);
        if (raw is null)
        {
            HandsStatusText.Text = error is null ? L("HandsApplyFailed") : LocalizationService.Error(error);
            EditorStatusText.Text = HandsStatusText.Text;
            return;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        string? visibilityError = null;
        await Task.Run(() => raw = RustNative.SetHandVisibility(
            project, widgetId, showHour, showMinute, showSecond, showCap, out visibilityError))
            .ConfigureAwait(true);
        if (raw is not null)
        {
            using var visibilityDocument = JsonDocument.Parse(raw);
            ApplyLayersPayload(visibilityDocument.RootElement.GetProperty("layers").GetRawText());
            LoadWidgetSettings();
            HandsStatusText.Text = LF("HandsApplied", name);
        }
        else
        {
            HandsStatusText.Text = visibilityError is null
                ? L("HandVisibilityFailed") : LocalizationService.Error(visibilityError);
        }
    }

    private void OptionalHand_Click(object sender, RoutedEventArgs e)
    {
        if (_loadingHands || !_initialized || _projectFile is null) return;
        var layer = _layers.FirstOrDefault(l => l.Kind == "analog-time" && l.Id == _selectedLayerId);
        if (layer is null) return;
        _hourHandEnabled = HourHandCheckBox.IsChecked == true;
        _minuteHandEnabled = MinuteHandCheckBox.IsChecked == true;
        _secondHandEnabled = SecondHandCheckBox.IsChecked == true;
        _capHandEnabled = CapHandCheckBox.IsChecked == true;
        ApplyHandVisibility(layer.Id, _hourHandEnabled, _minuteHandEnabled,
            _secondHandEnabled, _capHandEnabled);
    }

    /// <summary>Zapisuje widoczność wskazówek i odświeża podgląd.</summary>
    private bool ApplyHandVisibility(
        string widgetId, bool hour, bool minute, bool second, bool cap)
    {
        if (_projectFile is null) return false;
        var raw = RustNative.SetHandVisibility(
            _projectFile, widgetId, hour, minute, second, cap, out var error);
        if (raw is null)
        {
            var message = error is null ? L("HandVisibilityFailed") : LocalizationService.Error(error);
            HandsStatusText.Text = message;
            EditorStatusText.Text = message;
            LoadWidgetSettings(); // przywracamy stan zgodny z projektem
            return false;
        }
        using var document = JsonDocument.Parse(raw);
        ApplyLayersPayload(document.RootElement.GetProperty("layers").GetRawText());
        LoadWidgetSettings();
        HandsStatusText.Text = "";
        return true;
    }

    // --- Nawigacja ----------------------------------------------------------

    private void NavView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (!_initialized || args.SelectedItem is not NavigationViewItem item) return;
        var tag = item.Tag?.ToString();
        if (tag != "none") SetFooterHighlight(null);
        Navigate(tag);
    }

    /// <summary>Kliknięcie elementów stopki (PaneFooter) — nie zmieniają SelectedItem.</summary>
    private void FooterItem_Tapped(object sender, TappedRoutedEventArgs e)
    {
        if (!_initialized || sender is not NavigationViewItem item) return;
        SetFooterHighlight(item);

        // Przełączamy zaznaczenie na niewidoczną kartę — NavigationView sam zdejmie
        // wskaźnik z Home (przypisanie null zostawiało go widocznym w WinUI 3).
        NavView.SelectedItem = BlankNavItem;

        Navigate(item.Tag?.ToString());
    }

    /// <summary>
    /// Ustawia natywne zaznaczenie karty w stopce. IsSelected włącza te same stany wizualne
    /// NavigationViewItem, których NavigationView używa dla kart menu (tło, pasek akcentu,
    /// pogrubiony tekst) — bez ręcznego rysowania wskaźników.
    /// </summary>
    private void SetFooterHighlight(NavigationViewItem? selected)
    {
        AppSettingsNavItem.IsSelected = ReferenceEquals(selected, AppSettingsNavItem);
        ProjectSettingsNavItem.IsSelected = ReferenceEquals(selected, ProjectSettingsNavItem);
    }

    private void Navigate(string? tag)
    {
        switch (tag)
        {
            case "home":
                ShowView(editor: false);
                break;
            case "editor":
                ShowView(editor: true);
                break;
            case "widgets":
                ShowView(editor: false, widgets: true);
                break;
            case "assets":
                // Najpierw widok z zapamiętanymi miniaturkami, dopiero potem odświeżanie w tle.
                ShowView(editor: false, assets: true);
                _ = RefreshAssetsAsync();
                break;
            case "project-settings":
                LoadProjectSettings();
                ShowView(editor: false, projectSettings: true);
                break;
            case "settings":
                ShowView(editor: false, settings: true);
                break;
        }
    }

    private void LanguageCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || e.AddedItems.FirstOrDefault() is not StringCatalog catalog) return;
        LocalizationService.SetLanguage(catalog.Code);
        ApplyLocalization();
    }

    private void DefaultAuthorBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_initialized) return;
        // Puste pole = powrót do domyślnej nazwy użytkownika Windows.
        AppSettingsStore.SetDefaultAuthor(DefaultAuthorBox.Text.Trim());
    }

    /// <summary>Rysuje kratkę 16 px nad podglądem — co 4 linie są mocniejsze.</summary>
    private void BuildGridLines()
    {
        GridLinesCanvas.Children.Clear();
        for (var p = GridStep; p < ScreenSize; p += GridStep)
        {
            var major = p % (GridStep * 4) == 0;
            var brush = new SolidColorBrush(major
                ? Color.FromArgb(90, 160, 160, 200)
                : Color.FromArgb(45, 160, 160, 200));
            GridLinesCanvas.Children.Add(new Microsoft.UI.Xaml.Shapes.Line
            {
                X1 = p, Y1 = 0, X2 = p, Y2 = ScreenSize, Stroke = brush, StrokeThickness = 1,
            });
            GridLinesCanvas.Children.Add(new Microsoft.UI.Xaml.Shapes.Line
            {
                X1 = 0, Y1 = p, X2 = ScreenSize, Y2 = p, Stroke = brush, StrokeThickness = 1,
            });
        }
    }

    /// <summary>Opcje podglądu z flyoutu — teksty ustawiamy z katalogu (flyout nie dziedziczy DataContext).</summary>
    private void RefreshPreviewOptionLabels()
    {
        PreviewGridToggle.Text = L("PreviewGrid");
        UnselectedOutlineToggle.Text = L("UnselectedWidgetsOutline");
        LivePreviewToggle.Text = L("LivePreview");
        CustomDataEditItem.Text = L("CustomDataEdit");
    }

    /// <summary>Siatka podglądu włącza też przyciąganie do niej — jak dawny checkbox.</summary>
    private void PreviewGridToggle_Click(object sender, RoutedEventArgs e)
    {
        var enabled = PreviewGridToggle.IsChecked;
        GridLinesCanvas.Visibility = enabled ? Visibility.Visible : Visibility.Collapsed;
        RefreshToggleVisuals(PreviewGridToggle);
        if (!_initialized) return;
        AppSettingsStore.SetSnapToGrid(enabled);
    }

    /// <summary>Pokazuje lub chowa delikatny obrys niezaznaczonych widgetów.</summary>
    private void UnselectedOutlineToggle_Click(object sender, RoutedEventArgs e)
    {
        if (!_initialized) return;
        RefreshToggleVisuals(UnselectedOutlineToggle);
        AppSettingsStore.SetShowUnselectedOutline(UnselectedOutlineToggle.IsChecked);
        UpdateFrameVisuals();
    }

    /// <summary>
    /// Live Preview włącza wyłącznie realny czas i datę. Pozostałe pola dalej
    /// czytają dane predefiniowane — przełącznik nie kasuje tych wartości.
    /// </summary>
    private void LivePreviewToggle_Click(object sender, RoutedEventArgs e)
    {
        if (!_initialized) return;
        RefreshToggleVisuals(LivePreviewToggle);
        AppSettingsStore.SetLiveTimeDateEnabled(LivePreviewToggle.IsChecked);
        UpdateClockText();
    }

    /// <summary>Własny template flyoutu nie przełącza sam znacznika ✓ po kliknięciu.</summary>
    private static void RefreshToggleVisuals(ToggleMenuFlyoutItem item)
        => VisualStateManager.GoToState(item, item.IsChecked ? "Checked" : "Unchecked", false);

    /// <summary>Zębatka otwiera panel tylko dla typów danych obecnych na tarczy.</summary>
    private async void CustomDataEdit_Click(object sender, RoutedEventArgs e)
    {
        if (!_initialized || _projectFile is null) return;
        // Pozycja z zębatką nie jest przełącznikiem — nie zostawiamy znacznika ✓.
        CustomDataEditItem.IsChecked = false;
        RefreshToggleVisuals(CustomDataEditItem);
        PreviewOptionsFlyout.Hide();
        BuildCustomDataDialog();
        CustomDataDialog.XamlRoot = Content.XamlRoot ?? NavView.XamlRoot;
        try
        {
            await CustomDataDialog.ShowAsync();
        }
        catch
        {
            ShowStatus(L("ErrorCustomData"), InfoBarSeverity.Error);
        }
    }

    private void ResetCustomData_Click(object sender, RoutedEventArgs e)
    {
        AppSettingsStore.ClearCustomPreviewValues();
        BuildCustomDataDialog();
        UpdateClockText();
        CustomDataStatusText.Text = L("CustomDataCleared");
        CustomDataStatusText.Visibility = Visibility.Visible;
    }

    /// <summary>Oddzielny klucz magazynu dla każdego typu danych (bez duskich formatów wyświetlania).</summary>
    private static string? CustomValueKeyForKind(string kind) => kind switch
    {
        "steps" => "steps",
        "distance" => "distance",
        "heart" => "heart",
        "stress" => "stress",
        "pai" => "pai",
        "skin-temp" => "skinTemp",
        "sleep" => "sleep",
        "calories" => "calories",
        "spo2" => "spo2",
        "battery" => "battery",
        "weather-current" => "current",
        "lunar-date" => "lunarDate",
        "festival" => "festival",
        "world-clock" => "worldClock",
        _ => null,
    };

    private static string CustomLayerLabel(string kind) => kind switch
    {
        "steps" => L("LayerSteps"),
        "distance" => L("LayerDistance"),
        "heart" => L("LayerHeart"),
        "stress" => L("LayerStress"),
        "pai" => L("LayerPai"),
        "skin-temp" => L("LayerSkinTemp"),
        "sleep" => L("LayerSleep"),
        "calories" => L("LayerCalories"),
        "spo2" => L("LayerSpo2"),
        "battery" => L("LayerBattery"),
        "fat-burning" => L("LayerFatBurning"),
        "lunar-date" => L("LayerLunarDate"),
        "festival" => L("LayerFestival"),
        "world-clock" => L("LayerWorldClock"),
        "weather-current" => L("LayerWeatherCurrent"),
        _ => kind,
    };

    /// <summary>Realistyczna domyślna wartość źródła progressu (placeholder w Custom Data).</summary>
    private static string ProgressSourceDefaultText(string source) => source switch
    {
        "steps" => "16000",
        "distance" => "12.3",
        "heart" => "72",
        "calories" => "430",
        "stand" => "8",
        "fat-burning" => "32",
        "pai" => "42",
        "spo2" => "97",
        "sleep" => "425",
        "fixed" => "50",
        _ => "73",
    };

    /// <summary>Buduje pola na nowo przy każdym otwarciu — lista widgetów mogła się zmienić.</summary>
    private void BuildCustomDataDialog()
    {
        CustomDataFieldsPanel.Children.Clear();
        CustomDataStatusText.Visibility = Visibility.Collapsed;

        var kinds = _layers
            .Select(layer => layer.Kind)
            .Distinct(StringComparer.Ordinal)
            .ToList();
        var hasTime = kinds.Contains("time") || kinds.Contains("analog-time");
        var hasDate = kinds.Contains("date") || kinds.Any(IsDateFieldKind);
        var hasWeather = kinds.Any(IsWeatherKind);
        var sensorKinds = kinds.Where(kind => CustomValueKeyForKind(kind) is not null).ToList();
        // Widgety postępu nie mają własnego kindu w CustomValueKeyForKind —
        // ich pole wynika z wybranego źródła (np. bateria = 73%).
        var dataFields = new Dictionary<string, (string Label, string Fallback)>();
        foreach (var kind in sensorKinds)
        {
            var key = CustomValueKeyForKind(kind)!;
            dataFields[key] = (CustomLayerLabel(kind), _layers.FirstOrDefault(l => l.Kind == kind)?.Text ?? "");
        }
        foreach (var layer in _layers.Where(layer => IsProgressKind(layer.Kind)))
        {
            var source = layer.Progress?.Source;
            if (string.IsNullOrWhiteSpace(source) || source == "fixed" || dataFields.ContainsKey(source)) continue;
            dataFields[source] = (CustomLayerLabel(source), ProgressSourceDefaultText(source));
        }

        if (!hasTime && !hasDate && !hasWeather && dataFields.Count == 0)
        {
            CustomDataFieldsPanel.Children.Add(new TextBlock
            {
                Text = L("CustomDataEmpty"),
                FontSize = 12,
                TextWrapping = TextWrapping.Wrap,
                Foreground = new SolidColorBrush(Color.FromArgb(255, 150, 160, 180)),
            });
            return;
        }

        // Placeholdery muszą pokazywać ten sam moment co podgląd, gdy Live
        // Preview jest wyłączone — nie bieżący czas systemowy.
        var def = DefaultPreviewMoment();
        if (hasTime)
        {
            AddCustomGroup("CustomDataGroupTime", body =>
            {
                AddCustomNumber(body, "CustomDataHour", "hour", def.Hour, 0, 23);
                AddCustomNumber(body, "CustomDataMinute", "minute", def.Minute, 0, 59);
                AddCustomNumber(body, "CustomDataSecond", "second", def.Second, 0, 59);
            });
        }
        if (hasDate)
        {
            AddCustomGroup("CustomDataGroupDate", body =>
            {
                AddCustomNumber(body, "CustomDataDay", "day", def.Day, 1, 31);
                AddCustomNumber(body, "CustomDataMonth", "month", def.Month, 1, 12);
                AddCustomNumber(body, "CustomDataYear", "year", def.Year, 1, 9999);
                if (kinds.Contains("weekday-number"))
                    AddCustomFirstDay(body);
            });
        }
        if (dataFields.Count > 0)
        {
            AddCustomGroup("CustomDataGroupValues", body =>
            {
                foreach (var field in dataFields)
                    AddCustomText(body, field.Value.Label, field.Key, field.Value.Fallback);
            });
        }
        if (hasWeather)
        {
            // Puste pole = domyślna, pochmurna Warszawa (wartości z Rusta).
            var cityFallback = EffectiveWatchfaceLanguage() == "pl" ? "Warszawa" : "Warsaw";
            AddCustomGroup("CustomDataGroupWeather", body =>
            {
                AddCustomText(body, L("LayerWeatherCity"), "city", cityFallback);
                // Warunek wybieramy po nazwie — kody 0…28 pochodzą z tabeli Rusta.
                var conditions = new List<ChoiceItem>
                {
                    new() { Value = "", Name = L("CustomDataDefault") },
                };
                conditions.AddRange(RustNative.WeatherConditions(EffectiveWatchfaceLanguage(), out _));
                AddCustomChoice(body, L("LayerWeatherCondition"), "index", conditions);
                AddCustomText(body, L("LayerWeatherHigh"), "high", "22");
                AddCustomText(body, L("LayerWeatherLow"), "low", "16");
                // Jedno pole HH:MM zamiast osobnych godzin i minut.
                AddCustomTime(body, L("LayerSunrise"), "sunrise", "05:13",
                    "sunriseHour", "sunriseMinute");
                AddCustomTime(body, L("LayerSunset"), "sunset", "20:40",
                    "sunsetHour", "sunsetMinute");
            });
        }
    }

    private void AddCustomGroup(string labelKey, Action<StackPanel> build)
    {
        var group = new StackPanel { Spacing = 8 };
        group.Children.Add(new TextBlock
        {
            Text = L(labelKey),
            FontSize = 13,
            FontWeight = FontWeights.SemiBold,
        });
        var body = new StackPanel { Spacing = 6, Margin = new Thickness(12, 0, 0, 0) };
        build(body);
        group.Children.Add(body);
        CustomDataFieldsPanel.Children.Add(group);
    }

    private void AddCustomNumber(StackPanel parent, string labelKey, string key, int liveValue,
        int min, int max)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        row.Children.Add(new TextBlock
        {
            Text = L(labelKey),
            Width = 110,
            FontSize = 12,
            VerticalAlignment = VerticalAlignment.Center,
        });
        var box = new TextBox
        {
            Width = 160,
            FontSize = 12,
            Text = AppSettingsStore.GetCustomPreviewValue(key) ?? "",
            PlaceholderText = liveValue.ToString(CultureInfo.InvariantCulture),
        };
        box.TextChanged += (_, _) =>
        {
            var value = int.TryParse(box.Text, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
                ? Math.Clamp(parsed, min, max).ToString(CultureInfo.InvariantCulture)
                : box.Text;
            AppSettingsStore.SetCustomPreviewValue(key, value);
            UpdateClockText();
        };
        row.Children.Add(box);
        parent.Children.Add(row);
    }

    private void AddCustomText(StackPanel parent, string label, string key, string fallback)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        row.Children.Add(new TextBlock
        {
            Text = label,
            Width = 110,
            FontSize = 12,
            VerticalAlignment = VerticalAlignment.Center,
        });
        var box = new TextBox
        {
            Width = 200,
            FontSize = 12,
            Text = AppSettingsStore.GetCustomPreviewValue(key) ?? "",
            PlaceholderText = string.IsNullOrWhiteSpace(fallback) ? L("CustomDataValue") : fallback,
        };
        box.TextChanged += (_, _) =>
        {
            AppSettingsStore.SetCustomPreviewValue(key, box.Text);
            UpdateClockText();
        };
        row.Children.Add(box);
        parent.Children.Add(row);
    }

    /// <summary>Lista warunków pogody: etykieta czytelna dla człowieka, kod 0…28 w tle.</summary>
    private void AddCustomChoice(StackPanel parent, string label, string key,
        IReadOnlyList<ChoiceItem> items)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        row.Children.Add(new TextBlock
        {
            Text = label,
            Width = 110,
            FontSize = 12,
            VerticalAlignment = VerticalAlignment.Center,
        });
        var stored = AppSettingsStore.GetCustomPreviewValue(key);
        var combo = new ComboBox
        {
            Width = 200,
            FontSize = 12,
            // Bez DisplayMemberPath WinUI pokazuje `RustyWatch.ChoiceItem` zamiast nazwy.
            DisplayMemberPath = nameof(ChoiceItem.Name),
            ItemsSource = items,
            SelectedItem = items.FirstOrDefault(item => item.Value == (stored ?? "0")),
        };
        combo.SelectionChanged += (_, _) =>
        {
            // Pusty wybór usuwa nadpisanie i wraca do domyślnej pogody.
            AppSettingsStore.SetCustomPreviewValue(key, (combo.SelectedItem as ChoiceItem)?.Value);
            UpdateClockText();
        };
        row.Children.Add(combo);
        parent.Children.Add(row);
    }

    /// <summary>Jedno pole czasu HH:MM. Starsze pary godzina/minuta są wczytywane i czyszczone.</summary>
    private void AddCustomTime(StackPanel parent, string label, string key, string fallback,
        string legacyHourKey, string legacyMinuteKey)
    {
        var stored = AppSettingsStore.GetCustomPreviewValue(key);
        if (stored is null && (AppSettingsStore.GetCustomPreviewValue(legacyHourKey) is not null
            || AppSettingsStore.GetCustomPreviewValue(legacyMinuteKey) is not null))
        {
            var hour = ClampCustomInt(legacyHourKey, 0, 0, 23);
            var minute = ClampCustomInt(legacyMinuteKey, 0, 0, 59);
            stored = $"{hour:00}:{minute:00}";
        }
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        row.Children.Add(new TextBlock
        {
            Text = label,
            Width = 110,
            FontSize = 12,
            VerticalAlignment = VerticalAlignment.Center,
        });
        var box = new TextBox
        {
            Width = 200,
            FontSize = 12,
            Text = stored ?? "",
            PlaceholderText = fallback,
        };
        box.TextChanged += (_, _) =>
        {
            var text = box.Text.Trim();
            if (TryParseCustomTime(text, out var hour, out var minute))
            {
                AppSettingsStore.SetCustomPreviewValue(key, $"{hour:00}:{minute:00}");
                AppSettingsStore.SetCustomPreviewValue(legacyHourKey, null);
                AppSettingsStore.SetCustomPreviewValue(legacyMinuteKey, null);
            }
            else
            {
                // Nieprawidłowy wpis Rust zignoruje — podgląd wraca do wartości domyślnej.
                AppSettingsStore.SetCustomPreviewValue(key, text);
            }
            UpdateClockText();
        };
        row.Children.Add(box);
        parent.Children.Add(row);
    }

    private static bool TryParseCustomTime(string text, out int hour, out int minute)
    {
        hour = minute = 0;
        var parts = text.Split(':');
        return parts.Length == 2
            && int.TryParse(parts[0].Trim(), NumberStyles.None, CultureInfo.InvariantCulture, out hour)
            && int.TryParse(parts[1].Trim(), NumberStyles.None, CultureInfo.InvariantCulture, out minute)
            && hour is >= 0 and <= 23 && minute is >= 0 and <= 59;
    }

    private void AddCustomFirstDay(StackPanel parent)
    {
        var items = new List<ChoiceItem>
        {
            new() { Value = "monday", Name = L("FirstDayMonday") },
            new() { Value = "sunday", Name = L("FirstDaySunday") },
        };
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        row.Children.Add(new TextBlock
        {
            Text = L("FirstDayOfWeekLabel"),
            Width = 110,
            FontSize = 12,
            VerticalAlignment = VerticalAlignment.Center,
        });
        var combo = new ComboBox
        {
            Width = 200,
            FontSize = 12,
            DisplayMemberPath = nameof(ChoiceItem.Name),
            ItemsSource = items,
            SelectedItem = items.FirstOrDefault(item =>
                item.Value == (AppSettingsStore.GetCustomPreviewValue("firstDay") ?? AppSettingsStore.FirstDayOfWeek)),
        };
        combo.SelectionChanged += (_, _) =>
        {
            var value = (combo.SelectedItem as ChoiceItem)?.Value;
            if (string.IsNullOrEmpty(value)) AppSettingsStore.SetCustomPreviewValue("firstDay", null);
            else AppSettingsStore.SetCustomPreviewValue("firstDay", value);
            UpdateClockText();
        };
        row.Children.Add(combo);
        parent.Children.Add(row);
    }

    /// <summary>Autor wpisywany przy tworzeniu tarczy, gdy nie ustawiono własnego.</summary>
    private static string EffectiveDefaultAuthor()
    {
        var configured = AppSettingsStore.DefaultAuthor;
        return string.IsNullOrWhiteSpace(configured) ? Environment.UserName : configured;
    }

    /// <summary>Odświeża wszystkie teksty po zmianie języka bez restartu aplikacji.</summary>
    private void ApplyLocalization()
    {
        _localizationBridge.Refresh();
        LocalizationService.Current.Refresh();
        Title = L("AppTitle");
        StatusInfoBar.IsOpen = false;

        if (!ReferenceEquals(LanguageCombo.SelectedItem, LocalizationService.Current))
            LanguageCombo.SelectedItem = LocalizationService.Current;
        DefaultAuthorBox.PlaceholderText = LF("DefaultAuthorPlaceholder", Environment.UserName);
        RefreshPreviewOptionLabels();

        ProjectNameText.Text = _projectFile is null ? L("NavHome") : _projectName;
        if (_projectSettingsError is not null)
            ProjectSettingsErrorText.Text = LocalizationService.Error(_projectSettingsError);
        RefreshLayersList();
        LoadTextFitOptions();
        LoadWatchfaceLanguageOptions();
        LoadMeasurementSystemOptions();
        LoadFirstDayOfWeekOptions();
        LoadDistanceUnitOptions();
        LoadWeatherDayOptions();
        LoadSleepStageOptions();
        LoadFonts();
        LoadImages();
        LoadHandSets(); LoadStatusPacks(); LoadWeatherPacks();
        LoadWidgetCatalog();
        var selected = _layers.FirstOrDefault(l => l.Id == _selectedLayerId);
        InspectorTitle.Text = selected is null
            ? ""
            : LF("WidgetSettingsFor", selected.Name);
        RefreshRecent();
        if (_settings is not null)
        {
            EditorStatusText.Text = L("AutoSave");
            PreviewPatternText.Text = LF("FormatLabel", _settings.Pattern);
        }
    }

    // --- Projekty -----------------------------------------------------------

    private async void NewProject_Click(object sender, RoutedEventArgs e)
    {
        if (!await ShowProjectDialogAsync(saveAs: false)) return;

        var name = NewNameBox.Text.Trim();
        var author = NewAuthorBox.Text.Trim();
        var appId = uint.Parse(NewAppIdBox.Text.Trim());
        var projectsRoot = EnsureProjectsRoot();
        if (projectsRoot is null) return;

        var data = RustNative.CreateProjectIn(projectsRoot, name, author, appId, out var error);
        if (data is null)
        {
            ShowStatus(error is null ? L("ErrorCreateProject") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(data);
        var root = document.RootElement;
        var file = root.GetProperty("file").GetString()!;
        var widgets = root.GetProperty("widgets").GetInt32();

        // Nowa tarcza domyślnie używa dołączonej czcionki Inter (OFL) — plik trafia
        // też do `assets/fonts` projektu, żeby eksport był samowystarczalny.
        var applied = RustNative.ApplyDefaultFont(file, out var fontError);
        if (applied is null)
        {
            ShowStatus(fontError is null ? L("AssetsFontAddFailed") : LocalizationService.Error(fontError),
                InfoBarSeverity.Error);
            return;
        }

        if (OpenProjectFile(file))
            ShowStatus(LF("CreatedProject", name, new DirectoryInfo(projectsRoot).Name, widgets));
    }

    private void Save_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null) return;
        var json = RustNative.LoadProject(_projectFile, out var error);
        if (json is null)
        {
            ShowStatus(error is null ? L("ErrorLoadProject") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        var saved = RustNative.SaveProject(_projectFile, json, out error);
        ShowStatus(
            saved is null
                ? error is null ? L("ErrorSaveProject") : LocalizationService.Error(error)
                : LF("SavedProject", _projectName),
            saved is null ? InfoBarSeverity.Error : InfoBarSeverity.Success);
    }

    private async void SaveAs_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null) return;
        if (!await ShowProjectDialogAsync(saveAs: true)) return;

        var name = NewNameBox.Text.Trim();
        var author = NewAuthorBox.Text.Trim();
        var appId = uint.Parse(NewAppIdBox.Text.Trim());
        var projectsRoot = EnsureProjectsRoot();
        if (projectsRoot is null) return;

        var data = RustNative.SaveProjectAs(_projectFile, projectsRoot, name, author, appId, out var error);
        if (data is null)
        {
            ShowStatus(error is null ? L("ErrorSaveAs") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(data);
        var file = document.RootElement.GetProperty("file").GetString()!;
        if (OpenProjectFile(file)) ShowStatus(LF("SavedAs", name));
    }

    /// <summary>Wczytuje dane projektu do strony ustawień. Nazwa jest tylko wyświetlana.</summary>
    private void LoadProjectSettings()
    {
        if (_projectFile is null) return;
        var json = RustNative.LoadProject(_projectFile, out var error);
        if (json is null)
        {
            ShowStatus(error is null ? L("ErrorLoadProject") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        using var document = JsonDocument.Parse(json);
        var metadata = document.RootElement.GetProperty("metadata");
        ProjectNameBox.Text = metadata.GetProperty("name").GetString() ?? "";
        ProjectAuthorBox.Text = metadata.GetProperty("author").GetString() ?? "";
        ProjectVersionBox.Text = metadata.GetProperty("version").GetString() ?? "";
        ClearProjectSettingsError();
    }

    private void ClearProjectSettingsError()
    {
        _projectSettingsError = null;
        ProjectSettingsErrorText.Text = "";
        ProjectSettingsErrorText.Visibility = Visibility.Collapsed;
    }

    private void SaveProjectSettings_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null) return;
        var author = ProjectAuthorBox.Text.Trim();
        var version = ProjectVersionBox.Text.Trim();

        var data = RustNative.SetProjectMetadata(_projectFile, author, version, out var error);
        if (data is null)
        {
            _projectSettingsError = error ?? L("ErrorProjectSettings");
            ProjectSettingsErrorText.Text = LocalizationService.Error(_projectSettingsError);
            ProjectSettingsErrorText.Visibility = Visibility.Visible;
            ShowStatus(ProjectSettingsErrorText.Text, InfoBarSeverity.Error);
            return;
        }

        ClearProjectSettingsError();
        using var document = JsonDocument.Parse(data);
        var root = document.RootElement;
        // Nazwa nie jest edytowalna — aktualizujemy pozostałe pola z potwierdzenia Rusta.
        ProjectAuthorBox.Text = root.GetProperty("author").GetString() ?? author;
        ProjectVersionBox.Text = root.GetProperty("version").GetString() ?? version;
        ShowStatus(LF("SavedProjectSettings", _projectName));
    }

    /// <summary>Katalog główny nowych projektów — z ustawień, tworzony jeśli nie istnieje.</summary>
    private string? EnsureProjectsRoot()
    {
        var root = AppSettingsStore.ProjectsDirectory;
        try
        {
            Directory.CreateDirectory(root);
            return root;
        }
        catch (Exception ex)
        {
            ShowStatus(LF("ErrorProjectsFolder", ex.Message), InfoBarSeverity.Error);
            return null;
        }
    }

    private void ProjectsFolderBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_initialized) return;
        var raw = ProjectsFolderBox.Text.Trim();
        if (raw.Length == 0)
        {
            AppSettingsStore.SetProjectsDirectory("");
            return;
        }
        try
        {
            AppSettingsStore.SetProjectsDirectory(raw);
        }
        catch (Exception ex)
        {
            ShowStatus(LF("ErrorProjectsFolder", ex.Message), InfoBarSeverity.Error);
        }
    }

    private async void BrowseProjectsFolder_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
        };
        InitializeWithWindow.Initialize(picker, WindowHandle);
        var folder = await picker.PickSingleFolderAsync();
        if (folder is null) return;
        ProjectsFolderBox.Text = folder.Path;
    }

    private async Task<bool> ShowProjectDialogAsync(bool saveAs)
    {
        _localizationBridge.SetSaveAs(saveAs);

        var current = saveAs ? CurrentMetadata() : null;
        NewNameBox.Text = current?.Name ?? L("PlaceholderName");
        NewAuthorBox.Text = current?.Author ?? EffectiveDefaultAuthor();
        NewAppIdBox.Text = current?.AppId.ToString() ?? "";
        if (!uint.TryParse(NewAppIdBox.Text.Trim(), out _)) SetRandomAppId();

        NewProjectErrorText.Visibility = Visibility.Collapsed;
        NewProjectDialog.XamlRoot = Content.XamlRoot ?? NavView.XamlRoot;
        try
        {
            return await NewProjectDialog.ShowAsync() == ContentDialogResult.Primary;
        }
        catch (Exception ex)
        {
            ShowStatus(LF("ErrorOpenDialog", ex.Message), InfoBarSeverity.Error);
            return false;
        }
    }

    private (string Name, string Author, uint AppId)? CurrentMetadata()
    {
        if (_projectFile is null) return null;
        var json = RustNative.LoadProject(_projectFile, out var error);
        if (json is null)
        {
            ShowStatus(error is null ? L("ErrorLoadProject") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return null;
        }

        using var document = JsonDocument.Parse(json);
        var metadata = document.RootElement.GetProperty("metadata");
        return (
            metadata.GetProperty("name").GetString() ?? "",
            metadata.GetProperty("author").GetString() ?? "",
            metadata.GetProperty("appId").GetUInt32());
    }

    private void SetRandomAppId()
    {
        var id = RustNative.RandomAppId(out var error);
        // Rust jest źródłem prawdy; fallback tylko gdy FFI niedostępne.
        NewAppIdBox.Text = (id > 0 ? id : (uint)Random.Shared.Next(10_000_000, 100_000_000)).ToString();
    }

    private void RandomizeAppId_Click(object sender, RoutedEventArgs e) => SetRandomAppId();

    private void NewProjectDialog_PrimaryButtonClick(
        ContentDialog sender,
        ContentDialogButtonClickEventArgs args)
    {
        var name = NewNameBox.Text.Trim();
        var author = NewAuthorBox.Text.Trim();
        var appIdText = NewAppIdBox.Text.Trim();

        // Puste pole autora w oknie projektu też wraca do ustawionego domyślnego.
        if (author.Length == 0)
        {
            author = EffectiveDefaultAuthor();
            NewAuthorBox.Text = author;
        }

        string? error = null;
        if (name.Length == 0)
            error = L("ValidationName");
        else if (author.Length == 0)
            error = L("ValidationAuthor");
        else if (!uint.TryParse(appIdText, out var appId) || appId == 0)
            error = L("ValidationAppId");

        if (error is null)
        {
            NewProjectErrorText.Visibility = Visibility.Collapsed;
            return;
        }

        NewProjectErrorText.Text = error;
        NewProjectErrorText.Visibility = Visibility.Visible;
        args.Cancel = true;
    }

    private async void OpenProject_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
        };
        InitializeWithWindow.Initialize(picker, WindowHandle);
        var folder = await picker.PickSingleFolderAsync();
        if (folder is null) return;

        var file = Path.Combine(folder.Path, "project.json");
        if (!File.Exists(file))
        {
            ShowStatus(LF("ErrorOpenFolder", folder.Name), InfoBarSeverity.Error);
            return;
        }

        if (OpenProjectFile(file)) ShowStatus(LF("OpenedProject", _projectName));
    }

    private void RecentList_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is RecentProjectView item && OpenProjectFile(item.File))
            ShowStatus(LF("OpenedProject", item.Name));
    }

    private void RecentList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        _selectedRecent = e.AddedItems.FirstOrDefault() as RecentProjectView;
    }

    private void RemoveRecent_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedRecent is null)
        {
            ShowStatus(L("SelectRecent"), InfoBarSeverity.Warning);
            return;
        }

        var name = _selectedRecent.Name;
        var data = RustNative.RecentRemove(AppPaths.RecentStorePath, _selectedRecent.File, out var error);
        if (data is null)
        {
            ShowStatus(error is null ? L("ErrorRemoveRecent") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        RefreshRecent();
        ShowStatus(LF("RemovedRecent", name));
    }

    private void ClearRecent_Click(object sender, RoutedEventArgs e)
    {
        var data = RustNative.RecentClear(AppPaths.RecentStorePath, out var error);
        if (data is null)
        {
            ShowStatus(error is null ? L("ErrorClearRecent") : LocalizationService.Error(error), InfoBarSeverity.Error);
            return;
        }

        RefreshRecent();
        ShowStatus(L("ClearedRecent"));
    }

    /// <summary>Generuje podglądy i dopiero wtedy otwiera okno eksportu.</summary>
    private void Export_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null)
        {
            ShowStatus(L("ExportFirst"), InfoBarSeverity.Warning);
            return;
        }
        _ = GeneratePreviewAndShowExportAsync();
    }

    private async Task GeneratePreviewAndShowExportAsync()
    {
        var project = _projectFile;
        if (project is null) return;

        ExportButton.IsEnabled = false;
        ExportProgressBar.Visibility = Visibility.Visible;
        ExportProgressBar.Value = 0;
        ExportProgressBar.IsIndeterminate = false;
        ToolTipService.SetToolTip(ExportButton, LF("ExportProgress", 0));
        try
        {
            // Eksport musi widzieć ostateczny rozmiar zasobów — poczekaj na
            // ostatnie zadanie skalowania z wątka tła.
            await WaitForAssetJobsAsync().ConfigureAwait(true);
            // Jedna losowa seria dla wszystkich klatek — dane nie zmieniają się w trakcie.
            var series = await Task.Run(() =>
            {
                var data = RustNative.PreviewRandomSeries(EffectiveWatchfaceLanguage(), out var error);
                return (data, error);
            });
            if (series.data is null)
                throw new InvalidOperationException(
                    series.error is null ? L("ErrorExport") : LocalizationService.Error(series.error));

            // Rust jest jedynym źródłem liczby klatek — C# nie ma własnej stałej.
            int frameCount;
            using (var seriesDocument = JsonDocument.Parse(series.data!))
            {
                frameCount = seriesDocument.RootElement.TryGetProperty("frameCount", out var count)
                    ? count.GetInt32()
                    : 7;
            }
            ExportProgressBar.Maximum = frameCount;
            for (var index = 0; index < frameCount; index++)
            {
                var frameIndex = (uint)index;
                var frame = await Task.Run(() =>
                {
                    var data = RustNative.RenderPreviewFrame(project, frameIndex, series.data!, out var error);
                    return (data, error);
                });
                if (frame.data is null)
                    throw new InvalidOperationException(
                        frame.error is null ? L("ErrorExport") : LocalizationService.Error(frame.error));
                ExportProgressBar.Value = index + 1;
                ToolTipService.SetToolTip(ExportButton, LF("ExportProgress", index + 1));
            }

            // Kodowanie GIF-a też trwa — pasek przechodzi w tryb ciągłego ładowania,
            // żeby nie wyglądał na skończony przed pojawieniem się pliku.
            ExportProgressBar.IsIndeterminate = true;
            var encoded = await Task.Run(() =>
            {
                var data = RustNative.EncodePreviewGif(project, series.data!, out var error);
                return (data, error);
            });
            ExportProgressBar.IsIndeterminate = false;
            if (encoded.data is null)
                throw new InvalidOperationException(
                    encoded.error is null ? L("ErrorExport") : LocalizationService.Error(encoded.error));

            using (var document = JsonDocument.Parse(encoded.data))
            {
                var gif = document.RootElement.GetProperty("gif").GetString() ?? "";
                var bitmap = new BitmapImage { CreateOptions = BitmapCreateOptions.IgnoreImageCache };
                bitmap.UriSource = new Uri(gif);
                ExportPreviewImage.Source = bitmap;
                // Border w XAML ma CornerRadius="120" — przycina animację do okręgu
                // Active 2 (Round), więc w oknie nie widać kwadratowych rogów.
            }

            LoadExportProjectInfo();
            await ExportDialog.ShowAsync();
        }
        catch (Exception ex)
        {
            ShowStatus(ex.Message, InfoBarSeverity.Error);
        }
        finally
        {
            ExportProgressBar.Visibility = Visibility.Collapsed;
            ExportProgressBar.IsIndeterminate = false;
            ExportProgressBar.Value = 0;
            ExportButton.IsEnabled = true;
            ToolTipService.SetToolTip(ExportButton, null);
        }
    }

    private void LoadExportProjectInfo()
    {
        ExportProjectName.Text = _projectName ?? "";
        ExportProjectAuthor.Text = "";
        ExportProjectVersion.Text = "";
        ExportProjectAppId.Text = "";
        if (_projectFile is null) return;
        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(_projectFile));
            var metadata = document.RootElement.GetProperty("metadata");
            if (metadata.TryGetProperty("author", out var author))
                ExportProjectAuthor.Text = author.GetString() ?? "";
            if (metadata.TryGetProperty("version", out var version))
                ExportProjectVersion.Text = version.GetString() ?? "";
            if (metadata.TryGetProperty("appId", out var appId))
                ExportProjectAppId.Text = appId.GetRawText();
        }
        catch
        {
            ShowStatus(L("ErrorLoadProject"), InfoBarSeverity.Warning);
        }
    }

    /// <summary>Zapisuje paczkę ZIP do katalogu projektu.</summary>
    private void DialogExportZip_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null) return;
        var zipPath = Path.Combine(Path.GetDirectoryName(_projectFile) ?? "",
            ExportFileName(_projectName) + ".zip");
        var data = RustNative.ExportZip(_projectFile, zipPath, out var error);
        if (data is null)
        {
            ShowStatus(error is null ? L("ErrorExport") : LocalizationService.Error(error),
                InfoBarSeverity.Error);
            return;
        }
        using var document = JsonDocument.Parse(data);
        ExportDialog.Hide();
        ShowStatus(LF("Exported", document.RootElement.GetProperty("zip").GetString() ?? zipPath));
    }

    /// <summary>
    /// Bezpieczna nazwa pliku z nazwy projektu: spacje i znaki niedozwolone stają się
    /// kropkami, a plik ląduje w folderze projektu (tam, gdzie project.json).
    /// </summary>
    private static string ExportFileName(string? projectName)
    {
        var builder = new StringBuilder();
        foreach (var c in (projectName ?? "").Trim())
        {
            if (char.IsWhiteSpace(c) || c is '<' or '>' or ':' or '"' or '/' or '\\' or '|' or '?' or '*')
                builder.Append('.');
            else if (!char.IsControl(c))
                builder.Append(c);
        }
        var name = builder.ToString().Trim('.', ' ');
        if (name.Length == 0) name = "Watchface";

        var baseName = name.Split('.')[0].ToUpperInvariant();
        var reserved = baseName is "CON" or "PRN" or "AUX" or "NUL"
            || (baseName.Length == 4 && (baseName.StartsWith("COM") || baseName.StartsWith("LPT"))
                && char.IsDigit(baseName[3]));
        if (reserved) name = "_" + name;
        return name;
    }


    /// <summary>Eksportuje tarczę do katalogu roboczego i instaluje ją w symulatorze ZeppOS.</summary>
    private async void TestEmulator_Click(object sender, RoutedEventArgs e)
    {
        if (_projectFile is null)
        {
            ShowStatus(L("ExportFirst"), InfoBarSeverity.Warning);
            return;
        }

        TestEmulatorButton.IsEnabled = false;
        try
        {
            // Symulator jest oprogramowaniem Zepp Health — nie dołączamy go do aplikacji.
            // Gdy go brak, otwieramy wyłącznie oficjalną stronę pobierania.
            var simulator = ZeppSimulator.FindExecutable();
            if (simulator is null)
            {
                await Launcher.LaunchUriAsync(new Uri(ZeppSimulator.DownloadPage));
                ShowStatus(L("EmulatorNotInstalled"), InfoBarSeverity.Warning);
                return;
            }
            if (!ZeppSimulator.IsDeviceImageReady())
            {
                ZeppSimulator.Launch(simulator);
                ShowStatus(L("EmulatorDeviceMissing"), InfoBarSeverity.Warning);
                return;
            }
            if (!ZeppSimulator.IsWatchWindowOpen())
            {
                ZeppSimulator.Launch(simulator);
                ShowStatus(L("EmulatorDeviceClosed"), InfoBarSeverity.Warning);
                return;
            }
            if (FindZeusCommand() is null)
            {
                ShowStatus(L("EmulatorMissingZeus"), InfoBarSeverity.Error);
                return;
            }

            ShowStatus(L("EmulatorPreparing"), InfoBarSeverity.Informational);
            var raw = RustNative.PrepareEmulatorWorkspace(
                _projectFile, AppPaths.EmulatorDirectory, out var error);
            if (raw is null)
            {
                ShowStatus(error is null ? L("ErrorExport") : LocalizationService.Error(error),
                    InfoBarSeverity.Error);
                return;
            }

            using var document = JsonDocument.Parse(raw);
            var data = document.RootElement;
            var appDir = data.GetProperty("appDir").GetString() ?? "";
            var logPath = data.GetProperty("log").GetString() ?? "";

            var zeus = FindZeusCommand()!;

            var start = new ProcessStartInfo
            {
                FileName = Environment.GetEnvironmentVariable("ComSpec") ?? "cmd.exe",
                Arguments = $"/c \"\"{zeus}\" install --path \"{appDir}\" --target \"Amazfit Active 2 NFC (Round)\"\"",
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8,
            };
            var output = new List<string>();
            var process = new Process { StartInfo = start };
            process.OutputDataReceived += (_, args) =>
            {
                if (args.Data is not null) output.Add(args.Data);
            };
            process.ErrorDataReceived += (_, args) =>
            {
                if (args.Data is not null) output.Add(args.Data);
            };
            _emulatorProcess = process;
            process.Start();
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();
            await process.WaitForExitAsync();
            _emulatorProcess = null;

            var logText = string.Join(Environment.NewLine, output);
            await File.WriteAllTextAsync(logPath, logText);
            // zeus potrafi zwrócić kod 0 mimo błędu w logu.
            var failed = process.ExitCode != 0
                         || logText.Contains("[ERROR]", StringComparison.OrdinalIgnoreCase);
            if (!failed)
                ShowStatus(LF("EmulatorStarted", logPath));
            else
                ShowStatus(LF("EmulatorFailed", process.ExitCode, logPath), InfoBarSeverity.Error);
        }
        catch (Exception ex)
        {
            ShowStatus(LF("ErrorEmulator", ex.Message), InfoBarSeverity.Error);
        }
        finally
        {
            TestEmulatorButton.IsEnabled = true;
        }
    }

    /// <summary>Znajduje polecenie zeus zainstalowane globalnie przez npm.</summary>
    private static string? FindZeusCommand()
    {
        var path = Environment.GetEnvironmentVariable("PATH") ?? "";
        foreach (var directory in path.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries))
        {
            foreach (var name in new[] { "zeus.cmd", "zeus.exe" })
            {
                var candidate = Path.Combine(directory.Trim('"'), name);
                if (File.Exists(candidate)) return candidate;
            }
        }
        return null;
    }

    private void MainWindow_Closed(object sender, WindowEventArgs args)
    {
        try { _emulatorProcess?.Kill(entireProcessTree: true); }
        catch { /* proces już zakończony */ }
    }

    private void Sync_Click(object sender, RoutedEventArgs e)
    {
        ShowStatus(L("SyncHint"), InfoBarSeverity.Informational);
    }
}
