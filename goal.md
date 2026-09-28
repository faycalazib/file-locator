# Prospector — Outil de recherche full-text local (spec complète)

> 📊 **Tableau de suivi (fait / reste à faire) : [`docs/SUIVI.md`](docs/SUIVI.md)**
>
> À donner tel quel à Claude Code. Ce document est la spec de référence : stack, fonctionnalités, design system, plan de développement. Mets à jour les cases à cocher au fil de l'avancement pour garder le suivi.

## 0. Contexte & objectif

Construire un outil desktop cross-platform (Windows/Mac/Linux) de recherche full-text dans des fichiers locaux — équivalent de FileLocator Pro / Agent Ransack, mais plus rapide, plus moderne, et distribuable gratuitement à un public non-technique (pas de Python à installer, pas de ligne de commande requise, double-clic pour lancer).

Public cible : créateurs de contenu et utilisateurs grand public qui veulent retrouver rapidement du contenu dans leurs fichiers (documents, PDF, emails, code) sans compétence technique.

## 1. Stack technique (imposée)

- **Langage moteur** : Rust
- **Indexation / recherche** : Tantivy (équivalent Lucene, recherche floue native, très rapide)
- **Shell applicatif** : Tauri (binaire léger, cross-platform, frontend HTML/CSS/JS + backend Rust)
- **Extraction de contenu** : crates Rust natives en priorité (PDF, DOCX, XLSX, PPTX, ZIP) ; fallback vers un binaire local (type Apache Tika ou `readpst`) pour les formats exotiques (PST/MSG Outlook)
- **Frontend** : HTML/CSS/JS vanilla ou framework léger (Svelte recommandé — bundle minimal, cohérent avec l'objectif "binaire léger")

Ne pas proposer Electron, Python, ou toute stack nécessitant un runtime externe à installer par l'utilisateur final.

## 2. Architecture

```
core/ (Rust)
├── crawler/        parcours récursif, filtres (taille, date, extensions, exclusions)
├── extractors/      un module par famille de format
│   ├── text.rs       txt, code source, logs
│   ├── pdf.rs
│   ├── office.rs      docx, xlsx, pptx
│   ├── archive.rs     zip, rar, 7z (récursif dans les archives)
│   └── email.rs       pst, msg (via binaire externe si besoin)
├── index/            wrapper Tantivy : création, update incrémental, requêtes
├── watcher/          surveillance des dossiers indexés (notify crate), réindexation live
└── api/              commandes Tauri exposées au frontend

ui/ (Tauri frontend)
├── components/
│   ├── SearchBar
│   ├── ResultsList
│   ├── PreviewPanel
│   ├── IndexRail        (gestion des sources/index, "sites de fouille")
│   └── FilterPanel
└── styles/
    └── tokens.css      (voir section 5, valeurs figées)
```

## 3. Fonctionnalités — Phase MVP (parité FileLocator Pro)

- [x] Recherche par mot-clé, phrase exacte, opérateurs booléens (AND/OR/NOT, `-mot`) — Étape 1
- [x] Recherche par expression régulière (`/…/` dans la requête ou pastille `.*`) — Étape 2
- [x] Formats supportés : txt, code source (avec coloration syntaxique en preview), PDF, DOCX, XLSX, PPTX, ZIP (+ .xls/.ods, .eml, .msg, .pst)
- [x] Surlignage des correspondances en contexte dans les résultats (offsets sur le texte original)
- [x] Mode recherche indexée (instantané) vs non-indexée (scan à la volée, résultats en direct)
- [x] Filtres : type de fichier, taille, date de modification, langue, dossiers exclus
- [x] Panneau d'aperçu du document sélectionné avec navigation entre occurrences
- [x] Export des résultats (CSV, JSON) via la boîte « Enregistrer sous » native
- [x] Recherche multi-dossiers en une seule requête (plusieurs sites de fouille cochés)

## 4. Fonctionnalités — Phase différenciation (dépasser FileLocator Pro)

- [x] Recherche tolérante aux fautes de frappe (natif Tantivy ; correspondances approchantes signalées)
- [x] Indexation incrémentale en tâche de fond : un dossier indexé reste à jour automatiquement (file watcher), pas besoin de relancer un scan complet
- [x] Formats étendus : PST/MSG Outlook, RAR/7z, archives imbriquées (PST/MSG et ZIP à l'Étape 2 ; RAR 4/5 et 7z à l'Étape 3)
- [x] Aperçu riche : rendu réel des tableaux Excel (pas juste texte brut extrait), rendu des slides PPTX
- [x] Raccourci global façon Spotlight/Raycast (ouverture instantanée depuis n'importe où, ex. Ctrl/Cmd+Espace)
- [x] Recherches sauvegardées / favoris (★ : requête, mode, options, filtres et sites ; stockées dans le dossier des index)
- [ ] Multi-index géré en parallèle, avec bascule rapide entre "sites de fouille" — un index par site, cases à cocher ; groupes et bascule rapide au lot 7.2

### 4bis. Dépasser FileLocator Pro (ajouté le 27/09/2026, détail dans docs/COMPARATIF-FILELOCATOR.md)

- [ ] Étape 5 : critère nom de fichier, OCR, anciens formats, pièces jointes, NEAR, travail sur les résultats, intégration Windows, dates / attributs / empreintes
- [ ] Étape 6 : alertes, détecteurs, compteurs, rapport par mot-clé, doublons, images, extraction, mode portable
- [ ] Étape 7 : ligne de commande, groupes de sites, index sur le réseau, critères depuis un fichier
- [ ] Étape 8 : IA locale optionnelle (recherche par le sens et entre langues)
- [x] Rapport exportable (PDF) résumant une recherche avec extraits (boîte d'impression → « Enregistrer au format PDF »)
- [ ] Mise à jour automatique de l'application (auto-update Tauri)
- [x] 100% gratuit, aucune licence à saisir pour l'utilisateur final

## 5. Système de design — thème « Crayons de couleur »

**Décision du 2026-09-24.** Après deux revues de l'Étape 0 (« interface fade, trop dense, rien d'innovant »), le poste de contrôle navy a été abandonné. L'utilisateur a fourni 8 images de référence (dans `docs/references/`). Une maquette a été produite par style (`docs/mockups/`, galerie `index.html`), et le style **G · Crayons de couleur** a été retenu. La palette et les polices sont désormais libres, mais **figées dans le thème** : aucune couleur ni police en dehors de `ui/src/styles/themes/crayons.css`. Un **switch de thème** est prévu (voir `docs/THEMES.md`).

**Concept directeur.** C'est le carnet de fouille de l'utilisateur : papier blanc, traits d'encre tremblés à la main, hachures au crayon de couleur, doodles. Le public (créateurs de contenu, grand public) doit se sentir chez lui, pas dans un outil de développeur.

### Palette (thème `crayons`)

| Rôle | Nom | Hex | Texte associé (≥ 4,5:1) |
| --- | --- | --- | --- |
| Fond (papier) | Paper | `#FFFEFA` | — |
| Panneaux | Sheet | `#FFFFFF` | — |
| Texte, traits d'encre | Ink | `#262431` | — |
| Texte secondaire | Graphite | `#5C5968` | — |
| Crayon accent (correspondances, compteurs) | Pink | `#EE5A8E` | `#B8305F` |
| Crayon secondaire (sélection, liens) | Blue | `#3B9BE0` | `#1D68A8` |
| Crayon OK (suivi, aperçu) | Green | `#6CC04A` | `#37801D` |
| Crayon attention (indexation) | Orange | `#F39A2B` | `#9A5A06` |
| Crayon décoratif | Purple | `#9A6BD8` | — |
| Surligneur (correspondances) | Yellow | `#F5C93A` | — |
| Erreur | Red | `#E2474B` | `#B3262B` |

Les couleurs vives ne servent qu'aux traits, aux hachures et aux grands textes. Les petits textes utilisent les variantes `*-ink`.

### Typographie

- Titres, étiquettes, boutons : **Patrick Hand SC** (capitales dessinées à la main)
- Texte courant : **Patrick Hand**. Architects Daughter, prévue dans la maquette, a été écartée : son « 1 » ressemblait à un « l ».
- Chemins, code, regex : **Courier Prime** (machine à écrire)
- Arabe : **Baloo Bhaijaan 2**, placée en tête de pile quand `lang="ar"`
- Toutes les polices sont embarquées localement (Fontsource, licence OFL) : aucun CDN, l'application fonctionne hors ligne
- Échelle : display 56/60 (marque), titre 26/30, h2 21/26, texte 17/24, légende 13/18

### Traitements

- `.sketch` : contour d'encre qui tremble (filtre SVG `#wobble`, défini dans `ThemeDefs.svelte`). Seul le trait est filtré, le texte reste net.
- `.hatch` : remplissage hachuré au crayon (`--h` = couleur).
- Correspondances : surligneur hachuré jaune ; l'occurrence active est en rose.
- Doodles : icônes Microsoft Fluent Emoji « Color » (licence MIT), embarquées dans `ui/src/assets/doodles/`.

### Layout

```text
                 [ MON CARNET DE FOUILLE ]
                       P R O S P E C T O R
        ( JE CHERCHE : ______________________ )  [FOUILLER !]
 (✓ tolérance) (Aa)(ab)(.*) | (PDF)(WORD)… (+ AFFINER) | (INDEXÉE)(SCAN DIRECT)
┌─ SITES ─────┐  30 TROUVAILLES DANS 11 FICHIERS      ┌─ APERÇU ───────────┐
│ ☑ Clients   │  ┌──────────────────────────────┐ (7)  │ CONTRAT-…PDF       │
│ ☑ Mail ▒▒▒  │  │ CONTRAT-PRESTATION-V3.PDF    │      │ Le présent ▒contrat│
├─ RÉCENTS ───┤  │ …le présent ▒contrat▒ …      │      │ …                  │
├─ FAVORIS ───┤  └──────────────────────────────┘      │ [OUVRIR] ↑ 1/7 ↓   │
└─────────────┘                                        └────────────────────┘
```

Trois dispositions sont proposées dans Réglages → Mise en page : **Journal** (par défaut), **Registre** (tableau, filtres dans le rail) et **Strates** (résultats groupés par dossier, rail compact).

### Mouvement

- Pendant une recherche, une bande de hachures roses balaie les résultats.
- Chaque nouvelle trouvaille reçoit un bref « coup de surligneur » jaune.
- Aucune autre animation systématique ; tout est coupé sous `prefers-reduced-motion`.

### Principes

- Le contenu trouvé est la vedette : les extraits restent lisibles, et un chiffre ne doit jamais pouvoir être confondu avec une lettre
- Clarté plutôt que densité : filtres secondaires repliés derrière « Affiner », un seul extrait par résultat, métadonnées dans l'aperçu
- Contraste WCAG AA, focus clavier toujours visible (pointillés bleus)
- Thème clair « Crayons » par défaut ; thème sombre « Sonar » au choix (Réglages → Thème), qui aligne aussi la barre de titre native de la fenêtre

## 5bis. Internationalisation (obligatoire dès l'Étape 0)

Langues minimales : **anglais (`en`, langue de référence et de repli), français (`fr`), espagnol (`es`), arabe (`ar`, écrit de droite à gauche)**. Pour ajouter une langue, il suffit d'ajouter un fichier JSON.

### A. Interface

- **Aucun texte en dur** dans les composants : tout passe par des clés (`results.matches`) dans `ui/src/lib/i18n/locales/{en,fr,es,ar}.json`
- Runtime maison léger (store Svelte + `Intl.PluralRules`, `Intl.NumberFormat`, `Intl.DateTimeFormat`, `Intl.RelativeTimeFormat`), sans dépendance externe
- Pluriels selon les catégories ICU (`zero/one/two/few/many/other` ; l'arabe utilise les six)
- Au premier lancement, la langue du système est détectée (repli sur `en`) ; un sélecteur est disponible dans les réglages et le choix est conservé localement
- **RTL** : `<html lang dir>` suit la langue choisie. Le CSS utilise uniquement des propriétés logiques (`margin-inline-start`, `border-inline-start`, `inset-inline`…). En arabe, le layout 3 zones est inversé (rail à droite), et le balayage des résultats change de sens
- Les chemins, le code et les regex restent toujours en `dir="ltr"` avec `unicode-bidi: isolate`. Les extraits de documents sont en `dir="auto"`
- Le backend Rust ne renvoie **jamais** de texte traduit, seulement des codes d'erreur ou d'état (ex. `FileLocked { path }`), que l'UI traduit. Seuls les textes natifs (menus OS, tray) sont traduits côté Rust, à partir des mêmes JSON
- Contrôle qualité : `pnpm i18n:check` détecte les clés manquantes ou orphelines et les placeholders incohérents entre les 4 langues
- Distribution : installeurs multilingues (WiX/NSIS en-US, fr-FR, es-ES, ar-SA), page de téléchargement et guide utilisateur dans les 4 langues

### B. Recherche dans du contenu multilingue

- La langue de chaque document est détectée (`whatlang`) et stockée dans un champ `lang` filtrable
- Champ `body` avec un analyzer générique : tokenizer Unicode, minuscules, suppression des accents (« resume » trouve « résumé », « cancion » trouve « canción »)
- Champ `body_stem` avec un analyzer adapté à la langue : stemmers Snowball FR/ES/EN ; pour l'arabe, suppression du tashkeel et du tatweel, normalisation alef/ya/ta marbuta, puis light stemming
- Les requêtes portent sur les deux champs, avec un boost pour la correspondance exacte. La recherche floue s'applique sur `body`
- Le surlignage est calculé sur les offsets du texte original, pas du texte normalisé
- Des fixtures de test FR/ES/AR sont prévues (accents, pluriels, arabe avec et sans voyelles)

## 6. Plan de développement / suivi

### Étape 0 — UI statique avec données factices

- [x] Squelette Tauri + Svelte
- [x] `tokens.css` (structure) + thème `themes/crayons.css` (section 5)
- [x] Polices du thème (Patrick Hand SC, Patrick Hand, Courier Prime, Baloo Bhaijaan 2) embarquées localement
- [x] Runtime i18n + fichiers `en/fr/es/ar` + sélecteur de langue + bascule `dir` RTL
- [x] SearchBar, ResultsList, PreviewPanel, IndexRail, FilterPanel montés avec données mockées (dont extraits FR/ES/AR)
- [x] 3 mises en page (Journal, Registre, Strates) proposées dans les réglages
- [x] 8 maquettes de style à partir des références ; choix : G · Crayons de couleur
- [x] Thème « Crayons de couleur » implémenté, avec captures dans `docs/screenshots/crayons-*.png` (FR, AR/RTL, ES)
- [x] Thème « Crayons de couleur » validé par l'utilisateur dans la fenêtre Tauri (2026-09-24)
- [x] Switch de thème (Réglages → Thème) : « Crayons de couleur » (clair, par défaut) et « Sonar » (sombre, néon), avec captures `docs/screenshots/sonar-*.png`
- [x] Validation finale de l'utilisateur avant l'Étape 1 (2026-09-24)

### Étape 1 — Moteur de base

- [x] Crawler récursif parallèle avec exclusions (chemins et motifs `**\x`, dossiers système exclus d'office)
- [x] Extracteurs : txt, code, e-mail .eml (détection d'encodage), PDF, DOCX, avec protection contre les fichiers corrompus
- [x] Intégration Tantivy : un index par site de fouille, indexation parallèle, recherche
- [x] Analyzers multilingues (`body` sans accents ni voyelles arabes + `stem_en|fr|es|ar`) + détection de langue + tests FR/ES/AR
- [x] Requêtes : mots, « phrases », AND/OR/NOT, tolérance aux fautes ; surlignage exact, extraits, aperçu
- [x] Connexion UI ↔ backend (commandes Tauri) : sites de fouille, indexation en arrière-plan avec progression, recherche, aperçu, export, ouvrir / afficher dans le dossier
- [x] Dossier des index réglable (AppData par défaut, déplaçable avec ses sites) — Réglages → Dossier des index
- [x] Erreurs backend renvoyées sous forme de codes, traduites côté UI (4 langues)
- [x] Tests : 22 tests unitaires + 11 tests de bout en bout sur `test_fixtures/` + test de performance 100k fichiers
- [x] Validation de l'utilisateur dans la fenêtre Tauri (24/09/2026, après correction de BUG-019 et BUG-020)

### Étape 2 — Formats avancés

- [x] XLSX (+ .xls, .ods), PPTX, ZIP (récursif, protégé contre les bombes ZIP)
- [x] PST/MSG Outlook (sans outil externe : `outlook-pst` de Microsoft, `msg_parser`)
- [x] Recherche régulière + opérateurs booléens ; options Aa (casse) et ab (mot entier)
- [x] Scan direct (recherche sans index, résultats en direct)
- [x] Coloration syntaxique du code dans l'aperçu
- [x] Lecture séquentielle adaptée aux disques durs et USB (BUG-025)
- [ ] Validation de l'utilisateur dans la fenêtre Tauri

### Étape 3 — Différenciateurs

- [x] File watcher + indexation incrémentale (journal par site, rattrapage au démarrage, statut « Suivi en direct »)
- [x] Recherche floue (Étape 1, seuils stricts, BUG-019)
- [x] Raccourci global type Spotlight (Ctrl+Maj+Espace par défaut, modifiable ou désactivable dans les Réglages)
- [x] Recherches sauvegardées / favoris (★ : requête, mode, options, filtres et sites ; stockées dans le dossier des index)
- [x] Aperçu riche : tableaux Excel, diapositives PowerPoint
- [x] Rapport PDF d'une recherche (impression → « Enregistrer au format PDF »)
- [x] RAR 4/5 (UnRAR officiel) et 7z (Rust pur), archives imbriquées
- [ ] Validation de l'utilisateur dans la fenêtre Tauri (voir docs/TESTS-A-FAIRE.md)

### Étape 4 — Packaging & distribution

- [x] Bundler Tauri pour Windows (.exe par utilisateur + 4 MSI, EN/FR/ES/AR) — macOS (.dmg, notarisation) et Linux (.AppImage/.deb) plus tard
- [ ] Signature de code (éviter les alertes SmartScreen/Gatekeeper qui feraient fuir les abonnés) — reportée (choix du 27/09/2026) ; la page de téléchargement explique l'avertissement
- [ ] Auto-update configuré — code prêt ; clé et dépôt GitHub à fournir (docs/RELEASE.md)
- [x] Installeurs multilingues (en, fr, es, ar)
- [x] Page de téléchargement simple + mini doc utilisateur en EN/FR/ES/AR (`site/`, à publier sur GitHub Pages)

## 7. Critères d'acceptation

- Recherche indexée : résultats en moins de 200ms sur un index de 100k fichiers
- Indexation initiale : moins de 5 minutes pour 50 Go de documents mixtes sur un poste standard
- Application lancée en moins de 2 secondes
- Aucune dépendance externe à installer par l'utilisateur final
- Fonctionne sans connexion internet (recherche 100% locale)
- Interface entièrement disponible en EN/FR/ES/AR, sans aucun texte en dur, avec un RTL correct en arabe
- Une recherche sans accents ni voyelles arabes trouve les formes accentuées ou voyellées

## 8. Instructions de travail pour Claude Code

1. Commencer par l'Étape 0 uniquement (UI statique), proposer des maquettes (captures faites soi-même, sans serveur : Chrome headless sur `file://`), prendre des captures d'écran, s'arrêter et attendre validation avant de continuer.
2. Ne jamais improviser une couleur ou une police hors de la section 5.
3. Avancer étape par étape dans l'ordre de la section 6, cocher les cases au fur et à mesure, ne pas paralléliser les étapes.
4. Écrire des tests pour chaque extracteur de format (un fichier d'exemple par format dans `test_fixtures/`).
5. Committer à la fin de chaque étape avec un message clair.
