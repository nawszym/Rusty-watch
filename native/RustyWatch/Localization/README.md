# Katalogi języków / Language catalogs

Pliki z tego folderu są kopiowane do `%APPDATA%\RustyWatch\Localization` przy pierwszym
uruchomieniu aplikacji. Dalej aplikacja czyta je **tylko z `%APPDATA%`** — możesz dodawać
i poprawiać tłumaczenia bez przebudowy i bez ponownej instalacji.

Files from this folder are copied to `%APPDATA%\RustyWatch\Localization` on first launch.
Afterwards the app reads **only from `%APPDATA%`**, so you can add or edit translations
without rebuilding or reinstalling.

## Jak dodać język / Adding a language

1. Skopiuj `en.json` do `<kod>.json`, np. `de.json`. / Copy `en.json` to `<code>.json`.
2. Zmień `languageName` na nazwę języka w tym języku. / Set `languageName` to the name in that language.
3. Przetłumacz wartości w `strings`. / Translate the values under `strings`.
4. W **Ustawieniach aplikacji** kliknij **Odśwież języki**. / In **App Settings** click **Reload languages**.

Brakujące klucze są uzupełniane angielskim (`en.json`). Usunięcie pliku z `%APPDATA%`
przywróci jego wersję wbudowaną przy następnym starcie.

Missing keys fall back to English (`en.json`). Deleting a file from `%APPDATA%` restores
the bundled original on the next launch.

```json
{
  "languageName": "Deutsch",
  "strings": {
    "AppTitle": "Rusty Watch"
  }
}
```
