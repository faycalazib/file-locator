# Suivi du projet Prospector

> Mis à jour le 28/09/2026 : Étape 7 entièrement codée — lots 7.1 (ligne de commande), 7.2 (groupes de sites), 7.3 (index partagé) et 7.4 (liste de termes), à tester ; lots 6.1 (alertes), 6.2 (détecteurs, audit), 6.3 (compteurs), 6.4 (exports, copie), 6.5 (doublons), 6.6 (images), 6.7 (extraction) et 6.8 (mode portable) livrés, à tester : l'Étape 6 est entièrement codée ; l'Étape 5 est entièrement codée (à tester) ; Étape 4 en cours ; plan des Étapes 5 à 8 validé pour dépasser FileLocator Pro (voir plus bas).
> Ce fichier est le tableau de bord du projet. Il est mis à jour à la fin de chaque étape et après chaque correction de bug.
> La spécification de référence reste [`goal.md`](../goal.md). Les bugs sont détaillés dans [BUGS.md](BUGS.md). Les vérifications à faire dans l'application sont dans [TESTS-A-FAIRE.md](TESTS-A-FAIRE.md).

**Légende :** ✅ fait et validé · 🟢 fait, validation utilisateur en attente · 🟡 partiel · ⬜ à faire

## Vue d'ensemble

| Étape | Contenu | État | Validée le |
| --- | --- | --- | --- |
| 0 | Interface, thèmes, 4 langues | ✅ | 24/09/2026 |
| 1 | Moteur de recherche réel branché à l'interface | ✅ | 24/09/2026 |
| 2 | Excel, PowerPoint, ZIP, Outlook, expressions régulières, scan direct, coloration du code | 🟢 **à tester** | — |
| 3 | Suivi des dossiers en direct, favoris, raccourci global, aperçus riches, rapport PDF, RAR/7z | 🟢 **à tester** (6/6) | — |
| 4 | Installeurs, signature de code, mises à jour automatiques, page de téléchargement | 🟡 en cours | — |
| 5 | Combler les manques face à FileLocator Pro : nom de fichier, OCR, anciens formats, pièces jointes, NEAR, travail sur les résultats, intégration Windows, dates / attributs / empreintes | 🟢 **à tester** (8/8) | — |
| 6 | Nouveautés uniques : alertes, détecteurs, compteurs, rapport par mot-clé, doublons, images, extraction, mode portable | 🟢 **à tester** (8/8) | — |
| 7 | Automatisation et réseau : ligne de commande, groupes de sites, index sur le réseau, critères depuis un fichier | 🟢 **à tester** (4/4) | — |
| 8 | IA locale optionnelle : recherche par le sens et entre langues | ⬜ | — |

**Avancement global :**

- Phase MVP (goal.md §3) : **9/9** (en attente de ta validation).
- Phase différenciation (§4) : 8/10 faites, 1 en partie.
- Critères d'acceptation (§7) : 4/7 vérifiés.

## Phase MVP : parité avec FileLocator Pro (goal.md §3)

| Fonctionnalité | État | Détail |
| --- | --- | --- |
| Recherche par mots, « phrase exacte », AND / OR / NOT | ✅ | Aussi `-mot` et `\|`. Les opérateurs ne sont reconnus qu'en MAJUSCULES. |
| Expressions régulières | 🟢 | `/INV-\d{4}/` dans la requête, ou la pastille `.*` pour que toute la saisie soit une expression. Une expression invalide affiche une erreur traduite. |
| Casse exacte (`Aa`) et mot entier (`ab`) | 🟢 | « Mot entier » est activé par défaut. Désactivé, « contrat » trouve aussi « sous-contrats ». |
| Formats : txt, code, PDF, DOCX | ✅ | Plus les e-mails `.eml`, avec détection de l'encodage. |
| Formats : Excel (xlsx, xls, ods) | 🟢 | Toutes les feuilles, avec le nom de chaque feuille dans l'aperçu. |
| Formats : PowerPoint (pptx) | 🟢 | Texte de chaque diapositive, numérotée. |
| Formats : ZIP | 🟢 | Chaque fichier d'une archive devient un résultat (`archive.zip › dossier/fichier.pdf`). Les ZIP imbriqués sont lus jusqu'à 3 niveaux, avec une protection contre les bombes ZIP. |
| Coloration syntaxique du code dans l'aperçu | 🟢 | Mots-clés, chaînes, commentaires et nombres, aux couleurs du thème. |
| Surlignage en contexte | ✅ | Les correspondances approchantes sont signalées à part. |
| Recherche indexée (instantanée) | ✅ | 19 à 40 ms sur 100 000 fichiers. |
| Recherche non indexée (scan direct) | 🟢 | Pastille « Scan direct » : lit les dossiers sur le moment, les résultats arrivent en direct, avec un bouton Arrêter. |
| Filtres : type, taille, date, dossiers exclus | ✅ | Plus la langue du document. |
| Aperçu avec navigation entre occurrences | ✅ | Pour un document dans un ZIP ou un PST, « Ouvrir » ouvre l'archive ou la boîte mail. |
| Export CSV / JSON | ✅ | |
| Recherche multi-dossiers | ✅ | |

