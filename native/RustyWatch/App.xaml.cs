using System;
using System.IO;
using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;
using RustyWatch.Localization;

namespace RustyWatch;

public partial class App : Application
{
    public static Window? MainWindow { get; private set; }

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // Katalogi i język muszą być gotowe przed pierwszym FFI — inaczej
        // komunikat o braku redistu nie pokaże się w wybranym języku.
        AppPaths.Initialize();
        LocalizationService.Initialize();

        // `rusty_ffi.dll` importuje VCRUNTIME140.dll z VC++ Redistributable.
        // Sprawdzamy funkcjonalnie, zanim MainWindow wywoła pierwsze rw_*.
        if (!CanLoadRustFfi())
        {
            new VcRedistWindow().Activate();
            return;
        }

        MainWindow = new MainWindow();
        MainWindow.Activate();
    }

    private static bool CanLoadRustFfi()
    {
        var library = Path.Combine(AppContext.BaseDirectory, "rusty_ffi.dll");
        if (!File.Exists(library)) return false;
        try
        {
            return NativeLibrary.TryLoad(library, out _);
        }
        catch
        {
            return false;
        }
    }
}
