using System.Runtime.InteropServices;
using System.Text.Json;

namespace RustyWatch;

/// <summary>Czcionka z magazynu w %APPDATA%\RustyWatch\Fonts.</summary>
internal sealed record FontAssetDto(string File, string Family, bool IsDefault);

internal sealed record FontListDto(List<FontAssetDto> Fonts);

/// <summary>Obrazek z magazynu w %APPDATA%\RustyWatch\Images.</summary>
internal sealed record ImageAssetDto(string File, uint Width, uint Height, ulong Bytes);

internal sealed record ImageListDto(List<ImageAssetDto> Images);

/// <summary>Pozycja katalogu widgetów (id + kategoria) z rw_supported_widgets.</summary>
internal sealed record SupportedWidgetDto(string Kind, string Category);

/// <summary>Zestaw wskazówek z magazynu w %APPDATA%\RustyWatch\Hands.</summary>
internal sealed record HandSetDto(string Id, string Name, bool HasSecond, bool HasCap, uint Size);

internal sealed record HandSetListDto(List<HandSetDto> Hands);

/// <summary>Mostek P/Invoke do biblioteki rusty_ffi (rusty-core).</summary>
internal static class RustNative
{
    private const string Library = "rusty_ffi";

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_free_string")]
    private static extern void FreeString(IntPtr ptr);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_version")]
    private static extern IntPtr Version();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_create_project")]
    private static extern IntPtr CreateProject(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string author,
        uint appId);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_create_project_in")]
    private static extern IntPtr CreateProjectIn(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string destination,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string author,
        uint appId);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_save_project_as")]
    private static extern IntPtr SaveProjectAs(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string source,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string destination,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string author,
        uint appId);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_random_app_id")]
    private static extern IntPtr RandomAppId();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_load_project")]
    private static extern IntPtr LoadProject([MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_save_project")]
    private static extern IntPtr SaveProject(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string jsonData);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_project_metadata")]
    private static extern IntPtr SetProjectMetadata(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string author,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string version);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_editor_layers")]
    private static extern IntPtr EditorLayers([MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_move_layer")]
    private static extern IntPtr MoveLayer(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string id,
        int delta);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_widget_visibility")]
    private static extern IntPtr SetWidgetVisibility(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string id,
        [MarshalAs(UnmanagedType.I1)] bool visible);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_supported_widgets")]
    private static extern IntPtr SupportedWidgets();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_frame_geometry")]
    private static extern IntPtr FrameGeometry([MarshalAs(UnmanagedType.LPUTF8Str)] string inputJson);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_add_widget")]
    private static extern IntPtr AddWidget(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string kind,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledDir);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_widget")]
    private static extern IntPtr SetWidget(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string settingsJson,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_measurement_system")]
    private static extern IntPtr SetMeasurementSystem(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string system);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_first_day_of_week")]
    private static extern IntPtr SetFirstDayOfWeek(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string firstDay);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_ensure_defaults")]
    private static extern IntPtr EnsureDefaults(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledFontsDir,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledHandsDir);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_fonts_list")]
    private static extern IntPtr FontsList(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledDir);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_fonts_add")]
    private static extern IntPtr FontsAdd(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledDir,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string sourcePath);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_fonts_remove")]
    private static extern IntPtr FontsRemove(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string file);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_apply_default_font")]
    private static extern IntPtr ApplyDefaultFont(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledDir);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_apply_default_font_to_missing")]
    private static extern IntPtr ApplyDefaultFontToMissing(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledDir);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_backgrounds_list")]
    private static extern IntPtr BackgroundsList(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_backgrounds_add")]
    private static extern IntPtr BackgroundsAdd(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string sourcePath);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_backgrounds_remove")]
    private static extern IntPtr BackgroundsRemove(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string file);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_widget_image")]
    private static extern IntPtr SetWidgetImage(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string widgetId,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string file,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string fit,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_images_list")]
    private static extern IntPtr ImagesList(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_images_add")]
    private static extern IntPtr ImagesAdd(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string sourcePath);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_images_remove")]
    private static extern IntPtr ImagesRemove(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string file);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_ensure_project_background")]
    private static extern IntPtr EnsureProjectBackground(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_background")]
    private static extern IntPtr SetBackground(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string image,
        int autoScale,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_hands_list")]
    private static extern IntPtr HandsList(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string bundledDir);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_hand_visibility")]
    private static extern IntPtr SetHandVisibility(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string widgetId,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string visibilityJson,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_hands_add")]
    private static extern IntPtr HandsAdd(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string folder);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_hands_remove")]
    private static extern IntPtr HandsRemove(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_hands")]
    private static extern IntPtr SetHands(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string widgetId,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string setId,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appDataRoot);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_remove_widget")]
    private static extern IntPtr RemoveWidget(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_validate_project")]
    private static extern IntPtr ValidateProject([MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_export_zip")]
    private static extern IntPtr ExportZip(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string project,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string output);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_prepare_emulator_workspace")]
    private static extern IntPtr PrepareEmulatorWorkspace(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string project,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string root);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_recent_list")]
    private static extern IntPtr RecentList([MarshalAs(UnmanagedType.LPUTF8Str)] string store);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_recent_add")]
    private static extern IntPtr RecentAdd(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string store,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_recent_remove")]
    private static extern IntPtr RecentRemove(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string store,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_recent_clear")]
    private static extern IntPtr RecentClear([MarshalAs(UnmanagedType.LPUTF8Str)] string store);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_open_project")]
    private static extern IntPtr OpenProject(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string store,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_format_time")]
    private static extern IntPtr FormatTime(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string pattern, int hour, int minute, int second,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string locale);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_format_date")]
    private static extern IntPtr FormatDate(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string pattern, int day, int month, int year,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string locale);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_format_date_field")]
    private static extern IntPtr FormatDateField(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string kind, int day, int month, int year,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string firstDay);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_month_name")]
    private static extern IntPtr MonthName(
        int index, [MarshalAs(UnmanagedType.LPUTF8Str)] string locale);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_time_widget")]
    private static extern IntPtr TimeWidget([MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_set_time_widget")]
    private static extern IntPtr SetTimeWidget(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string projectFile,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string settingsJson);

    private static string? Call(Func<IntPtr> nativeCall, out string? error)
    {
        var ptr = nativeCall();
        string text;
        try
        {
            text = Marshal.PtrToStringUTF8(ptr) ?? "{}";
        }
        finally
        {
            FreeString(ptr);
        }

        using var document = JsonDocument.Parse(text);
        var root = document.RootElement;
        if (root.GetProperty("ok").GetBoolean())
        {
            error = null;
            return root.GetProperty("data").GetRawText();
        }

        error = root.GetProperty("error").GetString();
        return null;
    }

    public static string? Version(out string? error) => Call(Version, out error);

    /// <summary>Kopiuje wbudowane czcionki i wskazówki do %APPDATA% przy pierwszym uruchomieniu.</summary>
    public static string? EnsureDefaults(out string? error)
        => Call(() => EnsureDefaults(
            AppPaths.DataDirectory,
            AppPaths.BundledFontsDirectory,
            AppPaths.BundledHandsDirectory), out error);

    public static string? CreateProject(string path, string name, string author, uint appId, out string? error)
        => Call(() => CreateProject(path, name, author, appId), out error);

    public static string? CreateProjectIn(string destination, string name, string author, uint appId, out string? error)
        => Call(() => CreateProjectIn(destination, name, author, appId), out error);

    public static string? SaveProjectAs(string source, string destination, string name, string author, uint appId, out string? error)
        => Call(() => SaveProjectAs(source, destination, name, author, appId), out error);

    public static uint RandomAppId(out string? error)
    {
        var raw = Call(RandomAppId, out error);
        if (raw is null) return 0;
        using var document = JsonDocument.Parse(raw);
        return document.RootElement.GetProperty("appId").GetUInt32();
    }

    public static string? LoadProject(string path, out string? error)
        => Call(() => LoadProject(path), out error);

    public static string? SaveProject(string path, string json, out string? error)
        => Call(() => SaveProject(path, json), out error);

    public static string? SetProjectMetadata(string projectFile, string author, string version, out string? error)
        => Call(() => SetProjectMetadata(projectFile, author, version), out error);

    public static string? EditorLayers(string projectFile, out string? error)
        => Call(() => EditorLayers(projectFile), out error);

    public static string? MoveLayer(string projectFile, string id, int delta, out string? error)
        => Call(() => MoveLayer(projectFile, id, delta), out error);

    public static string? SetWidgetVisibility(string projectFile, string id, bool visible, out string? error)
        => Call(() => SetWidgetVisibility(projectFile, id, visible), out error);

    public static string? SupportedWidgets(out string? error) => Call(SupportedWidgets, out error);

    public static string? FrameGeometry(string inputJson, out string? error)
        => Call(() => FrameGeometry(inputJson), out error);

    public static string? AddWidget(string projectFile, string kind, out string? error)
        => Call(
            () => AddWidget(projectFile, kind, AppPaths.DataDirectory, AppPaths.BundledHandsDirectory),
            out error);

    public static string? SetWidget(string projectFile, string settingsJson, out string? error)
        => Call(() => SetWidget(projectFile, settingsJson, AppPaths.DataDirectory), out error);

    public static string? SetMeasurementSystem(string projectFile, string system, out string? error)
        => Call(() => SetMeasurementSystem(projectFile, system), out error);

    public static string? SetFirstDayOfWeek(string projectFile, string firstDay, out string? error)
        => Call(() => SetFirstDayOfWeek(projectFile, firstDay), out error);

    public static List<FontAssetDto> FontsList(out string? error)
    {
        var raw = Call(
            () => FontsList(AppPaths.DataDirectory, AppPaths.BundledFontsDirectory),
            out error);
        if (raw is null) return new List<FontAssetDto>();
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        var result = JsonSerializer.Deserialize<FontListDto>(raw, options);
        return result?.Fonts ?? new List<FontAssetDto>();
    }

    public static FontAssetDto? FontsAdd(string sourcePath, out string? error)
    {
        var raw = Call(
            () => FontsAdd(AppPaths.DataDirectory, AppPaths.BundledFontsDirectory, sourcePath),
            out error);
        if (raw is null) return null;
        using var document = JsonDocument.Parse(raw);
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        return JsonSerializer.Deserialize<FontAssetDto>(
            document.RootElement.GetProperty("font").GetRawText(), options);
    }

    public static void FontsRemove(string file, out string? error)
    {
        Call(() => FontsRemove(AppPaths.DataDirectory, file), out error);
    }

    public static string? ApplyDefaultFont(string projectFile, out string? error)
        => Call(
            () => ApplyDefaultFont(projectFile, AppPaths.DataDirectory, AppPaths.BundledFontsDirectory),
            out error);

    public static string? ApplyDefaultFontToMissing(string projectFile, out string? error)
        => Call(
            () => ApplyDefaultFontToMissing(projectFile, AppPaths.DataDirectory, AppPaths.BundledFontsDirectory),
            out error);

    public static List<ImageAssetDto> BackgroundsList(out string? error)
    {
        var raw = Call(() => BackgroundsList(AppPaths.DataDirectory), out error);
        if (raw is null) return new List<ImageAssetDto>();
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        return JsonSerializer.Deserialize<ImageListDto>(raw, options)?.Images ?? new List<ImageAssetDto>();
    }

    public static ImageAssetDto? BackgroundsAdd(string sourcePath, out string? error)
    {
        var raw = Call(() => BackgroundsAdd(AppPaths.DataDirectory, sourcePath), out error);
        if (raw is null) return null;
        using var document = JsonDocument.Parse(raw);
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        return JsonSerializer.Deserialize<ImageAssetDto>(
            document.RootElement.GetProperty("image").GetRawText(), options);
    }

    public static void BackgroundsRemove(string file, out string? error)
    {
        Call(() => BackgroundsRemove(AppPaths.DataDirectory, file), out error);
    }

    public static string? SetWidgetImage(string projectFile, string widgetId, string file,
        string fit, out string? error)
        => Call(
            () => SetWidgetImage(projectFile, widgetId, file, fit, AppPaths.DataDirectory),
            out error);

    public static List<ImageAssetDto> ImagesList(out string? error)
    {
        var raw = Call(() => ImagesList(AppPaths.DataDirectory), out error);
        if (raw is null) return new List<ImageAssetDto>();
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        var result = JsonSerializer.Deserialize<ImageListDto>(raw, options);
        return result?.Images ?? new List<ImageAssetDto>();
    }

    public static ImageAssetDto? ImagesAdd(string sourcePath, out string? error)
    {
        var raw = Call(() => ImagesAdd(AppPaths.DataDirectory, sourcePath), out error);
        if (raw is null) return null;
        using var document = JsonDocument.Parse(raw);
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        return JsonSerializer.Deserialize<ImageAssetDto>(
            document.RootElement.GetProperty("image").GetRawText(), options);
    }

    public static void ImagesRemove(string file, out string? error)
    {
        Call(() => ImagesRemove(AppPaths.DataDirectory, file), out error);
    }

    public static void EnsureProjectBackground(string projectFile, out string? error)
        => Call(() => EnsureProjectBackground(projectFile, AppPaths.DataDirectory), out error);

    public static string? SetBackground(string projectFile, string image, bool autoScale, out string? error)
        => Call(
            () => SetBackground(projectFile, image, autoScale ? 1 : 0, AppPaths.DataDirectory),
            out error);

    public static List<SupportedWidgetDto> SupportedWidgetItems(out string? error)
    {
        var raw = SupportedWidgets(out error);
        if (raw is null) return new List<SupportedWidgetDto>();
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        return JsonSerializer.Deserialize<List<SupportedWidgetDto>>(raw, options)
               ?? new List<SupportedWidgetDto>();
    }

    public static List<HandSetDto> HandsList(out string? error)
    {
        var raw = Call(
            () => HandsList(AppPaths.DataDirectory, AppPaths.BundledHandsDirectory),
            out error);
        if (raw is null) return new List<HandSetDto>();
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        var result = JsonSerializer.Deserialize<HandSetListDto>(raw, options);
        return result?.Hands ?? new List<HandSetDto>();
    }

    public static HandSetDto? HandsAdd(string folder, out string? error)
    {
        var raw = Call(() => HandsAdd(AppPaths.DataDirectory, folder), out error);
        if (raw is null) return null;
        using var document = JsonDocument.Parse(raw);
        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        return JsonSerializer.Deserialize<HandSetDto>(
            document.RootElement.GetProperty("hand").GetRawText(), options);
    }

    public static void HandsRemove(string id, out string? error)
    {
        Call(() => HandsRemove(AppPaths.DataDirectory, id), out error);
    }

    public static string? SetHands(string projectFile, string widgetId, string setId, out string? error)
        => Call(
            () => SetHands(projectFile, widgetId, setId, AppPaths.DataDirectory),
            out error);

    /// <summary>Włącza/wyłącza godzinę, minutę, sekundę i zaślepkę.</summary>
    public static string? SetHandVisibility(
        string projectFile, string widgetId,
        bool hour, bool minute, bool second, bool cap, out string? error)
        => Call(
            () => SetHandVisibility(
                projectFile,
                widgetId,
                JsonSerializer.Serialize(new { hour, minute, second, cap }),
                AppPaths.DataDirectory),
            out error);

    public static string? RemoveWidget(string projectFile, string id, out string? error)
        => Call(() => RemoveWidget(projectFile, id), out error);

    public static string? ValidateProject(string path, out string? error)
        => Call(() => ValidateProject(path), out error);

    public static string? ExportZip(string project, string output, out string? error)
        => Call(() => ExportZip(project, output), out error);

    public static string? PrepareEmulatorWorkspace(string project, string root, out string? error)
        => Call(() => PrepareEmulatorWorkspace(project, root), out error);

    public static string? RecentList(string store, out string? error)
        => Call(() => RecentList(store), out error);

    public static string? RecentAdd(string store, string projectFile, out string? error)
        => Call(() => RecentAdd(store, projectFile), out error);

    public static string? RecentRemove(string store, string projectFile, out string? error)
        => Call(() => RecentRemove(store, projectFile), out error);

    public static string? RecentClear(string store, out string? error)
        => Call(() => RecentClear(store), out error);

    public static string? OpenProject(string store, string projectFile, out string? error)
        => Call(() => OpenProject(store, projectFile), out error);

    public static string? FormatTime(string pattern, int hour, int minute, int second,
        string locale, out string? error)
        => Call(() => FormatTime(pattern, hour, minute, second, locale), out error);

    /// <summary>month jest 0-based (zgodnie z Date.getMonth()).</summary>
    public static string? FormatDate(string pattern, int day, int month, int year,
        string locale, out string? error)
        => Call(() => FormatDate(pattern, day, month, year, locale), out error);

    public static string? FormatDateField(string kind, int day, int month, int year,
        string firstDay, out string? error)
        => Call(() => FormatDateField(kind, day, month, year, firstDay), out error);

    public static string? MonthName(int index, string locale, out string? error)
    {
        var raw = Call(() => MonthName(index, locale), out error);
        if (raw is null) return null;
        using var document = JsonDocument.Parse(raw);
        return document.RootElement.GetProperty("name").GetString();
    }

    public static string? TimeWidget(string projectFile, out string? error)
        => Call(() => TimeWidget(projectFile), out error);

    public static string? SetTimeWidget(string projectFile, string settingsJson, out string? error)
        => Call(() => SetTimeWidget(projectFile, settingsJson), out error);
}