## Phase différenciation : dépasser FileLocator Pro (goal.md §4)

| Fonctionnalité | État | Détail |
| --- | --- | --- |
| Tolérance aux fautes de frappe | ✅ | Seuils stricts ; correspondances approchantes signalées (BUG-019). |
| Suivi des dossiers en direct + indexation incrémentale | 🟢 | Seuls les fichiers ajoutés, modifiés ou supprimés sont relus. Un fichier enregistré est retrouvable ~2 s après. Au lancement, les changements faits pendant que Prospector était fermé sont rattrapés. Un disque débranché garde ses documents dans l'index. Le site affiche « Suivi en direct ». |
| Formats étendus : Outlook PST/MSG, archives imbriquées, RAR/7z | 🟢 | PST et MSG sans outil externe (bibliothèque `outlook-pst` de Microsoft). ZIP, 7z (compressé « solide » compris) et RAR 4/5 (bibliothèque officielle UnRAR), archives imbriquées jusqu'à 3 niveaux. Les archives protégées par mot de passe sont comptées, pas lues. |
| Aperçu riche (tableaux Excel, diapositives) | 🟢 | Excel : un vrai tableau par feuille, colonnes alignées, montants à droite. PowerPoint : une carte par diapositive, numéro et titre. Les correspondances restent surlignées et navigables. |
| Raccourci global façon Spotlight | 🟢 | Ctrl+Maj+Espace par défaut : Prospector passe au premier plan, prêt à taper ; appuyé de nouveau, il se réduit. Choix dans les Réglages (3 combinaisons ou aucun). Un raccourci déjà pris par une autre application est signalé. |
| Recherches sauvegardées / favoris | 🟢 | Le bouton ★ enregistre la recherche affichée avec son nom, son mode, ses options, ses filtres et ses sites. L'étoile est pleine si elle est déjà enregistrée. La liste est dans le rail : un clic la relance, × la retire. Stockées dans le dossier des index. |
| Plusieurs index, bascule rapide | 🟢 | Un index par site, plusieurs sites cochables, groupes de sites nommés avec `Ctrl+1…9` (lot 7.2). |
| Rapport PDF | 🟢 | Téléchargement → « Rapport PDF… » (ou Ctrl+P) : critères de la recherche puis chaque fichier avec ses extraits surlignés. Enregistré via « Enregistrer au format PDF » de la boîte d'impression. Page blanche quel que soit le thème ; arabe et droite à gauche corrects. Exemples dans `docs/screenshots/report-*.pdf`. |
| Mise à jour automatique | 🟡 | Code en place (vérification au démarrage, bannière, Réglages → Mises à jour). S'active dès que la clé de mise à jour et le dépôt GitHub existent (docs/RELEASE.md). |
| 100 % gratuit, sans licence | ✅ | |

## Multilingue (goal.md §5bis)

| Fonctionnalité | État |
| --- | --- |
| Interface EN/FR/ES/AR (181 textes par langue, vérifiés), arabe de droite à gauche, pluriels arabes | ✅ |
| Langue de l'OS détectée, erreurs du moteur traduites | ✅ |
| Recherche sans accents ni voyelles arabes, pluriels regroupés à une lettre près | ✅ |
| Détection de la langue de chaque document (filtrable) | ✅ |
| Installeurs et documentation dans les 4 langues | 🟡 Installeurs .exe (choix de la langue) et 4 MSI configurés ; page de téléchargement et guide rédigés dans les 4 langues (`site/`) |

## Interface et réglages (au-delà de la spec)

| Élément | État |
| --- | --- |
| Thèmes « Crayons de couleur » (clair) et « Sonar » (sombre), 3 mises en page | ✅ |
| Dossier des index réglable | ✅ |
| Sites de fouille : ajout, réindexation, arrêt, suppression, progression | ✅ |
| Accessibilité : clavier, focus visible, animations réduites si demandé | ✅ |

## Critères d'acceptation (goal.md §7)

