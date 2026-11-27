using System;
using Microsoft.UI.Xaml;
using RustyWatch.Localization;

namespace RustyWatch;

/// <summary>
/// Okno pokazywane zamiast edytora, gdy nie można wczytać rusty_ffi.dll
/// (brak VC++ Redistributable). Po zamknięciu komunikatu aplikacja się wyłącza.
/// </summary>
public sealed partial class VcRedistWindow : Window
{
    private const string DownloadUrl = "https://aka.ms/vs/17/release/vc_redist.x64.exe";

    public VcRedistWindow()
    {
        InitializeComponent();
        AppWindow.Resize(new Windows.Graphics.SizeInt32(720, 480));

        Title = LocalizationService.Get("VcRedistMissingTitle");
        MissingDialog.Title = Title;
        MissingDialog.CloseButtonText = LocalizationService.Get("Close");
        MessageText.Text = LocalizationService.Get("VcRedistMissingMessage");
        DownloadLink.Content = LocalizationService.Get("VcRedistDownload");
        DownloadLink.NavigateUri = new Uri(DownloadUrl);

        // ContentDialog dostaje XamlRoot dopiero po załadowaniu drzewa XAML.
        RootGrid.Loaded += OnLoaded;
    }

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        RootGrid.Loaded -= OnLoaded;
        try
        {
            MissingDialog.XamlRoot = RootGrid.XamlRoot;
            await MissingDialog.ShowAsync();
        }
        catch
        {
            // Nawet gdy dialog nie może się pokazać, nie zostawiamy pustego okna.
        }
        Close();
        Application.Current.Exit();
    }
}
