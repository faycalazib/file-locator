"""Theme generator: one spec per theme (reference images sent by the user on
2026-10-15) -> ui/src/styles/themes/<id>.css, font imports, THEMES list,
theme names in 4 languages, and a contrast report (WCAG AA, 4.5:1).

Run `python scripts/themegen.py` after adding or changing a spec in SPECS; it
can be run again safely (it rewrites what it generated). Then check the
screenshots (docs/THEMES.md, "Adding a theme"). The hand-written themes
(crayons, sonar, clay, carnet, scrapbook, soft, brush, electric, gold) are
not generated."""
import json, pathlib, re, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
UI = ROOT / "ui/src"
sys.stdout.reconfigure(encoding="utf-8")

# ── Fonts: family -> (package, weights, has an arabic subset) ─────────────
FONTS = {
    "Lora": ("lora", [400, 600, 700]),
    "Manrope": ("manrope", [400, 500, 600, 700, 800]),
    "Playfair Display": ("playfair-display", [600, 700]),
    "DM Serif Display": ("dm-serif-display", [400]),
    "Josefin Sans": ("josefin-sans", [600, 700]),
    "DM Sans": ("dm-sans", [400, 500, 700]),
    "Urbanist": ("urbanist", [400, 600, 700]),
    "Plus Jakarta Sans": ("plus-jakarta-sans", [400, 600, 700]),
    "Oxanium": ("oxanium", [400, 600, 700]),
    "Rajdhani": ("rajdhani", [500, 600, 700]),
    "Barlow": ("barlow", [300, 400, 500, 700]),
    "IBM Plex Sans": ("ibm-plex-sans", [400, 500, 600, 700]),
}
ARABIC_FONTS = {"Amiri": ("amiri", [400, 700])}