| Critère | État | Mesure |
| --- | --- | --- |
| Recherche < 200 ms sur 100 000 fichiers | ✅ | 19 à 40 ms |
| Indexation < 5 min pour 50 Go mixtes | 🟡 | 100 000 fichiers texte froids sur **disque dur USB** en 204 s. 50 Go mixtes (PDF, Word…) pas encore mesurés. |
| Lancement < 2 s | ⬜ | À mesurer sur la version installée (Étape 4) |
| Aucune dépendance à installer | ✅ | Même pour Outlook : pas de `readpst` à livrer |
| 100 % hors ligne | ✅ | |
| Interface EN/FR/ES/AR, RTL correct | ✅ | |
| Recherche insensible aux accents et voyelles arabes | ✅ | |

## Qualité

| Indicateur | Valeur |
| --- | --- |
| Tests Rust | 34 unitaires, 29 de bout en bout (11 pour l'Étape 1, 9 pour l'Étape 2, 9 pour l'Étape 3), 1 de performance (100 000 fichiers) et 3 de diagnostic |
| Fichiers d'exemple (`test_fixtures/`) | 27 fichiers : texte, code, PDF, Word, Excel, PowerPoint, ZIP imbriqué, 7z solide, RAR 5, RAR chiffré, bombe ZIP, .eml, .msg, .pst, fichiers corrompus |
| Interface | `pnpm check` : 0 erreur, 0 avertissement · `pnpm i18n:check` : 4 langues cohérentes |
| Rust | `cargo clippy -D warnings` : aucun avertissement |
| Bugs documentés | 35 (BUG-001 à BUG-035). BUG-022 (antivirus qui bloque brièvement un fichier d'index) reste à surveiller ; BUG-026 ajoute des nouvelles tentatives automatiques. |

## À surveiller

- **Module de sens (lot 8.1)** :
  - le téléchargement ne fonctionnera qu'une fois le ZIP joint à une version GitHub et `MODULE_URL` renseigné (voir RELEASE.md) ;
  - la version quantifiée du modèle demande un processeur avec AVX2 (PC d'après 2013 environ) : à signaler proprement si elle manque.
- **Liste de termes (lot 7.4)** : 2 000 termes en 375 ms sur un petit site ; à mesurer sur 100 000 fichiers, surtout avec la tolérance aux fautes activée (chaque mot est aussi cherché avec fautes).
- **Index partagé (lot 7.3)** :
  - testé avec deux moteurs sur le même dossier local ; reste à vérifier sur un vrai partage entre deux PC ;
  - chaque recherche lit l'index à travers le réseau : à chronométrer en réseau local, en Wi-Fi et en VPN ;
  - le relais repose sur l'heure des PC : des horloges très décalées (plus d'une minute) retarderaient ou avanceraient le relais ;
  - les alertes ne se déclenchent que sur le PC qui tient l'index ;
  - un lecteur ouvert pendant que l'autre PC réécrit tout un index (changement de version) peut garder des fichiers ouverts : Windows empêche alors leur suppression jusqu'à la prochaine recherche.
- **Mode portable sur une vraie clé USB (lot 6.8)** : le changement de lettre est testé en simulant une autre lettre ; à vérifier avec une vraie clé sur deux PC, et chronométrer la réécriture d'un gros index (lecture et écriture sur la clé).
- **Extraction depuis une grosse boîte PST (lot 6.7)** : « Extraire vers… » et « Ouvrir » relisent la boîte jusqu'au message voulu (les autres messages ne sont pas analysés, mais la boîte est parcourue) : à chronométrer sur une vraie boîte de plusieurs Go.
- **Licences** : `THIRD-PARTY-NOTICES.txt` régénéré le 28/09/2026 (454 crates, application et ligne de commande) et copié dans `site/` ; à relancer (`python scripts/third-party.py`) à chaque nouvelle dépendance.
- **Doublons presque identiques sur un très gros site** : la comparaison relit le texte stocké de tous les documents (mesuré seulement sur de petits sites) ; à chronométrer sur 100 000 documents.
- **Temps d'indexation (27/09/2026, lot 6.3)** : le test de performance a indexé les 100 000 fichiers de `target/tmp/perf-corpus` en **590 s**, contre 204 s lors de la mesure de référence du 25/09. Les recherches restent à 27-37 ms (compteurs compris). La phase de lecture progresse de façon régulière (≈ 170 fichiers/s), ce qui évoque le disque ou l'antivirus plutôt que le code, mais ce n'est pas vérifié : à mesurer de nouveau (disque au repos, exclusion antivirus) avant de conclure.
- **Disques durs et disques USB (BUG-025)** : l'indexation y est limitée par le disque, à environ 500 fichiers par seconde pour de petits fichiers. Le code lit maintenant dans l'ordre des chemins, avec un seul thread de lecture.
- **Antivirus (BUG-022)** : Windows Defender peut ralentir l'écriture des index et, rarement, bloquer un fichier. Une nouvelle tentative automatique ou une exclusion conseillée sera étudiée à l'Étape 4.
- **Licence UnRAR (pour l'Étape 4)** : la lecture des RAR utilise la bibliothèque officielle UnRAR, gratuite pour extraire. L'installeur devra inclure son texte de licence (voir ARCHITECTURE.md, « Third-party licences »).

## Étape 3 : avancement

1. 🟢 **Suivi des dossiers en direct** et indexation incrémentale : seuls les fichiers modifiés sont réindexés.
2. 🟢 **Favoris** (le bouton ★) et recherches sauvegardées.
3. 🟢 **Raccourci global** pour ouvrir Prospector de n'importe où.
4. 🟢 **Aperçu riche** : vrais tableaux pour Excel, diapositives pour PowerPoint.
5. 🟢 **Rapport PDF** d'une recherche.
6. 🟢 **RAR / 7z**.

## Étape 4 : avancement

Choix faits le 27/09/2026 : pas de signature de code pour l'instant, hébergement sur GitHub (dépôt à créer plus tard), Windows d'abord, éditeur « Faycal Azib », icône A, clé de mise à jour avec mot de passe dans `E:\keys\prospector`.

1. 🟢 **Icône de l'application** : piste A « Loupe crayonnée » (lisible en 16 px), toutes tailles générées, reprise sur le site.
2. 🟢 **Installeurs Windows** : .exe par utilisateur sans droits admin, choix de la langue EN/FR/ES/AR ; 4 MSI ; licences tierces incluses (UnRAR obligatoire).
3. 🟡 **Mises à jour automatiques** : code et interface prêts. Restent : ta clé (`E:\keys\prospector`, avec mot de passe, générée par toi) et l'adresse du dépôt GitHub (plus tard).
4. 🟢 **Page de téléchargement + guide** en 4 langues (`site/`), publiés par GitHub Pages.
5. 🟢 **Automatisation GitHub** : `release.yml` (tests, puis installeurs dans une version brouillon avec le fichier de mise à jour) et `site.yml` (page).
6. 🟢 **Procédure de publication** : [RELEASE.md](RELEASE.md) ; [CHANGELOG.md](../CHANGELOG.md).
7. 🟢 **Conseil antivirus** (BUG-022) : dans le guide (exclusion du dossier des index).
8. ⬜ **À mesurer par toi sur la version installée** : lancement < 2 s.
9. ⬜ **Plus tard** : signature du code, macOS et Linux.

## Plan pour dépasser FileLocator Pro (Étapes 5 à 8)

> Validé le 27/09/2026. La comparaison complète (✅ / 🟡 / ❌) est dans [COMPARATIF-FILELOCATOR.md](COMPARATIF-FILELOCATOR.md).
> Choix : ordre selon l'utilité au quotidien ; IA en module téléchargeable ; pour les fonctions d'entreprise, une version simple (ligne de commande, export HTML, index sur le réseau), sans interfaces .NET / COM, ni règles d'administrateur, ni XSLT.

### Étape 5 : les manques les plus utiles au quotidien

| Lot | Contenu | État |
| --- | --- | --- |
| 5.1 | **Nom de fichier** : jokers `*.pdf`, expressions régulières, recherche sur le nom seul, recherche de dossiers | 🟢 à tester |
| 5.2 | **OCR intégré à Windows** : texte des images et des PDF scannés (FR, AR…), sans rien installer | 🟢 à tester |
| 5.3 | **Anciens formats et autres** : .doc, .ppt, RTF, ODT / ODP, EPUB ; archives JAR, TAR, GZ, BZ2 | 🟢 à tester |
| 5.4 | **Pièces jointes** des e-mails (.msg, .pst, .eml), MBOX (Thunderbird), OST | 🟢 à tester (PST/OST : à vérifier sur tes vraies boîtes) |
| 5.5 | **NEAR** (mots proches) et LINES | 🟢 à tester (+ `LIKE mot`, regex multi-lignes avec `(?s)`) |
| 5.6 | **Travail sur les résultats** : chercher dans les résultats, filtre instantané, historique ← →, onglets, enregistrer et rouvrir des résultats | 🟢 à tester |
| 5.7 | **Intégration Windows** : une seule instance, clic droit « Chercher avec Prospector » dans l'Explorateur (indexé si le dossier est dans un site, sinon scan direct), ouvrir dans l'éditeur à la bonne ligne, fichiers `.prospector` ouverts d'un double-clic | 🟢 à tester |
| 5.8 | **Dates libres** (du … au …), **date de création** (indexée) et **dernier accès** (lu sur le disque), **attributs** lecture seule / caché / système, **empreinte** MD5 / SHA-256, « **Trouver les copies** » (même taille, puis même empreinte) ; index existants migrés depuis leur texte stocké, sans rien relire | 🟢 à tester |

### Étape 6 : les nouveautés qu'aucun concurrent n'a

| Lot | Contenu | État |
| --- | --- | --- |
| 6.1 | **Alertes** sur une recherche enregistrée : cloche 🔔 sur une ★, notification Windows dès qu'un fichier se met à correspondre (y compris les changements faits pendant que Prospector était fermé), pastille du nombre de nouveaux, étiquette « Nouveau » ; icône près de l'horloge, fermeture qui garde Prospector en arrière-plan, lancement au démarrage de Windows | 🟢 à tester |
| 6.2 | **Détecteurs** validés (13) : IBAN, BIC, RIB, carte bancaire, e-mail, téléphone, adresse IP, montant, n° de TVA, SIREN/SIRET, n° de sécurité sociale, DNI/NIE, passeport/CNI ; **audit des données personnelles** avec totaux par type, filtre en un clic, rapport PDF masqué | 🟢 à tester |
| 6.3 | **Compteurs par critère** : nombre de fichiers trouvés par type (pastilles du haut), langue, année (nouvelle rangée sous la Date) et site (rail, sites décochés compris), chacun sans son propre filtre ; un clic = filtre | 🟢 à tester |
| 6.4 | **Rapport par mot-clé** (occurrences de chaque terme par fichier, totaux ; aussi dans le rapport PDF), **copie groupée** vers un dossier ou un ZIP (arborescence gardée ou à plat, rien d'écrasé, conteneurs une fois), **export HTML**, **CSV compatible Excel** (séparateur selon la langue), **choix des colonnes**, fichiers affichés ou tous les fichiers trouvés (jusqu'à 10 000) | 🟢 à tester |
| 6.5 | **Doublons** : fichiers identiques (regroupés par taille via les manifestes, puis SHA-256) et documents presque identiques (MinHash sur le texte indexé, seuil réglable, entre formats) ; place récupérable ; mise à la **Corbeille** avec confirmation, jamais tout un groupe ; export CSV | 🟢 à tester |
| 6.6 | **Images** : miniatures dans les résultats, grande image dans l'aperçu avec **les mots trouvés entourés sur l'image** (position donnée par l'OCR de Windows), texte OCR surligné dessous ; orientation de l'appareil photo respectée ; sans nouvelle bibliothèque | 🟢 à tester |
| 6.7 | **Extraire** un fichier d'une archive ou d'un e-mail : « Extraire vers… » dans l'aperçu, « Ouvrir » ouvre le fichier lui-même (copie dans le dossier des données, vidée au lancement suivant), copie groupée qui extrait (case cochée par défaut) ; images d'une archive ou jointes à un e-mail : miniature (sauf dans une boîte PST/OST/mbox), grande image et mots entourés ; les messages eux-mêmes ne s'extraient pas | 🟢 à tester |
| 6.8 | **Mode portable** : un fichier `portable` à côté de `prospector.exe` met tout dans `ProspectorData` (réglages, index, données de l'interface WebView2), rien d'écrit sur le PC (ni menu de l'Explorateur, ni lancement au démarrage, ni mise à jour installée : un lien « Télécharger ») ; clé qui change de lettre : sites, index, manifestes et recherches enregistrées réécrits sans rien relire ; indépendant d'un Prospector installé ; ZIP fabriqué par `scripts/portable.py` et joint à chaque version, lien sur la page de téléchargement | 🟢 à tester |

### Étape 7 : automatisation et réseau

| Lot | Contenu | État |
| --- | --- | --- |
| 7.1 | **Ligne de commande** `prospector-cli` : `search`, `scan`, `index` (refusé si l'application est ouverte), `sites` ; tous les filtres ; sortie texte / JSON / JSON Lines / CSV ; codes de sortie 0/1/2/3 ; mêmes données que l'application (y compris portable) ; case « Ligne de commande dans le PATH » dans les Réglages, retirée à la désinstallation ; livrée dans les installeurs et le ZIP portable ; messages en anglais, expliquée dans le guide (4 langues) | 🟢 à tester |
| 7.2 | **Groupes de sites** : bande de pastilles en haut du panneau « Sites de fouille » (maquette validée), un clic ou `Ctrl+1…9` coche exactement les sites du groupe, `Ctrl+0` = tous ; « + Groupe » enregistre les sites cochés ; clic droit : renommer, remplacer par les sites cochés, supprimer ; pastilles de couleur sous chaque site ; rail étroit : pastilles numérotées ; stockés dans le dossier des données (suivent le mode portable), `--group` en ligne de commande | 🟢 à tester |
| 7.3 | **Index partagé** sur un disque réseau : un PC le tient à jour (bail `maintainer.json` renouvelé toutes les 30 s), les autres le lisent et voient ses mises à jour sans rien faire ; relais automatique après 2 minutes sans signe de vie ; lecteurs réseau convertis en `\\serveur\partage\…` ; recherches enregistrées, alertes et groupes gardés sur chaque PC (déplacés une fois) ; bandeau « Index partagé · tenu à jour par … », ajout / réindexation / suppression masqués en lecture ; ligne de commande : `index` refusé si un autre PC tient l'index | 🟢 à tester |
| 7.4 | **Liste de termes** : « + Liste de termes » lit un fichier texte / CSV (un terme par ligne, encodage détecté, commentaires `#`, expressions `/…/`, 5 000 termes au plus), pastille + panneau (maquette validée) : « au moins un » ou « tous », aperçu, rapport par terme, remplacer, retirer ; combinée aux mots de la recherche ; suit onglets, historique, recherches enregistrées, fichiers `.prospector` ; `--terms-file` / `--terms-all` en ligne de commande | 🟢 à tester |

### Étape 8 : IA locale optionnelle

> Proposition validée le 28/09/2026. **Recherche par le sens et entre langues** (« contrat de location » trouve aussi *bail*, *lease agreement*, *عقد إيجار*), 100 % locale, dans un module optionnel.
> Choix :
> - modèle **Granite Embedding 97M multilingual r2** (IBM, avril 2026, Apache 2.0, 52 langues dont FR / EN / ES / AR, vecteurs de 384 nombres, ONNX quantifié de 98 Mo) ;
> - exécution par `ort` (ONNX Runtime 1.24) chargé depuis le dossier du module, et `tokenizers` ;
> - module d'environ 140 Mo (modèle + tokenizer + ONNX Runtime) en **un ZIP joint à nos versions GitHub**, empreinte SHA-256 inscrite dans l'application ;
> - index du sens **au choix, site par site** (case « Sens » ; proposée à l'installation) ;
> - recherche par le sens **à activer** avec une pastille « ≈ Sens », qui reste activée d'une recherche à l'autre.
> Écartés : EmbeddingGemma 300M (licence Gemma restrictive), Qwen3-Embedding 0.6B (trop gros), multilingual-e5-small (nettement moins bon).

| Lot | Contenu | État |
| --- | --- | --- |
| 8.1 | **Module** : ZIP reproductible de 77 Mo (133 Mo installé) fabriqué par `scripts/sense-module.py` ; installation vérifiée (empreinte du ZIP, puis de chaque fichier), depuis un fichier ou par téléchargement (inactif tant que le dépôt n'existe pas), suppression ; section des Réglages (maquette validée) ; modèle chargé en 1,2 s ; « contrat de location » ↔ *lease agreement* 0,87, *contrato de arrendamiento* 0,88, *عقد إيجار* 0,88, contre 0,68 pour un texte sans rapport ; BUG-035 | 🟢 à tester |
| 8.2 | **Index du sens** : passages d'environ 250 mots, vecteurs en 8 bits à côté de l'index de chaque site, calcul en arrière-plan reprenable et incrémental, progression dans le rail ; mesure sur 100 000 fichiers (estimation : 20 à 60 min, à confirmer) | ⬜ |
| 8.3 | **Recherche** : pastille « ≈ Sens », fusion des classements mots + sens (RRF), étiquette « par le sens » avec le passage le plus proche ; `--meaning` en ligne de commande ; maquette avant intégration | ⬜ |
| 8.4 | **Mesures et finitions** : vitesse, mémoire, disque ; version portable et index partagé ; guide en 4 langues ; licences (Apache 2.0, MIT) | ⬜ |

## Historique

| Date | Événement |
| --- | --- |
| 28/09/2026 | `.gitignore` complété : caches Vite, Python (`__pycache__`), clés de signature (`*.key`, `.env`…, cf. RELEASE.md), fichiers temporaires, OS, `.claude/settings.local.json` |
| 28/09/2026 | Lot 8.1 : module de recherche par le sens (`core/src/sense/`, `ort` + `tokenizers`, `scripts/sense-module.py`), installation / téléchargement / suppression (`src-tauri/src/sense.rs`), section des Réglages ; tests sur le vrai modèle ; BUG-035 (vecteurs changés par le remplissage) ; 188 tests au vert |
| 28/09/2026 | Lot 7.4 : listes de termes (`core/src/terms.rs`, `SearchRequest.term_list`), `TermList.svelte`, commande `read_term_list`, `--terms-file` ; index et scan direct identiques, rapport par terme ; 2 000 termes cherchés en 375 ms (petit site, compilation de débogage) ; maquettes `docs/mockups/terms-*.png` ; l'Étape 7 est entièrement codée ; 185 tests au vert |
| 28/09/2026 | Lot 7.3 : index partagé (`core/src/share.rs` : bail, chemins réseau ; moteur en lecture seule, relecture du catalogue et des index ; dossier personnel), `src-tauri/src/share.rs` (bail tenu en arrière-plan, bascule de rôle, libération à la fermeture), interface et ligne de commande, guide ; 181 tests au vert |
| 28/09/2026 | Lot 7.2 : groupes de sites (`core/src/groups.rs`, `site-groups.json`), commandes Tauri, `SiteGroups.svelte` + store, `Ctrl+0…9` (touche physique, AZERTY compris), pastilles sous les sites, `--group` dans `prospector-cli` (et groupes listés par `sites`), guide ; maquettes `docs/mockups/groups-*.png` ; 176 tests au vert |
| 28/09/2026 | Lot 7.1 : crate `cli/` (`prospector-cli`, clap), `core/src/locate.rs` (dossier des données partagé avec l'application), `core/src/userpath.rs` (PATH de l'utilisateur), case des Réglages, crochets de désinstallation NSIS et WiX, `tauri.release.conf.json`, licences tierces régénérées (application + ligne de commande), guide (ligne de commande, version portable) ; BUG-034 ; 175 tests au vert |
| 28/09/2026 | Lot 6.8 : mode portable (`src-tauri/src/portable.rs`, `core/src/portable.rs`) : données à côté du programme, fenêtre créée par le code pour y mettre les données WebView2, identifiant propre, changement de lettre suivi (`move_drive` : index réécrits depuis leurs champs stockés, manifestes, recherches enregistrées, catalogue), réglages système masqués, mises à jour annoncées seulement ; `scripts/portable.py`, étape de `release.yml`, lien sur la page de téléchargement ; 165 tests au vert |
| 28/09/2026 | Lot 6.7 : extraction depuis les archives et les e-mails (`core/src/unpack.rs`, `extract_raw` : même parcours que l'indexation en mode « capture », seules les entrées menant au fichier sont décompressées, sans extraction de texte), champ `innerKind` des résultats, « Extraire vers… », « Ouvrir » le fichier lui-même, copie groupée qui extrait, images internes (miniature, mots entourés) ; test qui extrait chaque document interne indexé des fichiers d'exemple ; 159 tests au vert |
| 28/09/2026 | Lot 6.6 : miniatures par l'imagerie de Windows (`core/src/thumb.rs`, JPEG, orientation EXIF), mots entourés sur l'image (`image_matches` : OCR avec positions + mêmes règles de surlignage), aperçu image + texte ; BUG-033 ; 150 tests au vert |
| 27/09/2026 | Lot 6.5 : doublons exacts (`core/src/dupes.rs`, tailles depuis les manifestes, empreinte seulement des tailles partagées) et presque identiques (MinHash 64 valeurs, tranches LSH, seuil 70 à 95 %), fenêtre Doublons (bouton dans le rail), Corbeille Windows (crate `trash`) ; 148 tests au vert |
| 27/09/2026 | Lot 6.4 : fenêtre « Exporter… » (Excel CSV / HTML / JSON, colonnes au choix dont une par mot-clé et par donnée détectée, portée affichés / tous), rapport par mot-clé (`keyword_report`), copie groupée (`copy_files`, dossier ou ZIP, progression, arrêt, bilan) ; 146 tests au vert |
| 27/09/2026 | Lot 6.3 : compteurs par type, langue, année et site sur tous les fichiers trouvés (collecteur Tantivy sur champs rapides, `kind` et `lang` rendus rapides, index migrés depuis leur texte), chaque critère compté sans son propre filtre ; affichés dans les filtres existants ; 141 tests au vert |
| 27/09/2026 | Lot 6.2 : 13 détecteurs validés par clé ou somme de contrôle (`core/src/detect.rs`), chiffres arabes reconnus, combinés aux mots (ET), identiques en index et en scan direct ; totaux par type sur tout le site (au-delà des 200 fichiers affichés) ; audit en un clic, bandeau filtrable, rapport PDF avec récapitulatif et numéros masqués ; 139 tests au vert |
| 27/09/2026 | Lot 6.1 : alertes sur une recherche enregistrée (requête construite par l'interface, fichiers déjà vus mémorisés, vérification après chaque mise à jour d'un site sur ses seuls fichiers changés : filtre `inFiles`), notifications Windows, pastille et étiquette « Nouveau », icône dans la zone de notification, fermeture en arrière-plan, lancement au démarrage ; 130 tests au vert |
| 27/09/2026 | Lot 5.8 : date de création (nouveau champ d'index `created`), dernier accès, période personnalisée, attributs (lecture seule, caché, système), empreinte MD5 / SHA-256 et « Trouver les copies », dans les deux modes (l'index donne les candidats, le disque tranche) ; migration des index depuis leur texte stocké (ni relecture ni OCR) ; 129 tests au vert |
| 27/09/2026 | Lot 5.7 : une seule instance (un 2ᵉ lancement passe la main), « Chercher avec Prospector » au clic droit de l'Explorateur (dossier, fond de dossier, disque ; recherche indexée limitée au dossier s'il est dans un site, sinon scan direct de ce dossier ; réglable), « Ouvrir à la ligne N » dans l'éditeur de code (VS Code, VSCodium, Cursor, Notepad++, Sublime Text ou commande personnalisée), association des fichiers `.prospector` ; BUG-032 ; 121 tests au vert |
| 24/09/2026 | Plan validé, multilingue ajouté à la spec |
| 24/09/2026 | Étape 0 : 8 maquettes, thème « Crayons de couleur », puis thème « Sonar » ; étape validée |
| 24/09/2026 | Étape 1 : moteur Tantivy multilingue ; BUG-019 (alger/alter) et BUG-020 (Alger/Algérie) corrigés ; étape validée |
| 25/09/2026 | Étape 2 : Excel, PowerPoint, ZIP, Outlook .msg/.pst, expressions régulières, options Aa/ab, scan direct, coloration du code |
| 25/09/2026 | Étape 2 : indexation lente sur disque dur USB détectée et corrigée (BUG-021, BUG-025) ; 48 tests au vert |
| 26/09/2026 | Étape 3 : indexation incrémentale + suivi des dossiers en direct ; 55 tests au vert |
| 27/09/2026 | Étape 3 : recherches enregistrées (★) ; BUG-026 (écriture refusée par l'antivirus) corrigé par des nouvelles tentatives ; 56 tests au vert |
| 27/09/2026 | Étape 3 : raccourci global (Ctrl+Maj+Espace, réglable) |
| 27/09/2026 | Étape 3 : aperçu riche (tableaux Excel, diapositives PowerPoint) ; 59 tests au vert |
| 27/09/2026 | Étape 3 : rapport PDF d'une recherche |
| 27/09/2026 | Étape 3 : RAR et 7z ; les 6 points de l'étape sont faits ; 63 tests au vert |
| 27/09/2026 | Lot 5.6 : onglets (Ctrl+T / Ctrl+W / Ctrl+Tab, un scan direct continue dans un onglet caché), historique Alt+← / Alt+→ par onglet, filtre instantané de la liste, « Chercher dans ces résultats » (`withinPaths`, par le moteur), enregistrer / ouvrir des résultats (`.prospector`, Ctrl+S / Ctrl+O) ; 107 tests au vert |
| 27/09/2026 | Lot 5.5 : `a NEAR b` / `NEAR:n` (en caractères), `LIKE mot`, `LINES:a-b` / `LINES:a+`, syntaxe de FileLocator Pro ; vérification du texte commune à l'index et au scan direct (`QueryLogic`) ; 106 tests au vert |
| 27/09/2026 | Lot 5.4 : pièces jointes des e-mails (.msg, .eml, .pst/.ost), boîtes MBOX ; BUG-031 ; 100 tests au vert |
| 27/09/2026 | Lot 5.3 : .doc et .ppt (fichiers réels du corpus Apache POI), RTF, ODT/ODP, EPUB, TAR/GZ/BZ2/JAR, PDF protégés contre la copie ; BUG-030 ; 94 tests au vert |
| 27/09/2026 | Lot 5.2 : OCR intégré à Windows (images PNG/JPEG/TIFF…, pages scannées des PDF, arabe compris), réglable ; BUG-029 corrigé ; 80 tests au vert |
| 27/09/2026 | Lot 5.1 : critère « nom de fichier » (jokers, exclusions, regex, noms sans accents) et recherche de dossiers ; résultats identiques dans l'index et en scan direct ; 71 tests au vert |
| 27/09/2026 | Comparaison avec FileLocator Pro : environ 20 manques identifiés ; plan en Étapes 5 à 8 validé ([COMPARATIF-FILELOCATOR.md](COMPARATIF-FILELOCATOR.md)) |
| 27/09/2026 | Étape 4 : installeurs Windows, mises à jour automatiques (à activer), page et guide en 4 langues, automatisation GitHub ; BUG-027 (une erreur dans un PDF pouvait fermer toute l'application installée) et BUG-028 corrigés |
