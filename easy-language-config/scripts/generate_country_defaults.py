#!/usr/bin/env python3
"""Generate the app's country/layout tables.

Sources:
- CLDR supplementalData.xml (pinned release, downloaded when --cldr is omitted)
- xkeyboard-config evdev.xml (Arch: /usr/share/X11/xkb/rules/evdev.xml)
- iso-codes iso_639-2.json (Arch: /usr/share/iso-codes/json/iso_639-2.json)
- assets/locales.csv (glibc locales shipped with the app)

Outputs:
- assets/country_defaults.csv: country ISO2, glibc locale, xkb layout
- assets/layouts.csv: xkb layout id, English description

Run on Arch with xkeyboard-config and iso-codes installed:
    python3 scripts/generate_country_defaults.py

The generated files are committed so the app builds without network access.
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import pathlib
import sys
import tempfile
import urllib.request
import xml.etree.ElementTree as ET

CLDR_URL = (
    "https://raw.githubusercontent.com/unicode-org/cldr/release-49/"
    "common/supplemental/supplementalData.xml"
)

# Preferred layout per language when the layout has no countryList entry for
# the territory. Values are xkb layout ids.
PREFERRED_LAYOUT = {
    "ar": "ara",
    "ca": "es",
    "cs": "cz",
    "cy": "gb",
    "da": "dk",
    "de": "de",
    "el": "gr",
    "en": "us",
    "es": "es",
    "eu": "es",
    "fa": "ir",
    "fi": "fi",
    "fr": "fr",
    "ga": "ie",
    "gd": "gb",
    "gl": "es",
    "he": "il",
    "hi": "in",
    "hr": "hr",
    "hu": "hu",
    "hy": "am",
    "it": "it",
    "ja": "jp",
    "ka": "ge",
    "kk": "kz",
    "ko": "kr",
    "lt": "lt",
    "lv": "lv",
    "mk": "mk",
    "nl": "nl",
    "no": "no",
    "pl": "pl",
    "pt": "pt",
    "ro": "ro",
    "ru": "ru",
    "sk": "sk",
    "sl": "si",
    "sq": "al",
    "sr": "rs",
    "sv": "se",
    "th": "th",
    "tr": "tr",
    "uk": "ua",
    "vi": "vn",
    "zh": "cn",
}


def load_iso639(path: pathlib.Path) -> dict[str, str]:
    """Map ISO 639-3/bibliographic codes to ISO 639-1 codes."""
    entries = json.loads(path.read_text(encoding="utf-8"))["639-2"]
    mapping = {}
    for entry in entries:
        alpha2 = entry.get("alpha_2")
        if not alpha2:
            continue
        mapping[entry["alpha_3"]] = alpha2
        if entry.get("bibliographic"):
            mapping[entry["bibliographic"]] = alpha2
    return mapping


def load_layouts(path: pathlib.Path, alpha3_to_alpha2: dict[str, str]):
    layouts = {}
    by_country = collections.defaultdict(list)
    by_language = collections.defaultdict(list)
    for layout in ET.parse(path).getroot().findall("./layoutList/layout"):
        config = layout.find("configItem")
        code = config.findtext("name")
        description = config.findtext("description")
        if not code or not description or code == "custom":
            continue
        layouts[code] = description
        for country in config.findall("./countryList/iso3166Id"):
            by_country[country.text].append(code)
        for language in config.findall("./languageList/iso639Id"):
            alpha2 = alpha3_to_alpha2.get(language.text)
            if alpha2:
                by_language[alpha2].append(code)
    return layouts, by_country, by_language


def load_primary_languages(path: pathlib.Path) -> dict[str, str]:
    """Map territory to its most spoken official language."""
    primary = {}
    for territory in ET.parse(path).getroot().find(".//territoryInfo").findall("territory"):
        code = territory.get("type", "")
        if len(code) != 2 or not code.isalpha():
            continue
        languages = [
            (
                float(entry.get("populationPercent", "0")),
                entry.get("officialStatus", ""),
                entry.get("type"),
            )
            for entry in territory.findall("languagePopulation")
        ]
        official = [entry for entry in languages if entry[1]] or languages
        if not official:
            continue
        official.sort(reverse=True)
        primary[code] = official[0][2].split("_")[0]
    return primary


def choose_layout(
    country: str,
    language: str,
    layouts: dict[str, str],
    by_country: dict[str, list[str]],
    by_language: dict[str, list[str]],
) -> str:
    candidates = [code for code in by_country.get(country, []) if code in layouts]
    if not candidates:
        candidates = sorted(
            by_language.get(language, []),
            key=lambda code: (code != PREFERRED_LAYOUT.get(language), code),
        )
        if not candidates:
            preferred = PREFERRED_LAYOUT.get(language)
            if preferred in layouts:
                return preferred
    return candidates[0] if candidates else "us"


def choose_locale(language: str, country: str, locales: set[str]) -> str:
    exact = f"{language}_{country}"
    if exact in locales:
        return exact
    for locale in sorted(locales):
        if locale.endswith(f"_{country}"):
            return locale
    return "en_US"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cldr", type=pathlib.Path, help="local supplementalData.xml")
    parser.add_argument("--evdev", type=pathlib.Path, default=pathlib.Path("/usr/share/X11/xkb/rules/evdev.xml"))
    parser.add_argument("--iso639", type=pathlib.Path, default=pathlib.Path("/usr/share/iso-codes/json/iso_639-2.json"))
    parser.add_argument("--assets", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parent.parent / "assets")
    args = parser.parse_args()

    if not args.evdev.is_file():
        print(f"missing xkeyboard-config rules: {args.evdev}", file=sys.stderr)
        return 1
    if not args.iso639.is_file():
        print(f"missing iso-codes data: {args.iso639} (install iso-codes)", file=sys.stderr)
        return 1

    if args.cldr:
        cldr_path = args.cldr
        temporary = None
    else:
        temporary = tempfile.NamedTemporaryFile(suffix=".xml", delete=False)
        with urllib.request.urlopen(CLDR_URL) as response:
            temporary.write(response.read())
        temporary.close()
        cldr_path = pathlib.Path(temporary.name)

    locales = {
        row[0]
        for row in csv.reader((args.assets / "locales.csv").open(encoding="utf-8"))
    }
    locales.discard("id")

    alpha3_to_alpha2 = load_iso639(args.iso639)
    layouts, by_country, by_language = load_layouts(args.evdev, alpha3_to_alpha2)
    primary = load_primary_languages(cldr_path)

    duplicates = [
        description
        for description, count in collections.Counter(layouts.values()).items()
        if count > 1
    ]
    if duplicates:
        print(f"duplicate layout descriptions: {duplicates}", file=sys.stderr)
        return 1

    rows = []
    for country in sorted(primary):
        language = primary[country]
        locale = choose_locale(language, country, locales)
        if locale not in locales:
            print(f"no locale available for {country}", file=sys.stderr)
            return 1
        layout = choose_layout(country, language, layouts, by_country, by_language)
        rows.append((country, locale, layout))

    with (args.assets / "layouts.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(["id", "name"])
        for code, description in sorted(layouts.items(), key=lambda item: item[1]):
            writer.writerow([code, description])

    with (args.assets / "country_defaults.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(["country", "locale", "layout"])
        writer.writerows(rows)

    if temporary:
        pathlib.Path(temporary.name).unlink(missing_ok=True)

    print(f"wrote {len(rows)} countries and {len(layouts)} layouts to {args.assets}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