# ── Specs ────────────────────────────────────────────────────────────────
# Each spec: ids, names, chrome, swatch, ref, colors, fonts, material, page.
SPECS = [
  dict(id="leafora", ref="Leafora (plant shop, dark sage glass)", chrome="dark",
       names=dict(fr="Feuillage", en="Foliage", es="Follaje", ar="أوراق الشجر"),
       swatch=["#1e2a22", "#2a382e", "#a8cf8e", "#e6d58a"],
       c=dict(bg="#1e2a22", surface="#2a382e", text="#eef3ea", dim="#b9c7b6", faint="#6f7f6c", line="#46574a",
              green="#a8d08d", blue="#8fc1d8", pink="#e8a3b5", orange="#e3b27a", purple="#b9a6e0", yellow="#e6d58a",
              accent="#a8cf8e", accent_ink="#c3e2ad", secondary="#8fc1d8", secondary_ink="#b5dbe9",
              ok="#a8d08d", ok_ink="#c3e2ad", warn="#e3b27a", warn_ink="#f0cfa6", danger="#e88b8b", danger_ink="#f2b0b0",
              query="#e5f1dc", mark_text="#16231a", mark_bg="#a8cf8e", mark_active_text="#1f1a05", mark_active_bg="#e6d58a"),
       title_color="#eef3ea", title_shadow="drop-shadow(0 4px 14px rgb(0 0 0 / 0.4))",
       fonts=dict(brand="Lora", display="Manrope", body="Manrope", mono="JetBrains Mono"),
       m=dict(radius="22px", bw="1px", mix=35, highlight="rgb(255 255 255 / 0.08)", drop="0 18px 34px -20px rgb(0 0 0 / 0.55)"),
       page="radial-gradient(circle at 86% 18%, rgb(130 170 115 / 0.32), transparent 34%), radial-gradient(circle at 8% 92%, rgb(95 135 95 / 0.3), transparent 40%), linear-gradient(160deg, #26352b, #172019)"),

  dict(id="potea", ref="Potea (plant shop, charcoal and vivid green)", chrome="dark",
       names=dict(fr="Pousse", en="Sprout", es="Brote", ar="برعم"),
       swatch=["#181a20", "#1f222a", "#12d18e", "#4c9bff"],
       c=dict(bg="#181a20", surface="#1f222a", text="#f5f6f8", dim="#a3a7b3", faint="#5c616d", line="#2c303a",
              green="#12d18e", blue="#4c9bff", pink="#ff6b9a", orange="#ff9f43", purple="#a78bfa", yellow="#facc15",
              accent="#12d18e", accent_ink="#4ee0a8", secondary="#4c9bff", secondary_ink="#85baff",
              ok="#12d18e", ok_ink="#4ee0a8", warn="#facc15", warn_ink="#fde047", danger="#f75555", danger_ink="#ff8a8a",
              query="#ffffff", mark_text="#06281b", mark_bg="#12d18e", mark_active_text="#2a2200", mark_active_bg="#fde047"),
       title_color="#12d18e", title_shadow="drop-shadow(0 6px 16px rgb(18 209 142 / 0.3))",
       fonts=dict(brand="Urbanist", display="Urbanist", body="Urbanist", mono="JetBrains Mono"),
       m=dict(radius="16px", bw="1px", mix=30, drop="0 16px 30px -20px rgb(0 0 0 / 0.7)"),
       page="#181a20"),

  dict(id="neural", ref="Neural Profit Interface (lime HUD)", chrome="dark", case="uppercase",
       names=dict(fr="Interface neurale", en="Neural HUD", es="Interfaz neural", ar="واجهة عصبية"),
       swatch=["#020402", "#060c06", "#8cff3c", "#3cffd2"],
       c=dict(bg="#020402", surface="#071007", text="#d4ffbf", dim="#8fcf72", faint="#3d6b2c", line="#2f5a20",
              green="#8cff3c", blue="#3cffd2", pink="#ff5fa2", orange="#ffb13c", purple="#b48cff", yellow="#e8ff3c",
              accent="#8cff3c", accent_ink="#a8ff6a", secondary="#3cffd2", secondary_ink="#7affe2",
              ok="#8cff3c", ok_ink="#a8ff6a", warn="#ffb13c", warn_ink="#ffc96e", danger="#ff4d4d", danger_ink="#ff8080",
              query="#e8ffdc", mark_text="#eaffdc", mark_bg="rgb(140 255 60 / 0.25)", mark_active_text="#021000", mark_active_bg="#8cff3c"),
       title_color="#b7ff6a", title_shadow="drop-shadow(0 0 10px rgb(140 255 60 / 0.7))",
       fonts=dict(brand="Oxanium", display="Oxanium", body="Oxanium", mono="JetBrains Mono"), rail="310px",
       m=dict(radius="4px", bw="1px", fixed="rgb(140 255 60 / 0.55)", glow="0 0 16px -6px rgb(140 255 60 / 0.7)"),
       page="radial-gradient(circle at 40% 18%, rgb(140 255 60 / 0.1), transparent 40%), linear-gradient(rgb(140 255 60 / 0.045) 1px, transparent 1px) 0 0 / 28px 28px, linear-gradient(90deg, rgb(140 255 60 / 0.045) 1px, transparent 1px) 0 0 / 28px 28px, #020402"),

  dict(id="nebula", ref="AI assistant and Futuristic Glass UI (indigo glass, violet and pink glow)", chrome="dark",
       names=dict(fr="Nébuleuse", en="Nebula", es="Nebulosa", ar="سديم"),
       swatch=["#070a1f", "#11163a", "#8b6bff", "#ff5fcf"],
       c=dict(bg="#070a1f", surface="#11163a", text="#eef0ff", dim="#aab0dd", faint="#565c8f", line="#2a3170",
              green="#4ade80", blue="#5b8cff", pink="#ff5fcf", orange="#ff9a5c", purple="#9b6bff", yellow="#ffd166",
              accent="#8b6bff", accent_ink="#b9a4ff", secondary="#ff5fcf", secondary_ink="#ff9be0",
              ok="#4ade80", ok_ink="#7ee8a6", warn="#ffd166", warn_ink="#ffe09a", danger="#ff5c7a", danger_ink="#ff93a6",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(139 107 255 / 0.38)", mark_active_text="#ffffff", mark_active_bg="#6a4be0"),
       title_color="#ffffff", title_shadow="drop-shadow(0 0 16px rgb(155 107 255 / 0.75))",
       fonts=dict(brand="Outfit", display="Outfit", body="Outfit", mono="JetBrains Mono"),
       m=dict(radius="22px", bw="1px", mix=55, glow="0 0 22px -9px var(--k, rgb(139 107 255 / 0.6))", highlight="rgb(255 255 255 / 0.07)",
              drop="0 20px 40px -22px rgb(0 0 0 / 0.8)"),
       page="radial-gradient(ellipse at 92% 30%, rgb(255 95 207 / 0.22), transparent 40%), radial-gradient(ellipse at 8% 82%, rgb(91 140 255 / 0.22), transparent 45%), radial-gradient(circle at 50% -6%, rgb(139 107 255 / 0.28), transparent 42%), radial-gradient(circle at 1px 1px, rgb(255 255 255 / 0.12) 1px, transparent 1.6px) 0 0 / 34px 34px, #070a1f"),

  dict(id="lagoon", ref="Plant app (deep teal and emerald)", chrome="dark",
       names=dict(fr="Lagon", en="Lagoon", es="Laguna", ar="بحيرة"),
       swatch=["#0c2c2e", "#13403f", "#3fe0b5", "#5fc8f0"],
       c=dict(bg="#0c2c2e", surface="#13403f", text="#e8fbf6", dim="#a6d7cc", faint="#4f8a80", line="#23605a",
              green="#4fe3a0", blue="#5fc8f0", pink="#ff8fb1", orange="#ffb36b", purple="#b99cff", yellow="#f4e06d",
              accent="#3fe0b5", accent_ink="#7aefcf", secondary="#5fc8f0", secondary_ink="#95dcf6",
              ok="#4fe3a0", ok_ink="#8af0bf", warn="#f4e06d", warn_ink="#f8ea9e", danger="#ff7a7a", danger_ink="#ffa8a8",
              query="#ffffff", mark_text="#052a22", mark_bg="#3fe0b5", mark_active_text="#2a2400", mark_active_bg="#f4e06d"),
       title_color="#7af0cf", title_shadow="drop-shadow(0 6px 18px rgb(0 0 0 / 0.35))",
       fonts=dict(brand="Manrope", display="Manrope", body="Manrope", mono="JetBrains Mono"),
       m=dict(radius="26px", bw="1px", mix=40, highlight="rgb(255 255 255 / 0.08)", drop="0 18px 36px -22px rgb(0 0 0 / 0.6)"),
       page="radial-gradient(circle at 95% 92%, rgb(63 224 181 / 0.18), transparent 35%), radial-gradient(circle at 4% 10%, rgb(95 200 240 / 0.14), transparent 30%), linear-gradient(160deg, #10393a, #072022)"),

  dict(id="cosmo", ref="Plixie (space education game, vivid purple and yellow)", chrome="dark",
       names=dict(fr="Cosmo", en="Cosmo", es="Cosmo", ar="كوزمو"),
       swatch=["#5a2ee6", "#4321c4", "#ffd23f", "#3ad1ff"],
       c=dict(bg="#5a2ee6", surface="#4321c4", text="#ffffff", dim="#ddd2ff", faint="#8f78e8", line="#7c5cf2",
              green="#3ee08a", blue="#3ad1ff", pink="#ff6fb5", orange="#ff9a3c", purple="#c9a8ff", yellow="#ffd23f",
              accent="#ffd23f", accent_ink="#ffe27a", secondary="#3ad1ff", secondary_ink="#8fe6ff",
              ok="#3ee08a", ok_ink="#9ff0c3", warn="#ff9a3c", warn_ink="#ffc28a", danger="#ff5d7a", danger_ink="#ffb0be",
              query="#ffffff", mark_text="#2b1a00", mark_bg="#ffd23f", mark_active_text="#3a1fb0", mark_active_bg="#ffffff"),
       title_shadow="drop-shadow(0 5px 0 rgb(30 10 110 / 0.55))",
       fonts=dict(brand="Fredoka", display="Nunito", body="Nunito", mono="DM Mono"),
       m=dict(radius="24px", bw="2px", mix=30, highlight="rgb(255 255 255 / 0.18)", drop="0 16px 26px -14px rgb(25 5 90 / 0.7)"),
       doodles=1,
       page="radial-gradient(circle at 12% 88%, rgb(255 154 60 / 0.42), transparent 22%), radial-gradient(circle at 92% 8%, rgb(150 100 255 / 0.85), transparent 30%), linear-gradient(135deg, #6236f0, #4a22d8)"),

  dict(id="greenhouse", ref="Plant care app (frosted sage glass, light)", chrome="light",
       names=dict(fr="Serre", en="Greenhouse", es="Invernadero", ar="دفيئة"),
       swatch=["#e7efe6", "#f6faf4", "#4f8a5c", "#d6b54a"],
       c=dict(bg="#e7efe6", surface="#f6faf4", text="#1f2e22", dim="#55685a", faint="#9db2a2", line="#d3e0d2",
              green="#4f8a5c", blue="#5a8fb5", pink="#d9708f", orange="#d98c4a", purple="#8a76c4", yellow="#d6b54a",
              accent="#4f8a5c", accent_ink="#3b6e47", secondary="#5a8fb5", secondary_ink="#3f6f93",
              ok="#4f8a5c", ok_ink="#3b6e47", warn="#d98c4a", warn_ink="#8a5a1a", danger="#d65a5a", danger_ink="#b03838",
              query="#2f5d3a", mark_text="#1f2e22", mark_bg="#cfe6c6", mark_active_text="#ffffff", mark_active_bg="#3b6e47"),
       title_color="#2f5d3a", title_shadow="drop-shadow(0 6px 14px rgb(40 70 45 / 0.18))",
       fonts=dict(brand="Plus Jakarta Sans", display="Plus Jakarta Sans", body="Plus Jakarta Sans", mono="JetBrains Mono"),
       m=dict(radius="24px", bw="1px", mix=25, highlight="rgb(255 255 255 / 0.9)", drop="0 16px 30px -22px rgb(40 70 45 / 0.45)"),
       page="radial-gradient(circle at 0% 30%, rgb(120 170 120 / 0.35), transparent 30%), radial-gradient(circle at 100% 0%, rgb(140 180 140 / 0.35), transparent 35%), radial-gradient(circle at 80% 100%, rgb(160 200 160 / 0.4), transparent 35%), #e7efe6"),

  dict(id="noor", ref="Noor (Islamic companion app: cream, forest green, gold, serif)", chrome="light",
       names=dict(fr="Noor", en="Noor", es="Noor", ar="نور"),
       swatch=["#f6efe4", "#fffaf2", "#2f5d4b", "#c9a24a"],
       c=dict(bg="#f6efe4", surface="#fffaf2", text="#2b3a32", dim="#5d6b62", faint="#b0a58f", line="#e3d6bf",
              green="#2f5d4b", blue="#4c7a8c", pink="#c9707a", orange="#c98b4a", purple="#8a6f9e", yellow="#c9a24a",
              accent="#2f5d4b", accent_ink="#2f5d4b", secondary="#c9a24a", secondary_ink="#80621c",
              ok="#3f7a5c", ok_ink="#2f6a4c", warn="#c98b4a", warn_ink="#8a5410", danger="#b8494f", danger_ink="#9a3036",
              query="#2f5d4b", mark_text="#2b3a32", mark_bg="#f0dfae", mark_active_text="#ffffff", mark_active_bg="#2f5d4b"),
       title_color="#2f5d4b", title_shadow="drop-shadow(0 3px 0 rgb(201 162 74 / 0.4))",
       fonts=dict(brand="Playfair Display", display="Lora", body="Lora", mono="Courier Prime"),
       arabic=dict(brand="Amiri", body="Amiri"),
       m=dict(radius="22px", bw="1px", mix=30, highlight="rgb(255 255 255 / 0.8)", drop="0 14px 26px -18px rgb(90 70 30 / 0.35)"),
       page="radial-gradient(circle at 50% -4%, rgb(232 210 160 / 0.55), transparent 42%), radial-gradient(circle at 100% 100%, rgb(47 93 75 / 0.08), transparent 35%), #f6efe4"),

  dict(id="orchard", ref="Fruit nutrition app (pastel mint to peach)", chrome="light",
       names=dict(fr="Verger", en="Orchard", es="Huerto", ar="بستان"),
       swatch=["#e2f5e6", "#fde3d6", "#2f8f57", "#f08a3c"],
       c=dict(bg="#f4f1e6", surface="#fffdf7", text="#233127", dim="#5b6b60", faint="#a8b3a6", line="#e5e8dc",
              green="#3f9c5a", blue="#4a9ad4", pink="#e8607a", orange="#f08a3c", purple="#9a6fd0", yellow="#f2c23a",
              accent="#2f8f57", accent_ink="#237447", secondary="#f08a3c", secondary_ink="#a8530f",
              ok="#3f9c5a", ok_ink="#237447", warn="#f08a3c", warn_ink="#9a5a00", danger="#e05a4a", danger_ink="#b33a2c",
              query="#237447", mark_text="#233127", mark_bg="#ffe3a3", mark_active_text="#ffffff", mark_active_bg="#237447"),
       title_shadow="drop-shadow(0 6px 10px rgb(60 80 40 / 0.18))",
       fonts=dict(brand="DM Sans", display="DM Sans", body="DM Sans", mono="DM Mono"),
       m=dict(radius="22px", bw="1px", mix=20, highlight="rgb(255 255 255 / 0.85)", drop="0 14px 28px -20px rgb(60 80 40 / 0.35)"),
       doodles=1,
       page="linear-gradient(135deg, #dcf4e2, #f7f0dc 50%, #fde0d6)"),

  dict(id="candy", ref="Liquid glass pastel widgets (glossy candy buttons)", chrome="light",
       names=dict(fr="Bonbon", en="Candy", es="Caramelo", ar="حلوى"),
       swatch=["#f3e9f7", "#fdf8ff", "#8f6be8", "#f070a8"],
       c=dict(bg="#f3e9f7", surface="#fdf8ff", text="#2e2440", dim="#665b7a", faint="#b3a8c4", line="#eadff2",
              green="#4cc38a", blue="#5aa8f0", pink="#f070a8", orange="#f59a4a", purple="#8f6be8", yellow="#f5c83a",
              accent="#8f6be8", accent_ink="#6a48c8", secondary="#f070a8", secondary_ink="#b8306c",
              ok="#4cc38a", ok_ink="#237a50", warn="#f59a4a", warn_ink="#8a5a00", danger="#e5506a", danger_ink="#b52e48",
              query="#6a48c8", mark_text="#2e2440", mark_bg="#f9d3e6", mark_active_text="#ffffff", mark_active_bg="#7550d8"),
       title_shadow="drop-shadow(0 6px 10px rgb(143 107 232 / 0.35))",
       fonts=dict(brand="Fredoka", display="Nunito", body="Nunito", mono="DM Mono"),
       m=dict(radius="28px", bw="1.5px", mix=35,
              highlight="rgb(255 255 255 / 0.95)", inset="inset 0 12px 18px -12px rgb(255 255 255 / 0.95), inset 0 -6px 12px -8px rgb(143 107 232 / 0.25)",
              drop="0 12px 24px -12px rgb(143 107 232 / 0.45)"),
       page="radial-gradient(circle at 10% 15%, rgb(255 190 220 / 0.7), transparent 30%), radial-gradient(circle at 90% 20%, rgb(190 220 255 / 0.7), transparent 30%), radial-gradient(circle at 20% 90%, rgb(200 245 210 / 0.7), transparent 30%), radial-gradient(circle at 85% 85%, rgb(255 240 180 / 0.7), transparent 30%), #f3e9f7"),

  dict(id="cocoon", ref="Music dashboard (cozy cream, sage, peach)", chrome="light",
       names=dict(fr="Cocon", en="Cocoon", es="Capullo", ar="شرنقة"),
       swatch=["#f8efe4", "#fffaf3", "#e8735f", "#7fae95"],
       c=dict(bg="#f8efe4", surface="#fffaf3", text="#3d3530", dim="#6f645c", faint="#b9aa9a", line="#ecdfd0",
              green="#7fae95", blue="#7fa7c9", pink="#f08a7a", orange="#f2a15a", purple="#a58ac9", yellow="#f2c55a",
              accent="#e8735f", accent_ink="#b8452f", secondary="#6f9e87", secondary_ink="#3f6e57",
              ok="#7fae95", ok_ink="#3f6e57", warn="#f2a15a", warn_ink="#8a5a10", danger="#d9534f", danger_ink="#a83a36",
              query="#b8452f", mark_text="#3d3530", mark_bg="#fde2b8", mark_active_text="#2a1510", mark_active_bg="#f2a090"),
       title_color="#5a4a40", title_shadow="drop-shadow(0 4px 0 rgb(120 90 60 / 0.15))",
       fonts=dict(brand="Baloo 2", display="Nunito", body="Nunito", mono="DM Mono"),
       m=dict(radius="22px", bw="1px", mix=18, highlight="rgb(255 255 255 / 0.9)",
              drop="0 6px 0 -2px rgb(120 90 60 / 0.08), 0 18px 28px -20px rgb(120 90 60 / 0.4)"),
       doodles=1,
       page="radial-gradient(circle at 0% 100%, rgb(127 174 149 / 0.25), transparent 30%), radial-gradient(circle at 100% 0%, rgb(240 138 122 / 0.18), transparent 28%), #f8efe4"),

  dict(id="marine", ref="AutoHub (navy glass, blue bokeh)", chrome="dark",
       names=dict(fr="Marine", en="Navy", es="Marino", ar="بحري"),
       swatch=["#071226", "#0f1f3d", "#3d8bff", "#5fd0ff"],
       c=dict(bg="#071226", surface="#0f1f3d", text="#eaf1ff", dim="#a7b8d8", faint="#4e6182", line="#23365c",
              green="#3fd18b", blue="#3d8bff", pink="#ff6b9a", orange="#ff9a4d", purple="#8b7bff", yellow="#ffd166",
              accent="#3d8bff", accent_ink="#8ab8ff", secondary="#5fd0ff", secondary_ink="#9be0ff",
              ok="#3fd18b", ok_ink="#7ee0ad", warn="#ffd166", warn_ink="#ffe09a", danger="#ff5c6e", danger_ink="#ff97a3",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(61 139 255 / 0.38)", mark_active_text="#ffffff", mark_active_bg="#2563d8"),
       title_color="#ffffff", title_shadow="drop-shadow(0 0 16px rgb(61 139 255 / 0.65))",
       fonts=dict(brand="Plus Jakarta Sans", display="Plus Jakarta Sans", body="Plus Jakarta Sans", mono="JetBrains Mono"),
       m=dict(radius="18px", bw="1px", mix=40, glow="0 0 18px -9px var(--k, rgb(61 139 255 / 0.5))", highlight="rgb(255 255 255 / 0.08)",
              drop="0 18px 34px -20px rgb(0 0 0 / 0.75)"),
       page="radial-gradient(circle at 18% 14%, rgb(61 139 255 / 0.26), transparent 24%), radial-gradient(circle at 76% 8%, rgb(120 170 255 / 0.2), transparent 20%), radial-gradient(circle at 60% 92%, rgb(61 139 255 / 0.16), transparent 30%), linear-gradient(180deg, #0a1a36, #050c1c)"),

  dict(id="daylight", ref="Daylight (wellness: off-white, periwinkle, butter, navy)", chrome="light",
       names=dict(fr="Lumière du jour", en="Daylight", es="Luz del día", ar="ضوء النهار"),
       swatch=["#f7f5fb", "#ffffff", "#6f78cf", "#f5d46a"],
       c=dict(bg="#f7f5fb", surface="#ffffff", text="#1f2147", dim="#5c5f86", faint="#a9abc7", line="#e4e3f2",
              green="#6fb58a", blue="#8e95d8", pink="#e88aa6", orange="#f0a668", purple="#8e95d8", yellow="#f5d46a",
              accent="#6f78cf", accent_ink="#4b53a8", secondary="#f5d46a", secondary_ink="#7a5c00",
              ok="#5a9a72", ok_ink="#3d7a55", warn="#f0a668", warn_ink="#8a5a00", danger="#d95c6a", danger_ink="#b03a4a",
              query="#2b2d5c", mark_text="#1f2147", mark_bg="#fbe9a6", mark_active_text="#ffffff", mark_active_bg="#2b2d5c"),
       title_color="#1f2147", title_shadow="drop-shadow(0 0 0 transparent)",
       fonts=dict(brand="Josefin Sans", display="DM Sans", body="DM Sans", mono="DM Mono"),
       m=dict(radius="20px", bw="1px", mix=25, drop="0 12px 28px -20px rgb(31 33 71 / 0.28)"),
       page="radial-gradient(circle at 92% 4%, rgb(245 212 106 / 0.38), transparent 20%), radial-gradient(circle at 0% 100%, rgb(142 149 216 / 0.25), transparent 35%), #f7f5fb",
       extra="[data-theme='daylight'] .title { letter-spacing: 8px; }"),

  dict(id="cloud", ref="Recenter (wellness: sky, clouds, mint, serif)", chrome="light",
       names=dict(fr="Nuage", en="Cloud", es="Nube", ar="سحابة"),
       swatch=["#cfe7ff", "#fbfdff", "#2c7a52", "#5aa9e6"],
       c=dict(bg="#dcefff", surface="#fbfdff", text="#1d2b3a", dim="#556577", faint="#a3b3c4", line="#d8e6f3",
              green="#4fae7d", blue="#5aa9e6", pink="#ef8aa6", orange="#f0a050", purple="#9a86d8", yellow="#f5c842",
              accent="#3f9a6c", accent_ink="#2c7a52", secondary="#5aa9e6", secondary_ink="#2f72a8",
              ok="#4fae7d", ok_ink="#2c7a52", warn="#f0a050", warn_ink="#8a5a00", danger="#d9534f", danger_ink="#b0383a",
              query="#1d2b3a", mark_text="#1d2b3a", mark_bg="#d2f0da", mark_active_text="#ffffff", mark_active_bg="#2c7a52"),
       title_color="#14261d", title_shadow="drop-shadow(0 6px 14px rgb(60 110 160 / 0.22))",
       fonts=dict(brand="DM Serif Display", display="Nunito", body="Nunito", mono="DM Mono"),
       m=dict(radius="24px", bw="1px", mix=20, highlight="rgb(255 255 255 / 0.95)", drop="0 16px 30px -18px rgb(60 110 160 / 0.35)"),
       doodles=1,
       page="radial-gradient(ellipse at 8% 100%, #ffffff 0, transparent 24%), radial-gradient(ellipse at 92% 96%, #ffffff 0, transparent 22%), radial-gradient(ellipse at 50% 108%, rgb(255 255 255 / 0.95), transparent 26%), linear-gradient(180deg, #bfe0ff, #e8f4ff 62%)"),

  dict(id="lavender", ref="AirPods shop and Moon Play (lilac and deep violet)", chrome="light",
       names=dict(fr="Lavande", en="Lavender", es="Lavanda", ar="خزامى"),
       swatch=["#efeafc", "#ffffff", "#6c48d8", "#e86fb0"],
       c=dict(bg="#efeafc", surface="#ffffff", text="#231a44", dim="#5c5180", faint="#aaa1c8", line="#e2daf7",
              green="#4cbf8a", blue="#6b8cff", pink="#e86fb0", orange="#f39a5a", purple="#7a52d6", yellow="#f5c84a",
              accent="#6c48d8", accent_ink="#5a36c4", secondary="#e86fb0", secondary_ink="#b53a7c",
              ok="#4cbf8a", ok_ink="#237a50", warn="#f39a5a", warn_ink="#8a5a00", danger="#e5506a", danger_ink="#b52e48",
              query="#5a36c4", mark_text="#231a44", mark_bg="#e4d9ff", mark_active_text="#ffffff", mark_active_bg="#2a1b6e"),
       title_color="#5a36c4", title_shadow="drop-shadow(0 8px 14px rgb(90 54 196 / 0.25))",
       fonts=dict(brand="Plus Jakarta Sans", display="Plus Jakarta Sans", body="Plus Jakarta Sans", mono="DM Mono"),
       m=dict(radius="22px", bw="1px", mix=25, drop="0 16px 30px -20px rgb(90 54 196 / 0.35)"),
       page="radial-gradient(circle at 10% 10%, rgb(160 130 240 / 0.35), transparent 30%), radial-gradient(circle at 90% 90%, rgb(232 111 176 / 0.2), transparent 30%), #efeafc"),

  dict(id="frost", ref="Login screen (neumorphism, soft blue)", chrome="light",
       names=dict(fr="Givre", en="Frost", es="Escarcha", ar="صقيع"),
       swatch=["#e6ecf8", "#e6ecf8", "#2f6bff", "#7a5cff"],
       c=dict(bg="#e6ecf8", surface="#e9eefa", text="#18233d", dim="#4b5a78", faint="#9aa8c4", line="#d2dbee",
              green="#3fb37f", blue="#2f6bff", pink="#e8619a", orange="#f2924a", purple="#7a5cff", yellow="#f2c02f",
              accent="#2f6bff", accent_ink="#1e4fd6", secondary="#7a5cff", secondary_ink="#5a3ee0",
              ok="#3fb37f", ok_ink="#1f7a50", warn="#f2924a", warn_ink="#8a5a00", danger="#e5485e", danger_ink="#b8283e",
              query="#1e4fd6", mark_text="#18233d", mark_bg="#cddbff", mark_active_text="#ffffff", mark_active_bg="#2459e6"),
       title_color="#0e1733", title_shadow="drop-shadow(3px 3px 5px rgb(163 177 210 / 0.8)) drop-shadow(-3px -3px 5px rgb(255 255 255 / 0.9))",
       fonts=dict(brand="Manrope", display="Manrope", body="Manrope", mono="JetBrains Mono"),
       m=dict(radius="18px", bw="1px", mix=10, inset="", drop="7px 7px 16px rgb(163 177 210 / 0.6), -7px -7px 16px rgb(255 255 255 / 0.9)"),
       page="radial-gradient(circle at 0% 0%, rgb(47 107 255 / 0.16), transparent 25%), radial-gradient(circle at 100% 100%, rgb(47 107 255 / 0.13), transparent 25%), #e6ecf8"),

  dict(id="starry", ref="Sleep app (indigo night, stars, yellow)", chrome="dark",
       names=dict(fr="Nuit étoilée", en="Starry night", es="Noche estrellada", ar="ليلة مرصعة بالنجوم"),
       swatch=["#0c1140", "#161c5a", "#ffc34d", "#9a7bff"],
       c=dict(bg="#0c1140", surface="#161c5a", text="#f3f1ff", dim="#b9b6e8", faint="#5c62a8", line="#2c3480",
              green="#5fe0a0", blue="#6aa8ff", pink="#ff8ccf", orange="#ffb35c", purple="#9a7bff", yellow="#ffc34d",
              accent="#ffc34d", accent_ink="#ffd78a", secondary="#9a7bff", secondary_ink="#c2adff",
              ok="#5fe0a0", ok_ink="#8ef0bf", warn="#ffb35c", warn_ink="#ffd78a", danger="#ff6b7d", danger_ink="#ffa0ab",
              query="#ffffff", mark_text="#241600", mark_bg="#ffc34d", mark_active_text="#ffffff", mark_active_bg="#5b3fd6"),
       title_color="#ffffff", title_shadow="drop-shadow(0 0 14px rgb(255 195 77 / 0.5))",
       fonts=dict(brand="Lora", display="Manrope", body="Manrope", mono="JetBrains Mono"),
       m=dict(radius="22px", bw="1px", mix=35, glow="0 0 16px -8px var(--k, rgb(154 123 255 / 0.5))", highlight="rgb(255 255 255 / 0.06)",
              drop="0 18px 34px -20px rgb(0 0 0 / 0.7)"),
       page="radial-gradient(circle at 86% 10%, rgb(255 195 77 / 0.22), transparent 14%), radial-gradient(circle at 1px 1px, rgb(255 255 255 / 0.4) 1px, transparent 1.6px) 0 0 / 90px 90px, radial-gradient(circle at 1px 1px, rgb(255 255 255 / 0.2) 1px, transparent 1.5px) 45px 30px / 60px 60px, linear-gradient(180deg, #121a5c, #080b2e)"),

  dict(id="aurora", ref="Glazzed dashboard (blurred color gradient, dark panels)", chrome="dark",
       names=dict(fr="Aurore", en="Aurora", es="Aurora", ar="شفق"),
       swatch=["#2a3a6a", "#222c4d", "#4a9bff", "#ff6b8a"],
       c=dict(bg="#1b2440", surface="#222c4d", text="#f0f3fa", dim="#b5bed6", faint="#5c6788", line="#34406a",
              green="#59d18c", blue="#4a9bff", pink="#ff6b8a", orange="#ff9f5a", purple="#a07bff", yellow="#ffd166",
              accent="#4a9bff", accent_ink="#8cc0ff", secondary="#59d18c", secondary_ink="#8de6b0",
              ok="#59d18c", ok_ink="#8de6b0", warn="#ffd166", warn_ink="#ffe09a", danger="#ff5c6e", danger_ink="#ff97a3",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(74 155 255 / 0.38)", mark_active_text="#ffffff", mark_active_bg="#2a6fd6"),
       title_color="#ffffff", title_shadow="drop-shadow(0 4px 12px rgb(0 0 0 / 0.35))",
       fonts=dict(brand="Nunito Sans", display="Nunito Sans", body="Nunito Sans", mono="JetBrains Mono"),
       m=dict(radius="4px", bw="1px", mix=30, highlight="rgb(255 255 255 / 0.05)", drop="0 14px 28px -18px rgb(0 0 0 / 0.6)"),
       page="radial-gradient(circle at 15% 20%, rgb(60 120 200 / 0.6), transparent 40%), radial-gradient(circle at 80% 40%, rgb(200 60 90 / 0.45), transparent 40%), radial-gradient(circle at 40% 90%, rgb(70 160 140 / 0.5), transparent 45%), #1b2440"),

  dict(id="sport", ref="Running app (cyan to indigo gradient, thin type)", chrome="dark", case="uppercase",
       names=dict(fr="Élan", en="Stride", es="Impulso", ar="انطلاق"),
       swatch=["#1fb5d6", "#22337f", "#19c2ff", "#ff5fa8"],
       c=dict(bg="#1d2a6b", surface="#22337f", text="#ffffff", dim="#c2cdf3", faint="#6b7bc0", line="#3b4c9a",
              green="#3fe0c0", blue="#34c3ff", pink="#ff5fa8", orange="#ff9a5c", purple="#9a7bff", yellow="#ffd24a",
              accent="#19c2ff", accent_ink="#7ad8ff", secondary="#ff5fa8", secondary_ink="#ffa3cd",
              ok="#3fe0c0", ok_ink="#8af0dc", warn="#ffd24a", warn_ink="#ffe08a", danger="#ff6b7d", danger_ink="#ffacb5",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(25 194 255 / 0.32)", mark_active_text="#001a2a", mark_active_bg="#19c2ff"),
       title_color="#ffffff", title_shadow="drop-shadow(0 6px 16px rgb(5 10 40 / 0.4))",
       fonts=dict(brand="Barlow", display="Barlow", body="Barlow", mono="JetBrains Mono"),
       m=dict(radius="6px", bw="1px", mix=30, drop="0 16px 30px -18px rgb(5 10 40 / 0.6)"),
       page="linear-gradient(160deg, #1fb5d6 0%, #2a3f9e 45%, #1a1f5e 100%)",
       extra="[data-theme='sport'] .title { font-weight: 300; letter-spacing: 10px; }"),

  dict(id="fleet", ref="Fleet analytics dashboard (navy, panels glowing in their color)", chrome="dark",
       names=dict(fr="Tableau de bord", en="Dashboard", es="Panel", ar="لوحة القيادة"),
       swatch=["#0b1630", "#10203f", "#2fe07f", "#2f9bff"],
       c=dict(bg="#0b1630", surface="#10203f", text="#f2f6ff", dim="#a9b8d6", faint="#4c5f86", line="#22355e",
              green="#2fe07f", blue="#2f9bff", pink="#ff4f7a", orange="#ffb02e", purple="#9a7bff", yellow="#ffd23f",
              accent="#2f9bff", accent_ink="#85c2ff", secondary="#2fe07f", secondary_ink="#7aeeb0",
              ok="#2fe07f", ok_ink="#7aeeb0", warn="#ffb02e", warn_ink="#ffcf7a", danger="#ff4f7a", danger_ink="#ff90a8",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(47 155 255 / 0.34)", mark_active_text="#04260f", mark_active_bg="#2fe07f"),
       title_color="#ffffff", title_shadow="drop-shadow(0 0 14px rgb(47 155 255 / 0.5))",
       fonts=dict(brand="Inter", display="Inter", body="Inter", mono="JetBrains Mono"),
       m=dict(radius="14px", bw="2px", mix=100, glow="0 0 26px -8px var(--k, transparent)", tint=8, highlight="rgb(255 255 255 / 0.06)"),
       page="radial-gradient(circle at 1px 1px, rgb(47 155 255 / 0.13) 1px, transparent 1.6px) 0 0 / 22px 22px, #0b1630"),

  dict(id="command", ref="Online income command center (navy-black, cyan neon, uppercase)", chrome="dark", case="uppercase",
       names=dict(fr="Centre de commande", en="Command center", es="Centro de mando", ar="مركز القيادة"),
       swatch=["#050b14", "#0a1626", "#19d3ff", "#ffd23f"],
       c=dict(bg="#050b14", surface="#0a1626", text="#e6f6ff", dim="#9fc0d6", faint="#3f6a85", line="#12324a",
              green="#39e58c", blue="#19d3ff", pink="#ff4fa3", orange="#ff9e2c", purple="#a46bff", yellow="#ffd23f",
              accent="#19d3ff", accent_ink="#6ee4ff", secondary="#ffd23f", secondary_ink="#ffe17a",
              ok="#39e58c", ok_ink="#7af0b5", warn="#ff9e2c", warn_ink="#ffc373", danger="#ff4f6a", danger_ink="#ff8ea0",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(25 211 255 / 0.3)", mark_active_text="#241a00", mark_active_bg="#ffd23f"),
       title_color="#ffcc33", title_shadow="drop-shadow(0 0 12px rgb(255 204 51 / 0.55))",
       fonts=dict(brand="Rajdhani", display="Rajdhani", body="Rajdhani", mono="JetBrains Mono"),
       sizes=dict(body="17px"),
       m=dict(radius="8px", bw="1px", fixed="rgb(25 211 255 / 0.5)", glow="0 0 14px -6px rgb(25 211 255 / 0.65)"),
       page="radial-gradient(circle at 50% -10%, rgb(25 211 255 / 0.18), transparent 40%), linear-gradient(rgb(25 211 255 / 0.04) 1px, transparent 1px) 0 0 / 40px 40px, linear-gradient(90deg, rgb(25 211 255 / 0.04) 1px, transparent 1px) 0 0 / 40px 40px, #050b14"),

  dict(id="telemetry", ref="Airframe system status (drone telemetry: dark terrain, translucent cards, green nominal, orange warn)",
       chrome="dark", case="uppercase",
       names=dict(fr="Télémétrie", en="Telemetry", es="Telemetría", ar="القياس عن بعد"),
       swatch=["#141716", "#1c201f", "#7fd88f", "#ff7a45"],
       c=dict(bg="#121514", surface="#1c201f", text="#e7ece8", dim="#a3ada7", faint="#4f5853", line="#2f3532",
              green="#7fd88f", blue="#6fb3e0", pink="#e88aa0", orange="#ff7a45", purple="#a79be0", yellow="#e6d36a",
              accent="#7fd88f", accent_ink="#9fe6a9", secondary="#ff7a45", secondary_ink="#ff9d72",
              ok="#7fd88f", ok_ink="#9fe6a9", warn="#ff7a45", warn_ink="#ff9d72", danger="#ff5a5a", danger_ink="#ff8f8f",
              query="#ffffff", mark_text="#ffffff", mark_bg="rgb(127 216 143 / 0.3)", mark_active_text="#0a1f0f", mark_active_bg="#7fd88f"),
       title_color="#e7ece8", title_shadow="drop-shadow(0 6px 18px rgb(0 0 0 / 0.55))",
       fonts=dict(brand="IBM Plex Sans", display="IBM Plex Sans", body="IBM Plex Sans", mono="JetBrains Mono"), rail="300px",
       m=dict(radius="10px", bw="1px", mix=25, highlight="rgb(255 255 255 / 0.05)", drop="0 18px 34px -20px rgb(0 0 0 / 0.7)"),
       page="radial-gradient(ellipse at 50% 62%, rgb(92 102 78 / 0.38), transparent 58%), radial-gradient(ellipse at 20% 30%, rgb(70 76 66 / 0.3), transparent 45%), linear-gradient(180deg, #1b1f1d, #0d0f0e)",
       extra="[data-theme='telemetry'] .title { font-weight: 600; letter-spacing: 6px; }"),

  dict(id="onyx", ref="Payro (finance app: pure black, dark cards, colored categories, amber and teal)", chrome="dark",
       names=dict(fr="Onyx", en="Onyx", es="Ónix", ar="جزع"),
       swatch=["#0b0b0c", "#18181a", "#f5b83d", "#2cc5b4"],
       c=dict(bg="#0b0b0c", surface="#18181a", text="#f4f4f5", dim="#a1a1aa", faint="#52525b", line="#27272a",
              green="#34c77b", blue="#4c8dff", pink="#e8664a", orange="#f08a3c", purple="#a78bfa", yellow="#f5b83d",
              accent="#f5b83d", accent_ink="#f8cc6e", secondary="#2cc5b4", secondary_ink="#5fdccd",
              ok="#34c77b", ok_ink="#6ee0a3", warn="#f5b83d", warn_ink="#f8cc6e", danger="#e8664a", danger_ink="#f08f78",
              query="#ffffff", mark_text="#241800", mark_bg="#f5b83d", mark_active_text="#022622", mark_active_bg="#2cc5b4"),
       title_color="#ffffff", title_shadow="drop-shadow(0 6px 16px rgb(0 0 0 / 0.6))",
       fonts=dict(brand="DM Sans", display="DM Sans", body="DM Sans", mono="JetBrains Mono"),
       m=dict(radius="18px", bw="1px", mix=22, highlight="rgb(255 255 255 / 0.04)"),
       page="#0b0b0c"),
]

# ── Colors and contrast ──────────────────────────────────────────────────
def parse(c):
    c = c.strip()
    if c.startswith("#"):
        h = c[1:]
        if len(h) == 3:
            h = "".join(ch * 2 for ch in h)
        return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4)), 1.0
    m = re.match(r"rgb\((\d+) (\d+) (\d+)(?: / ([\d.]+))?\)", c)
    if m:
        return (int(m[1]), int(m[2]), int(m[3])), float(m[4] or 1)
    return None, None

def over(fg, bg):
    (r, a), (b, _) = parse(fg), parse(bg)
    return tuple(round(a * x + (1 - a) * y) for x, y in zip(r, b))

def lum(rgb):
    def ch(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (ch(v) for v in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b

def ratio(a, b):
    la, lb = lum(a), lum(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)

def rgba(hexc, a):
    (r, g, b), _ = parse(hexc)
    return f"rgb({r} {g} {b} / {a})"

def check(spec):
    c = spec["c"]
    surf = parse(c["surface"])[0]
    bg = parse(c["bg"])[0]
    out = []
    def need(label, fg, back, minimum=4.5):
        if parse(fg)[0] is None or parse(back)[0] is None:
            return
        f = over(fg, back) if parse(fg)[1] < 1 else parse(fg)[0]
        r = ratio(f, parse(back)[0])
        if r < minimum:
            out.append(f"{label} {r:.1f}:1")
    for k in ("text", "dim", "accent_ink", "secondary_ink", "ok_ink", "warn_ink", "danger_ink", "query"):
        need(k, c[k], c["surface"])
    need("text on page", c["text"], c["bg"])
    need("dim on page", c["dim"], c["bg"])
    mb = c["mark_bg"]
    if parse(mb)[0] is not None:
        mbg = "#%02x%02x%02x" % over(mb, c["surface"]) if parse(mb)[1] < 1 else mb
        need("mark", c["mark_text"], mbg)
    if parse(c["mark_active_bg"])[0] is not None:
        need("mark active", c["mark_active_text"], c["mark_active_bg"])
    syn = syntax(spec)
    for k, v in syn.items():
        need(f"syntax {k}", v, c["surface"])
    return out

def syntax(spec):
    c = spec["c"]
    return dict(kw=c["secondary_ink"] if spec["chrome"] == "dark" else c["accent_ink"], str=c["ok_ink"], com=c["dim"], num=c["warn_ink"])

# ── CSS ──────────────────────────────────────────────────────────────────
def css(spec):
    t, c, m, f = spec["id"], spec["c"], spec["m"], spec["fonts"]
    dark = spec["chrome"] == "dark"
    a = (lambda light, darkv: darkv if dark else light)
    fills = dict(accent=rgba(c["accent"], a(0.22, 0.22)), accent_strong=rgba(c["accent"], a(0.42, 0.42)),
                 secondary=rgba(c["secondary"], a(0.22, 0.2)), ok=rgba(c["ok"], a(0.26, 0.2)),
                 warn=rgba(c["warn"], a(0.6, 0.35)), yellow=rgba(c["yellow"], a(0.45, 0.24)))
    syn = syntax(spec)
    q = lambda fam, fallback: f"'{fam}', {fallback}"
    ar = spec.get("arabic", {})
    sizes = spec.get("sizes", {})
    border = m.get("fixed") or f"color-mix(in srgb, var(--k, var(--line)) {m.get('mix', 30)}%, var(--line))"
    shadows = [s for s in (m.get("glow"), f"inset 0 1px 0 {m['highlight']}" if m.get("highlight") else None, m.get("inset") or None, m.get("drop")) if s]
    tint = f"color-mix(in srgb, var(--k, transparent) {m['tint']}%, transparent)" if m.get("tint") else "none"
    title = f"  --title-color: {spec['title_color']};\n" if spec.get("title_color") else ""
    rail = f"  --rail-w: {spec['rail']};\n" if spec.get("rail") else ""
    lines = f"""/*
 * Theme "{spec['names']['en']}" ({spec['chrome']}) — after a reference image sent by the user on
 * 2026-10-15: {spec['ref']}. Generated by scripts/themegen.py, then checked on
 * screenshots (3 layouts, FR and AR). Contrast checked: text and small colored text
 * at least 4.5:1 on the panels.
 */
:root[data-theme='{t}'] {{
  --bg: {c['bg']};
  --surface: {c['surface']};
  --text: {c['text']};
  --text-dim: {c['dim']};
  --text-faint: {c['faint']}; /* decoration only */
  --line: {c['line']};

  --green: {c['green']};
  --blue: {c['blue']};
  --pink: {c['pink']};
  --orange: {c['orange']};
  --purple: {c['purple']};
  --yellow: {c['yellow']};

  --accent: {c['accent']};
  --accent-ink: {c['accent_ink']};
  --secondary: {c['secondary']};
  --secondary-ink: {c['secondary_ink']};
  --ok: {c['ok']};
  --ok-ink: {c['ok_ink']};
  --warn: {c['warn']};
  --warn-ink: {c['warn_ink']};
  --danger: {c['danger']};
  --danger-ink: {c['danger_ink']};

  --query-ink: {c['query']};

  --mark-text: {c['mark_text']};
  --mark-bg: {c['mark_bg']};
  --mark-active-text: {c['mark_active_text']};
  --mark-active-bg: {c['mark_active_bg']};

  --fill-accent: {fills['accent']};
  --fill-accent-strong: {fills['accent_strong']};
  --fill-secondary: {fills['secondary']};
  --fill-ok: {fills['ok']};
  --fill-warn: {fills['warn']};
  --fill-yellow: {fills['yellow']};

  --shadow-pop: {a('0 22px 44px -20px rgb(30 30 60 / 0.4)', '0 0 0 1px rgb(255 255 255 / 0.06), 0 26px 48px -20px rgb(0 0 0 / 0.85)')};
  --tilt: 0deg;
  --doodle-opacity: {spec.get('doodles', 0)};
  --sweep-bg: linear-gradient(90deg, transparent, {rgba(c['accent'], 0.14)} 70%, {rgba(c['accent'], 0.5)} 98%, transparent);
  --ping-bg: linear-gradient(90deg, {rgba(c['accent'], 0.32)}, transparent 70%);

{title}  --title-stroke: 0px;
  --title-mix: #fff;
  --title-mix-pct: 100%;
  --title-shadow: {spec['title_shadow']};

  --display-case: {spec.get('case', 'none')};

  --syn-kw: {syn['kw']};
  --syn-str: {syn['str']};
  --syn-com: {syn['com']};
  --syn-num: {syn['num']};

  --font-brand: {q(f['brand'], "'Baloo Bhaijaan 2', sans-serif")};
  --font-display: {q(f['display'], "'Baloo Bhaijaan 2', sans-serif")};
  --font-body: {q(f['body'], "'Baloo Bhaijaan 2', sans-serif")};
  --font-mono: {q(f['mono'], "'Baloo Bhaijaan 2', monospace")};
  --fs-body: {sizes.get('body', '16px')};
  --lh-body: 24px;
{rail}
  --stroke: {m.get('bw', '1px')};
  --r-sketch: {m.get('radius', '16px')};
  --wobble: opacity(1);
  --focus-ring: 2px solid var(--accent);

  color-scheme: {spec['chrome']};
}}

/* Arabic: {ar.get('body', 'Baloo Bhaijaan 2')} leads; Latin words fall back to {f['body']}. */
:root[data-theme='{t}']:lang(ar) {{
  --font-brand: {q(ar.get('brand', 'Baloo Bhaijaan 2'), q(f['brand'], 'sans-serif'))};
  --font-display: {q(ar.get('body', 'Baloo Bhaijaan 2'), q(f['display'], 'sans-serif'))};
  --font-body: {q(ar.get('body', 'Baloo Bhaijaan 2'), q(f['body'], 'sans-serif'))};
}}

/* ── Treatments (scoped to this theme) ─────────────────────────────── */

[data-theme='{t}'] .sketch {{
  border-radius: var(--r-sketch);
}}

[data-theme='{t}'] .sketch.round {{
  border-radius: 50%;
}}

[data-theme='{t}'] .sketch::before {{
  content: '';
  position: absolute;
  inset: 0;
  z-index: 0;
  border: var(--sw, var(--stroke)) solid {border};
  border-radius: inherit;
  background: {tint};
  box-shadow: {', '.join(shadows) if shadows else 'none'};
  pointer-events: none;
}}

[data-theme='{t}'] .sketch.dashed::before {{
  border-style: dashed;
}}

[data-theme='{t}'] .hatch {{
  background-image: linear-gradient(var(--h, var(--fill-yellow)), var(--h, var(--fill-yellow)));
}}

[data-theme='{t}'] body {{
  background: {spec['page']};
  background-attachment: fixed;
}}
"""
    if spec.get("extra"):
        lines += "\n" + spec["extra"] + "\n"
    return lines

# ── Writing ──────────────────────────────────────────────────────────────
def json_update(path, fn):
    raw = path.read_text(encoding="utf-8")
    data = json.loads(raw)
    fn(data)
    out = json.dumps(data, ensure_ascii=False, indent=2)
    if "\r\n" in raw:
        out = out.replace("\n", "\r\n")
    if raw.endswith("\n") and not out.endswith("\n"):
        out += "\r\n" if "\r\n" in raw else "\n"
    path.write_text(out, encoding="utf-8")

def main():
    problems = 0
    for spec in SPECS:
        bad = check(spec)
        print(f"{spec['id']:<11} {'OK' if not bad else 'CONTRAST: ' + ', '.join(bad)}")
        problems += len(bad)
        (UI / f"styles/themes/{spec['id']}.css").write_text(css(spec), encoding="utf-8")

    # Font imports, between markers.
    p = UI / "styles/fonts.css"; s = p.read_text(encoding="utf-8")
    start = "/* ── Themes from the user's reference images (themegen.py) ── */"
    s = s.split("\n" + start)[0].rstrip("\n")
    imports = [start]
    for fam, (pkg, weights) in FONTS.items():
        for w in weights:
            imports.append(f"@import '@fontsource/{pkg}/latin-{w}.css';")
        for w in weights:
            if (ROOT / f"node_modules/@fontsource/{pkg}/latin-ext-{w}.css").exists():
                imports.append(f"@import '@fontsource/{pkg}/latin-ext-{w}.css';")
    for fam, (pkg, weights) in ARABIC_FONTS.items():
        for w in weights:
            imports.append(f"@import '@fontsource/{pkg}/arabic-{w}.css';")
    s += "\n\n" + "\n".join(imports) + "\n"
    p.write_text(s, encoding="utf-8")

    # Stylesheets.
    p = UI / "main.ts"; s = p.read_text(encoding="utf-8")
    for spec in SPECS:
        line = f"import './styles/themes/{spec['id']}.css';"
        if line not in s:
            s = s.replace("import './styles/base.css';", line + "\nimport './styles/base.css';")
    p.write_text(s, encoding="utf-8")

    # THEMES list: the existing ones, then these.
    p = UI / "lib/stores/ui.svelte.ts"; s = p.read_text(encoding="utf-8")
    m = re.search(r"export type Theme =([^;]+);", s)
    ids = [x.strip().strip("'") for x in m[1].split("|") if x.strip()]
    for spec in SPECS:
        if spec["id"] not in ids:
            ids.append(spec["id"])
    s = s.replace(m[0], "export type Theme =\n  | " + "\n  | ".join(f"'{i}'" for i in ids) + ";")
    end = s.index("];", s.index("export const THEMES"))
    block = s[s.index("export const THEMES"):end]
    for spec in SPECS:
        if f"id: '{spec['id']}'" not in block:
            sw = ", ".join(f"'{x}'" for x in spec["swatch"])
            block += f"  {{ id: '{spec['id']}', chrome: '{spec['chrome']}', swatch: [{sw}] }},\n"
    s = s[:s.index("export const THEMES")] + block + s[end:]
    p.write_text(s, encoding="utf-8")

    for lang in ("fr", "en", "es", "ar"):
        json_update(UI / f"lib/i18n/locales/{lang}.json",
                    lambda d, lang=lang: d["settings"]["themeNames"].update({sp["id"]: sp["names"][lang] for sp in SPECS}))
    print(f"{len(SPECS)} themes written, {problems} contrast problem(s)")

if __name__ == "__main__":
    main()
