# Změny

Verze se řídí [sémantickým verzováním](https://semver.org/lang/cs/). Každá verze má na GitHubu tag `vX.Y.Z` a vydání se sestaveným agentem pro Windows.

## 1.1.0 – 2026-11-26

- **Wi-Fi**: telefon se připojí i bez kabelu. Páruje se QR kódem z editoru (záložka Připojení); spárované telefony jdou odebrat.
- **macOS**: agent umí spouštění, klávesové zkratky, přehrávač (Spotify, Hudba), hlasitost, zamknutí a vypnutí displeje. Na skutečném Macu zatím neověřeno.
- Displej telefonu zůstává zapnutý i přes Wi-Fi, kde to prohlížeč běžnou cestou nedovolí.
- Šifrované spojení pro Discord jde při sestavení vynechat (`--no-default-features`).

## 1.0.1 – 2026-11-26

- Projekt má licenci MIT.

## 1.0.0 – 2026-11-26

První vydání.

- Agent pro Windows (Rust), telefon se připojuje přes USB bez instalace aplikace.
- Tlačítka: spouštění programů, klávesové zkratky, hudba, hlasitost, zamknutí PC; čtyři vzhledy, vlastní ikony, textové klávesy.
- Widgety: hodiny, přehrávač, počítač, procesy, kalendář, časovač soustředění, úkoly.
- AI nástroje: relace Claude Code a Codexu, odpověď na žádost o povolení z telefonu, lokální modely (LM Studio, Ollama, servery kompatibilní s OpenAI).
- Discord: kdo je online na serveru, přes veřejný widget serveru.
- Vzhled: deset motivů, tapety, průhledné karty, vlastní písmo, noční režim.
- Editor v prohlížeči na PC: rozložení přetahováním, živý náhled.
