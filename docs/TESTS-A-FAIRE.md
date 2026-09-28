# Tests à faire dans l'application

> Liste des vérifications manuelles à faire dans la fenêtre Tauri (`pnpm tauri dev`), étape par étape.
> Coche chaque case quand le test est OK. Si un test échoue, dis-moi lequel : il sera corrigé et suivi dans [BUGS.md](BUGS.md).
> L'état global du projet est dans [SUIVI.md](SUIVI.md).

**Avant de commencer :** relance `pnpm tauri dev` après chaque livraison qui touche le back (Rust).

## Étape 2 — Formats, requêtes, scan direct

### Formats
- [x] Un site contenant `test_fixtures/` s'indexe sans erreur.
- [x] Chercher `renouvellement` trouve le fichier Excel `factures-2024.xlsx` (le mot est dans sa 2ᵉ feuille), et l'aperçu montre le nom de chaque feuille en titre de son tableau.
- [x] Chercher un mot d'une diapositive trouve le PowerPoint, et l'aperçu numérote les diapositives.
- [x] Un document dans un ZIP s'affiche sous la forme `archive.zip › dossier/fichier`.
- [x] Sur un document dans un ZIP, « Ouvrir » ouvre le fichier lui-même (copie extraite dans `…\opened\…`, lot 6.7) ; sur un message d'une boîte PST, il ouvre la boîte mail.
- [x] Les e-mails `.msg` sont trouvés par leur contenu (`"Test Email"` → `test_email.msg`). Le `.pst` d'exemple est vide : à vérifier avec une vraie boîte (lot 5.4).

### Requêtes et options
- [x] `/INV-\d{4}/` trouve les numéros de facture.
- [x] La pastille `.*` fait de toute la saisie une expression régulière.
- [x] Une expression invalide (`/[abc/`) affiche un message d'erreur traduit.
- [x] Pastille `Aa` : `Contrat` ne trouve plus `contrat`.
  - [x] (BUG-037) Avec `Aa`, les compteurs des pastilles de type correspondent aux fichiers affichés (`Contrat` + `Aa` : plus de TEXTE 43 pour 1 fichier).
- [x] Pastille `ab` désactivée : `contrat` trouve aussi les mots qui le contiennent (`contratos` dans `es/notas.md`). Pastille « Tolérance aux fautes » éteinte pour ce test. (`contrato` est surligné même avec `ab` : c'est une forme du mot, voulu.)
  - [x] (BUG-037) Sur le site GIT, `contrat` avec « Tolérance aux fautes », avec `Aa`, ou `ab` éteinte, répond nettement plus vite qu'avant (1 000 à 1 750 ms en `pnpm tauri dev`). Les temps de `pnpm tauri dev` sont ceux d'une compilation de débogage.
- [x] `échéance OR indemnité` → `windows-1252.txt` + `notes-contrat.txt` ; `contrat NOT signature` → sans `signature.eml` ni `notes-contrat.txt` ; `"contrat signé"` → `signature.eml` seul (dans test_fixtures).

### Scan direct
- [x] Pastille « Scan direct » : les résultats arrivent au fil de l'eau.
- [x] Le bouton Arrêter interrompt le scan.
- [x] L'aperçu d'un résultat de scan direct s'affiche.

### Aperçu
- [x] Un fichier de code (`.rs`, `.py`, `.js`…) est coloré, dans les deux thèmes.

## Étape 3 — Différenciateurs

### 3.1 Indexation incrémentale et suivi des dossiers en direct
- [x] Au premier lancement après la mise à jour, les index existants se reconstruisent une fois, tout seuls. (Déjà passé : les index actuels fonctionnent.)
- [x] Une fois l'indexation terminée, le site affiche « Suivi en direct ».
- [x] Le bouton ↻ s'appelle « Mettre à jour ». Cliqué sans rien avoir changé, il termine presque instantanément.
- [x] Créer un `.txt` contenant un mot rare dans un site suivi : en 2 à 3 s, le mot est trouvé.
- [x] Modifier ce fichier (remplacer le mot) : l'ancien mot n'est plus trouvé, le nouveau l'est.
- [x] Supprimer le fichier : il disparaît des résultats.
- [x] Renommer un dossier : ses fichiers sont trouvés sous le nouveau chemin, plus sous l'ancien.
- [x] Avec une recherche affichée, modifier un fichier trouvé : la liste se met à jour sans perdre le fichier sélectionné.
- [x] Fermer Prospector, modifier un fichier, relancer : le changement est rattrapé au démarrage.
- [ ] Débrancher un disque USB indexé puis relancer : le site passe en erreur, sans message répété, et ses documents restent trouvables. ⏸ Reporté (à faire plus tard).
- [ ] Copier d'un coup un dossier de plus de 50 fichiers : la progression s'affiche dans le rail. ⏸ Reporté (à faire plus tard).

### 3.2 Recherches enregistrées (★)
- [ ] Avant toute recherche, le bouton ★ est grisé.
- [ ] Après une recherche, ★ ouvre un petit formulaire, prérempli avec la requête. « Enregistrer » ajoute la recherche dans « Recherches enregistrées », dans le rail.
- [ ] L'étoile devient pleine pour une recherche déjà enregistrée. Si tu changes une option (Aa, un filtre…), elle redevient vide.
- [ ] Rouvrir ★ sur une recherche enregistrée : changer le nom puis « Enregistrer » la renomme, sans créer de doublon.
- [ ] « Retirer », dans le formulaire ou via le × au survol dans le rail, la supprime.
- [ ] Un clic sur une recherche enregistrée la relance avec son mode (indexée ou scan direct), ses options, ses filtres et ses sites.
- [ ] Les recherches enregistrées sont toujours là après un redémarrage de Prospector.
- [ ] Après avoir changé le dossier des index (Réglages), les recherches enregistrées suivent.
- [ ] En arabe, le formulaire et la liste s'affichent de droite à gauche.

### 3.3 Raccourci global
- [ ] Prospector réduit ou derrière une autre fenêtre : `Ctrl + Maj + Espace` le ramène au premier plan, le curseur dans la zone de recherche (le texte déjà saisi est sélectionné).
- [ ] Prospector au premier plan : le même raccourci le réduit.
- [ ] Réglages → « Raccourci global » : choisir `Ctrl + Alt + F`, qui fonctionne tout de suite ; l'ancien raccourci ne fait plus rien.
- [ ] Choisir « Aucun » : plus aucun raccourci ne fait apparaître Prospector.
- [ ] Le choix est conservé après un redémarrage.
- [ ] Si un raccourci est déjà utilisé par une autre application, un message le dit et l'ancien raccourci reste actif.

### 3.4 Aperçu riche (Excel et PowerPoint)
- [ ] Au premier lancement, les index se reconstruisent une fois : l'extraction Excel a changé pour garder les colonnes alignées.
- [x] Chercher `renouvellement` : l'aperçu de `factures-2024.xlsx` montre un tableau par feuille (« Janvier », « Février »), l'en-tête en gras, le mot surligné dans sa cellule.
- [ ] Sur un vrai classeur avec des cellules vides, les colonnes restent alignées.
- [ ] Les montants et les dates sont alignés à droite. Un tableau large défile horizontalement dans l'aperçu.
- [x] Chercher `tacita` : l'aperçu de `presentacion-oferta.pptx` montre une carte par diapositive, avec son numéro et son titre.
- [ ] Les flèches ↑ ↓ et F3 passent d'une occurrence à l'autre dans les tableaux et les diapositives.
- [ ] Sur un grand classeur (plus de 400 lignes), chaque correspondance garde le nom de sa feuille au-dessus, et `⋯` marque les lignes non affichées.
- [ ] En arabe, un tableau en français reste de gauche à droite.

### 3.5 Rapport PDF
- [ ] Après une recherche, le menu Téléchargement (⤓) propose « Rapport PDF… ». Il est grisé s'il n'y a aucun résultat.
- [ ] Le choisir ouvre la boîte d'impression de Windows. Avec l'imprimante « Microsoft Print to PDF » ou « Enregistrer au format PDF », on obtient un fichier PDF.
- [ ] Le PDF montre : la requête, le mode, les options, les filtres, les dossiers exclus, les sites, les totaux, puis chaque fichier avec son chemin, ses détails et ses extraits surlignés.
- [ ] Les résultats suivent l'ordre de tri affiché à l'écran.
- [ ] Avec le thème Sonar (sombre), la page reste blanche.
- [ ] En arabe, le rapport se lit de droite à gauche, les chemins restent de gauche à droite.
- [ ] `Ctrl + P` dans la fenêtre imprime aussi le rapport, pas l'écran.
- [ ] Après l'impression (ou l'annulation), l'application est intacte.

### 3.6 RAR et 7z
- [ ] Au premier lancement, les index se reconstruisent une fois : les RAR et 7z déjà vus comme « non pris en charge » sont relus.
- [ ] Chercher `terrassement` : trouvé dans `devis-chantier.rar › devis/terrassement.txt`. L'aperçu montre le texte.
- [ ] Chercher `palissade` puis `andamios` : trouvés dans `courrier-chantier.7z` (compression « solide »).
- [ ] Sur un de ces résultats, « Ouvrir » ouvre l'archive elle-même (7-Zip, WinRAR ou l'Explorateur).
- [ ] `verrouille.rar`, protégé par mot de passe, n'est pas lu ; le site affiche un fichier illisible de plus.
- [ ] Sur tes propres archives : un vrai .rar, un .7z et un RAR en plusieurs parties (`.part1.rar`, `.part2.rar`), dont le contenu n'apparaît qu'une fois.
- [ ] Le filtre « Archives » ne montre que des documents venant d'archives.
- [ ] En scan direct, les RAR et 7z sont lus aussi.

## Étape 4 — Installation et distribution

### 4.1 Installeur Windows (`target/release/bundle/nsis/Prospector_0.1.0_x64-setup.exe`)
- [ ] Au lancement de l'installeur, une fenêtre propose la langue (English, Français, Español, العربية). En arabe, l'installeur s'affiche de droite à gauche.
- [ ] L'installation se fait sans demande de droits administrateur.
- [ ] Prospector apparaît dans le menu Démarrer, avec son icône.
- [ ] Dans « Applications installées », l'éditeur affiché est « Faycal Azib ».
- [ ] Le dossier d'installation contient `licenses\THIRD-PARTY-NOTICES.txt` et `licenses\UnRAR-license.txt`.
- [ ] **Temps de lancement** (critère < 2 s) : du double-clic à la fenêtre utilisable. Note la durée mesurée.
- [ ] L'application installée retrouve tes sites et tes index (même dossier de données qu'en développement).
- [ ] Indexer un dossier contenant `test_fixtures/broken/damaged.pdf` : l'application ne se ferme pas (BUG-027).
- [ ] Désinstaller : Prospector disparaît, le dossier des index est conservé.

### 4.2 Installeurs MSI (`target/release/bundle/msi/`)
- [ ] Il y a 4 MSI (en-US, fr-FR, es-ES, ar-SA). Celui en français installe correctement.

### 4.3 Page de téléchargement et guide (`site/index.html`, `site/guide.html`, à ouvrir dans le navigateur)
- [ ] Les 4 langues s'affichent. L'arabe se lit de droite à gauche.
- [ ] Le texte sur l'avertissement Windows (« Informations complémentaires → Exécuter quand même ») est clair.
- [ ] Le guide couvre bien tout ce que tu veux que les utilisateurs sachent.

### 4.4 Mises à jour (une fois la clé et le dépôt en place, voir docs/RELEASE.md)
- [ ] Publier une version 0.1.1 : une copie installée en 0.1.0 affiche la carte « Prospector 0.1.1 est disponible ».
- [ ] « Mettre à jour et redémarrer » installe la nouvelle version, puis la relance.
- [ ] Réglages → Mises à jour → « Rechercher une mise à jour » répond « Prospector est à jour ».

## Étape 5 — Parité avec FileLocator Pro

### 5.1 Nom de fichier et dossiers
- [ ] Au premier lancement, les index se reconstruisent une fois (nouveau champ pour les noms).
- [ ] La pastille « + Nom de fichier » ouvre une 2ᵉ ligne, « Nommé », sous la zone de recherche. Le × la vide et la referme.
- [ ] Zone de texte vide, `*.xlsx` puis `Fouiller !` : seuls les classeurs sont listés, sans compteur ni extrait, avec « N fichiers » en en-tête.
- [ ] `contrat` + `*.docx` : seuls les fichiers Word contenant « contrat » sortent, y compris un Word rangé dans un ZIP.
- [ ] `*.pdf; *.docx` (plusieurs motifs), `!brouillon*` (exclusion), `/^inv-\d+/` (regex) donnent le résultat attendu. Une regex invalide (`/[a/`) affiche un message d'erreur.
- [ ] `resume*` trouve « Résumé.pdf » : les accents et les majuscules ne comptent pas.
- [ ] `*.rar` liste les documents de chaque RAR.
- [ ] Option « Dossiers » : `clients` liste les dossiers dont le nom contient « clients ». L'aperçu dit « C'est un dossier », et « Ouvrir » l'affiche dans l'Explorateur. `node_modules` n'apparaît jamais.
- [ ] Même chose en **scan direct** : les résultats sont les mêmes, et c'est rapide, car aucun fichier n'est ouvert quand la zone de texte est vide.
- [ ] Une recherche ★ enregistrée avec un nom de fichier le retrouve quand on la relance.
- [ ] Le rapport PDF affiche la ligne « Nom de fichier ».
- [ ] En arabe, la ligne « باسم » s'affiche à droite et le motif se lit de gauche à droite.

### 5.2 OCR (texte des images et des PDF scannés)
- [ ] Au premier lancement, les index se reconstruisent une fois : images et PDF sont relus.
- [ ] Réglages → « Texte des images (OCR) » : la case est cochée, avec la liste des langues installées dans Windows (français, anglais, arabe chez toi).
- [ ] Chercher `vitrerie` trouve `images/facture-scannee.png`, `horticulture` trouve `images/recu.jpg`, `المصعد` trouve `images/اعلان.png`, `toiture` trouve `scans/courrier-scanne.pdf`. Chaque fois, l'aperçu montre le texte lu.
- [ ] La pastille « Images » ne garde que les images.
- [ ] Sur tes vrais documents : un PDF scanné par ton scanner ou ton téléphone, une photo d'un document, une capture d'écran.
- [ ] Les petites icônes (moins de 200 pixels) ne sont pas lues.
- [ ] Décocher la case, puis la recocher : un message dit que les sites sont mis à jour, et seules les images et les PDF sont relus.
- [ ] Note le temps d'indexation d'un dossier de photos (l'OCR ralentit ce cas).

### 5.3 Anciens formats et autres documents
- [ ] Au premier lancement, les index se reconstruisent une fois : les .doc, .ppt, RTF, ODT… déjà vus comme « non pris en charge » sont relus.
- [ ] Sur `test_fixtures` : `boring` trouve `legacy/test2.doc`, `Molière` trouve `HeaderFooterUnicode.doc`, `subtitle` trouve `basic_test_ppt_file.ppt` (l'aperçu montre des cartes par diapositive), `bornage` trouve le `.rtf`, `ardoises` le `.odt`, `zinguerie` le `.odp`, `sémaphore` l'EPUB.
- [ ] Archives : `lampadaires` trouve `sauvegarde.tar.gz › rapports/bilan.txt`, `pare-feu` trouve `serveur.log.gz › serveur.log`, `javanais` trouve le `.jar`.
- [ ] `quinquennale` trouve `fr/protege.pdf` (PDF protégé contre la copie).
- [ ] **Sur tes vrais fichiers** (le plus important) : des anciens .doc et .ppt de tes archives, un RTF, un document LibreOffice, un livre EPUB.

### 5.4 Pièces jointes des e-mails et boîtes MBOX
- [ ] Au premier lancement, les index se reconstruisent une fois (les e-mails sont relus avec leurs pièces jointes).
- [ ] `chevronnage` trouve `mail/devis-charpente.eml › devis-charpente.docx`. `voliges` trouve le fichier dans le ZIP joint (`plans.zip › plans/notice.txt`).
- [ ] `phishing` trouve le .doc joint à `mail/avec-piece-jointe.msg`.
- [ ] `faîtage` trouve `mail/fournisseur.mbox › Commande de tuiles (2) › bon.txt`.
- [ ] « Ouvrir » sur une pièce jointe ouvre le message ou la boîte mail qui la contient.
- [ ] **Sur tes vraies boîtes Outlook (.pst, .ost)** : les pièces jointes (PDF, Word, Excel…) apparaissent sous chaque message. C'est le point à vérifier en priorité : je n'ai pas de vrai PST avec pièces jointes pour les tests automatiques.
- [ ] Sur une boîte Thunderbird (`.mbox`, ou un fichier sans extension dans le profil Thunderbird : renomme une copie en `.mbox` pour l'essai).

### 5.5 Proximité (NEAR), LIKE et LINES
- [ ] `solive entretoise` trouve `fr/proximite-proche.txt` et `fr/proximite-loin.txt` ; `solive NEAR entretoise` ne garde que `proximite-proche.txt`, et seuls les deux mots proches sont surlignés.
- [ ] `solive NEAR:500 entretoise` retrouve les deux fichiers ; `entretoise NOT solive NEAR entretoise` ne garde que `proximite-loin.txt`.
- [ ] Sur tes documents : `contrat NEAR résiliation` (et en arabe, `عقد NEAR فسخ`).
- [ ] `LIKE solyve` trouve les deux fichiers alors que la tolérance aux fautes est désactivée ; `solyve` seul ne trouve rien.
- [ ] `LINES:5+ entretoise` ne garde que `proximite-loin.txt` ; `LINES:1-3 (entretoise AND solive)` ne garde que `proximite-proche.txt`.
- [ ] Même résultats en **Scan direct**.
- [ ] Regex multi-lignes : `/(?s)solive.*entretoise/` trouve les deux fichiers.
- [ ] Le texte d'aide de la zone de recherche mentionne NEAR (4 langues) ; le guide du site liste NEAR, LIKE et LINES.

### 5.6 Travail sur les résultats
- [ ] **Onglets** : `+` (ou Ctrl+T) ouvre un onglet vide avec les mêmes sites et options ; chaque onglet garde sa recherche, ses résultats et le fichier sélectionné. Ctrl+Tab / Ctrl+Maj+Tab passe de l'un à l'autre, Ctrl+W ou le clic molette ferme. Le dernier onglet se vide au lieu de se fermer.
- [ ] Un **scan direct** lancé dans un onglet continue quand on passe à un autre (le point qui respire sur l'onglet), et ses résultats sont là en revenant.
- [ ] **Historique** : après 3 recherches, Alt+← revient à la précédente avec ses résultats (sans relancer), Alt+→ revient. Appuyer deux fois sur Entrée n'ajoute pas d'étape. Changer un filtre ajoute une étape.
- [ ] **Filtre instantané** : taper `dupont` dans « Filtrer la liste » ne garde que les fichiers dont le nom ou le dossier le contient, avec « 2 sur 11 » ; un filtre qui ne garde rien affiche « Effacer le filtre ».
- [ ] **Chercher dans ces résultats** : chercher `contrat`, cliquer le bouton, taper `résiliation` : seuls les fichiers de la première liste sont gardés. La pastille « Dans les résultats de « contrat » » reste tant qu'on ne la retire pas (×), puis on cherche partout. Idem en **Scan direct**, et avec un fichier dans un ZIP.
- [ ] **Enregistrer / ouvrir** : Ctrl+S enregistre `contrat.prospector` ; Ctrl+O le rouvre dans un nouvel onglet (« ouvert depuis contrat.prospector »), avec les passages, même après avoir modifié les fichiers ; l'aperçu montre le fichier actuel. Un fichier qui n'est pas des résultats Prospector est refusé avec un message.
- [ ] Une recherche ★ enregistrée se relance **une seule fois** (pas de double affichage).
- [ ] Ctrl+S / Ctrl+O / Ctrl+W ne déclenchent rien d'autre dans la fenêtre (pas de fenêtre « Enregistrer la page », l'application ne se ferme pas).
- [ ] En arabe : onglets, flèches ← → et barre de filtre en miroir ; la pastille « داخل نتائج » reste lisible avec une recherche en français.

### 5.7 Intégration Windows

> En mode dev, le menu clic droit est **désactivé par défaut** (il pointerait vers `target\debug`). Pour le tester : Réglages → Intégration Windows → cocher la case, **en laissant `pnpm tauri dev` ouvert** (le clic droit relance l'exe de dev, qui passe la main à la fenêtre ouverte). Décoche-la après les tests. La version installée l'active toute seule.

- [ ] **Une seule instance** : Prospector ouvert (même réduit), le relancer (menu Démarrer, ou `target\debug\prospector.exe` en dev) ne crée pas de 2ᵉ fenêtre : la fenêtre existante revient au premier plan.
- [ ] **Clic droit sur un dossier d'un site indexé** (Windows 11 : « Afficher plus d'options ») → « Chercher avec Prospector » : un nouvel onglet s'ouvre, pastille « 📁 Dans : D:\…\dossier », mode **Indexée**, curseur dans le champ. Taper un mot : seuls les fichiers de ce dossier (et sous-dossiers) sortent, instantanément.
- [ ] Même chose sur un dossier **hors de tout site** : mode **Scan direct**, seuls les fichiers de ce dossier ; l'aperçu fonctionne.
- [ ] Clic droit **dans le fond** d'un dossier ouvert, et sur un **disque** (`D:\`) : même comportement (le disque entier en scan direct si aucun site ne le couvre).
- [ ] La pastille « Dans : » reste pendant les recherches suivantes ; son × revient à tous les sites (la recherche se relance). Le nom de fichier (`*.pdf`) et « Dossiers » marchent aussi dans le dossier.
- [ ] Le dossier est dans une autre casse que le site (tapé en minuscules) : mêmes résultats.
- [ ] Changer la langue de l'interface : le texte du menu clic droit suit (fermer puis rouvrir le menu de l'Explorateur).
- [ ] Décocher la case : l'entrée disparaît du clic droit ; la recocher : elle revient.
- [ ] **Ouvrir dans l'éditeur** : sélectionner un fichier `.txt` ou de code, aller à la 3ᵉ occurrence (F3), cliquer « Ouvrir à la ligne N » : l'éditeur s'ouvre sur ce fichier, curseur à la bonne ligne. Cliquer un **numéro de ligne** de l'aperçu fait pareil pour cette ligne.
- [ ] Le bouton n'apparaît **pas** pour un PDF, un Word, un fichier dans un ZIP ou un e-mail.
- [ ] Réglages → Éditeur de code : les éditeurs installés sont listés (VS Code, Notepad++…). « Commande personnalisée » avec par ex. `"C:\Windows\notepad.exe" {file}` : le bouton ouvre le Bloc-notes. Une commande vide : message « Aucun éditeur de code trouvé ».
- [ ] Un fichier dont le nom contient des espaces et `&` s'ouvre correctement.
- [ ] **Version installée** : double-cliquer un fichier `.prospector` l'ouvre dans Prospector (dans la fenêtre déjà ouverte s'il y en a une), dans un nouvel onglet.
- [ ] **Version installée** : désinstaller Prospector retire l'entrée du clic droit ; une **mise à jour** la garde.
- [ ] En arabe : la pastille « في: » et la section des Réglages sont lisibles, le chemin reste dans le bon ordre.

### 5.8 Dates, attributs, empreintes

> Au premier lancement après cette version, chaque index est **réécrit depuis son texte stocké** pour ajouter la date de création : quelques secondes par site, sans relire les fichiers ni refaire l'OCR. Vérifie que tes sites restent « prêts » avec le même nombre de documents.

- [ ] **Date de création** : « + Affiner » → Date → « Créé » + « 7 derniers jours » : seuls les fichiers créés cette semaine (un vieux fichier modifié hier n'y est pas, contrairement à « Modifié »). Pastille « Créé · 7 derniers jours ».
- [ ] **Période personnalisée** : du 01/01/2024 au 31/03/2024 : le 31/03 est inclus. Avec seulement « du », pastille « depuis le … ». Mêmes résultats en **Scan direct**.
- [ ] **Dernier accès** : « Dernier accès » + « Aujourd'hui » : les fichiers ouverts aujourd'hui (Windows ne tient pas toujours cette date à jour : le message le dit).
- [ ] L'aperçu : survoler la date d'un résultat affiche « Créé le … ».
- [ ] **Lecture seule** : mettre un fichier en lecture seule (Propriétés), chercher un de ses mots avec la pastille « Lecture seule » : seul ce fichier sort, dans les deux modes.
- [ ] **Caché** : en mode Indexée, le message « Les fichiers cachés ne sont pas indexés » apparaît ; en **Scan direct**, un fichier caché (attribut Windows) contenant le mot est trouvé, et il ne l'est pas sans la pastille.
- [ ] **Empreinte** : coller le SHA-256 d'un fichier (PowerShell : `Get-FileHash fichier`) : ce fichier et ses copies exactes sortent. Un MD5 (`Get-FileHash -Algorithm MD5`) marche aussi, majuscules comprises. `1234` : message « Une empreinte MD5 a 32 caractères… ».
- [ ] **Trouver les copies** : copier un PDF sous un autre nom dans un site, sélectionner l'original, cliquer « Trouver les copies » : nouvel onglet « Copies de … » avec les deux fichiers ; un fichier de même taille mais différent n'y est pas. Le bouton est absent pour un document dans un ZIP ou un e-mail.
- [ ] La pastille « Copies de … » (×) retire le critère ; « Réinitialiser » dans les filtres aussi.
- [ ] Une recherche ★ enregistrée avec ces filtres les retrouve ; une recherche enregistrée **avant** cette version garde son étoile pleine et ne récupère pas de filtre d'une autre recherche.
- [ ] Le rapport PDF liste les nouveaux filtres.
- [ ] En arabe : groupe Date, attributs et empreinte lisibles, « البصمة (MD5 أو SHA-256) » dans le bon ordre.

## Étape 6 — Nouveautés

### 6.1 Alertes sur une recherche enregistrée

> En mode dev, Windows affiche les notifications au nom de « PowerShell » (ou pas du tout selon les réglages de notification) : c'est normal, la version installée les montre au nom de Prospector. La fenêtre démarre maintenant cachée puis s'affiche aussitôt : vérifie qu'elle apparaît bien à chaque lancement.

- [ ] Sur une recherche ★ du rail, la **cloche** apparaît au survol ; un clic l'allume (reste visible, colorée) avec le message « Alerte activée ».
- [ ] Dans un dossier suivi, **créer** un fichier contenant le mot de la recherche : dans les 2-3 secondes, une **notification** « Nouveaux résultats : <nom> » avec le nom du fichier, et une **pastille « 1 »** sur la recherche dans le rail.
- [ ] Modifier un fichier qui ne contenait pas le mot pour qu'il le contienne : annoncé aussi. Réenregistrer un fichier **déjà annoncé** ou qui correspondait déjà avant l'alerte : **rien**.
- [ ] Cliquer la recherche ★ : elle se relance, les fichiers annoncés portent l'étiquette **« Nouveau »**, la pastille disparaît.
- [ ] Plusieurs fichiers d'un coup (copier 5 fichiers) : une seule notification, 3 noms + « (+2) ».
- [ ] **Fermer la fenêtre** (×) alors qu'une alerte existe : Prospector reste près de l'horloge (icône) ; un fichier créé déclenche toujours la notification. Clic sur l'icône : la fenêtre revient. Clic droit → **Quitter** : Prospector se ferme vraiment.
- [ ] Sans aucune alerte (et réglage non touché), fermer la fenêtre quitte Prospector comme avant.
- [ ] Réglages → **Arrière-plan** : décocher « Fermer la fenêtre garde Prospector… » : la croix quitte. Recocher : la croix cache.
- [ ] **Changements faits Prospector fermé** : quitter, créer un fichier correspondant, relancer : notification au rattrapage du démarrage.
- [ ] « Lancer au démarrage de Windows » coché (**version installée**), redémarrer la session : Prospector démarre caché près de l'horloge, les alertes veillent.
- [ ] Le menu de l'icône (Ouvrir Prospector / Quitter) et le titre des notifications suivent la langue de l'interface.
- [ ] Éteindre la cloche : plus de notification pour cette recherche.
- [ ] En arabe : cloche, pastille et étiquette « جديد » bien placées.

### 6.2 Détecteurs et audit des données personnelles

> Pour tester, crée un fichier texte dans un site avec par exemple : `IBAN FR76 3000 6000 0112 3456 7890 189`, `carte 4111 1111 1111 1111`, `jean.dupont@exemple.fr`, `06 12 34 56 78`, `TVA FR44 732829320`, `SIRET 732 829 320 00074`, `sécu 1 85 05 78 006 084 91`, `DNI 12345678Z`, `1 250,50 €`. Et un autre avec des numéros faux : `FR76 3000 6000 0112 3456 7890 188`, `4111 1111 1111 1112`.

- [ ] « + Affiner » → **Détecteurs** : cocher « IBAN », sans mot : seul le fichier au bon IBAN sort, l'IBAN est surligné dans l'extrait et l'aperçu ; le fichier aux numéros faux n'apparaît pas.
- [ ] Chaque pastille (carte, e-mail, téléphone, TVA, SIREN/SIRET, sécurité sociale, DNI/NIE, montant…) trouve sa donnée ; l'infobulle dit ce qui est vérifié.
- [ ] Un détecteur + un mot (`Dupont` + IBAN) : seulement les fichiers qui ont les deux.
- [ ] **Audit des données personnelles** : tout est coché, la recherche se lance sans mot ; pastille « Audit des données personnelles » en haut ; bandeau « E-mail 12 · Téléphone 5 · IBAN 1… » au-dessus de la liste.
- [ ] Un clic sur un type du bandeau ne garde que ces fichiers ; un second clic les remet tous.
- [ ] Sur un gros site (plus de 200 fichiers trouvés), les nombres du bandeau comptent tous les fichiers, pas seulement ceux affichés.
- [ ] Même résultat en **Scan direct**.
- [ ] **Rapport PDF** (Ctrl+P) : tableau « Données détectées » en tête ; dans les extraits, IBAN, carte, n° de sécurité sociale sont masqués (`•••• 1111`), les e-mails et montants restent lisibles ; la note l'explique.
- [ ] Une recherche ★ enregistrée avec des détecteurs les garde ; une alerte 🔔 sur un audit prévient quand un nouveau fichier contient une donnée personnelle.
- [ ] Un numéro écrit en chiffres arabes (`٠٦ ١٢ ٣٤ ٥٦ ٧٨`) est reconnu comme téléphone.
- [ ] En arabe : groupe Détecteurs, bandeau et tableau du rapport lisibles.

### 6.3 Compteurs par critère

> Au premier lancement, les index sont de nouveau réécrits depuis leur texte stocké (les champs type et langue deviennent « rapides ») : quelques secondes par site, rien n'est relu.

- [ ] Après une recherche, chaque pastille de type en haut affiche son nombre de fichiers (« PDF 12 ») ; les types sans résultat sont en gris.
- [ ] Cocher « PDF » : la liste ne garde que les PDF, mais les autres pastilles gardent leur nombre (ce qu'on obtiendrait en les ajoutant).
- [ ] « + Affiner » : les langues affichent leur nombre ; la rangée **Années** liste les années trouvées (la plus récente d'abord) avec leur nombre.
- [ ] Clic sur une année : la date passe en « Période personnalisée » du 1er janvier au 31 décembre de cette année ; nouveau clic : « N'importe quand ».
- [ ] Avec « Créé » choisi comme type de date, les années sont celles de création.
- [ ] Dans le rail, chaque site affiche le nombre de fichiers trouvés, **y compris un site décoché** (ce qu'il apporterait si on le cochait).
- [ ] Sur un gros site (plus de 200 résultats), les nombres comptent tout, pas seulement les fichiers affichés.
- [ ] En **Scan direct**, les nombres correspondent aux résultats reçus.
- [ ] La recherche reste instantanée (compteurs compris).
- [ ] En arabe : nombres lisibles sur les pastilles, les années et le rail.

### 6.4 Exports, rapport par mot-clé, copie groupée

- [ ] Menu ⤓ → **Exporter…** : la fenêtre propose « Ceux affichés » / « Tous les fichiers trouvés » (indexé seulement), le format et les colonnes (mémorisées d'une fois sur l'autre).
- [ ] **Excel** : le fichier `.csv` s'ouvre d'un double-clic dans Excel **avec les colonnes séparées** (interface en français : séparateur « ; »), accents et arabe corrects, dates reconnues comme dates, tailles comme nombres.
- [ ] Recherche `contrat OR facture`, colonne **Par mot-clé** : une colonne « contrat » et une « facture » avec le nombre d'occurrences par fichier, et une dernière ligne « Fichiers contenant le mot ».
- [ ] **Page HTML** : s'ouvre dans le navigateur, critères en tête, tableau par mot-clé, résultats avec leurs extraits surlignés ; en arabe, la page est de droite à gauche.
- [ ] **JSON** : contient les résultats, les termes et leurs comptes.
- [ ] Portée **Tous les fichiers trouvés** sur une recherche de plus de 200 résultats : le fichier contient plus de lignes que la liste affichée.
- [ ] Après un audit (6.2), la colonne **Données détectées** ajoute une colonne par type (IBAN, e-mail…).
- [ ] **Rapport PDF** (menu ⤓) sur `contrat OR facture` : tableau « Par mot-clé » en tête et, sous chaque fichier, « contrat 2 · facture 1 ». Avec Ctrl+P, pas de tableau (normal : il est calculé depuis le menu).
- [ ] Menu ⤓ → **Copier les fichiers trouvés…** → « Un dossier », « Garder les sous-dossiers » : les fichiers sont copiés avec leurs sous-dossiers ; barre de progression ; bilan « N fichiers copiés » ; « Afficher dans le dossier » ouvre la destination.
- [ ] Copier une seconde fois dans le même dossier : rien n'est écrasé, les copies s'appellent « nom (2) ».
- [ ] « Une archive ZIP » : le `.zip` s'ouvre dans l'Explorateur avec les fichiers (et leurs dossiers si demandé).
- [ ] Un résultat dans un ZIP ou un e-mail, case « Extraire les fichiers… » **décochée** : l'archive / la boîte mail est copiée une seule fois, et le bilan le signale (cochée : voir 6.7).
- [ ] Beaucoup de fichiers : **Arrêter** interrompt la copie (« Copie arrêtée : N fichiers copiés »).
- [ ] En arabe : fenêtre en miroir, textes lisibles.

### 6.5 Doublons

- [ ] Rail → bouton **Doublons** (deux feuilles, à côté du +) : la fenêtre s'ouvre, avec les sites cochés rappelés.
- [ ] Copier un PDF sous un autre nom dans un site, attendre la mise à jour, **Chercher** : un groupe « Identiques » avec les deux fichiers et la place récupérable.
- [ ] Deux fichiers de même taille mais de contenu différent ne sont pas groupés.
- [ ] Un texte recopié avec quelques mots changés (fichier .txt ou .docx) : groupe « Proches à NN % » ; un texte sans rapport n'y est pas.
- [ ] Monter le seuil à 95 % : les groupes « proches » moins ressemblants disparaissent.
- [ ] Décocher « Documents presque identiques » : seulement les identiques (plus rapide).
- [ ] Cocher tous les fichiers d'un groupe : impossible, le dernier reste grisé (« un fichier est toujours gardé ») ; un document dans un ZIP ne se coche pas.
- [ ] **Mettre à la corbeille** : confirmation, puis les fichiers sont dans la Corbeille de Windows (restaurables) et disparaissent de la liste ; le groupe disparaît s'il ne reste qu'un fichier ; l'index se met à jour tout seul.
- [ ] Un fichier ouvert dans Word ne part pas : message « n'ont pas pu être mis à la corbeille ».
- [ ] **Exporter la liste** : un CSV avec groupe, type, ressemblance, chemin, taille, date.
- [ ] Sur un gros site : la progression avance (comparaison des fichiers, puis des textes) et **Arrêter** donne des résultats partiels.
- [ ] Ouvrir / Afficher dans le dossier depuis une ligne fonctionnent.
- [ ] En arabe : fenêtre en miroir, lisible.

### 6.6 Images

- [ ] Rechercher un mot présent dans une photo ou un scan (OCR activé) : dans la liste, la carte de l'image montre sa **miniature** au lieu de l'icône.
- [ ] L'aperçu montre **l'image en grand**, et le mot cherché est **entouré** sur l'image (« 1 mot entouré sur l'image ») ; le texte lu par l'OCR est dessous, surligné.
- [ ] ↑ / ↓ (ou F3) : l'occurrence active est marquée à la fois dans le texte et sur l'image (cadre plus épais).
- [ ] Une photo prise de côté (téléphone tenu verticalement) apparaît **dans le bon sens**, cadres compris.
- [ ] Un mot tapé avec une faute (tolérance active) : cadre en pointillés.
- [ ] Une image en arabe : les mots arabes trouvés sont entourés.
- [ ] Une image dans un ZIP ou jointe à un e-mail : voir 6.7 (miniature et image en grand).
- [ ] Beaucoup d'images dans les résultats : la liste reste fluide, les miniatures arrivent au fur et à mesure.

### 6.7 Extraire d'une archive ou d'un e-mail

- [ ] Un résultat dans un ZIP (`archive.zip › dossier/fichier.pdf`) : l'aperçu montre **Extraire vers…** ; la boîte « Enregistrer sous » propose le nom du fichier ; le fichier écrit s'ouvre normalement, identique à l'original.
- [ ] Même chose pour un 7z, un RAR, un .tar.gz, et un fichier dans un ZIP lui-même dans un ZIP.
- [ ] Une pièce jointe d'un .eml, d'un .msg, d'un message d'une boîte .pst (Outlook) et d'une boîte mbox (Thunderbird) : **Extraire vers…** fonctionne.
- [ ] Un **message** d'une boîte PST (pas une pièce jointe) : pas de bouton « Extraire vers… » ; **Ouvrir** ouvre la boîte mail, comme avant.
- [ ] **Ouvrir** sur une pièce jointe ou un fichier d'archive : le fichier lui-même s'ouvre dans son application (Word, lecteur PDF…), sans passer par l'archive.
- [ ] Le fichier ouvert est dans `<dossier des index>\opened\…` ; au lancement suivant de Prospector, ce dossier est vidé (sauf un fichier encore ouvert).
- [ ] **Afficher dans le dossier** montre toujours l'archive ou la boîte mail.
- [ ] Une **image** dans un ZIP ou jointe à un .eml / .msg : miniature dans la liste, image en grand dans l'aperçu, mots cherchés entourés.
- [ ] Une image jointe à un message d'une boîte PST ou mbox : icône normale dans la liste (voulu), mais l'image en grand dans l'aperçu.
- [ ] Copie groupée (Exporter → Copier) avec des résultats dans des archives, case **Extraire les fichiers des archives et des e-mails** cochée : seuls les fichiers trouvés sont copiés ; « Garder les sous-dossiers » les range dans un dossier au nom de l'archive (`dossier-clients.zip\notes\devis.txt`) ; même chose vers un ZIP.
- [ ] Une archive modifiée depuis l'indexation (fichier retiré) : message « Ce document n'a pas pu être sorti de son archive… ».
- [ ] Une grosse boîte PST : noter le temps de « Extraire vers… » sur une pièce jointe (voir « À surveiller » dans SUIVI.md).
- [ ] En arabe, espagnol, anglais : boutons et messages traduits.

### 6.8 Mode portable

- [ ] `pnpm tauri build`, puis `python scripts/portable.py` : le ZIP `target/release/bundle/portable/Prospector_…_x64-portable.zip` contient `prospector.exe`, `portable`, `README.txt` et `licenses\`.
- [ ] Décompresser le ZIP sur une clé USB et lancer `prospector.exe` : un dossier `ProspectorData` apparaît à côté (`settings.json`, `data\`, `webview\`).
- [ ] Rien de nouveau dans `%APPDATA%\app.prospector.desktop*` ni `%LOCALAPPDATA%\app.prospector.desktop*` (le dossier `…portable` ne doit pas y apparaître).
- [ ] Réglages : la section « Mode portable » montre le dossier de la clé à la place de « Dossier des index » ; pas de case « Menu de l'Explorateur » ni « Lancer au démarrage de Windows ».
- [ ] Thème, langue et mise en page choisis sont retrouvés au lancement suivant depuis la clé, et ne changent pas ceux du Prospector installé.
- [ ] Avec le Prospector installé déjà ouvert : lancer la version portable ouvre bien une **deuxième** fenêtre (la portable), sans ramener l'installée.
- [ ] Créer un site sur un dossier de la clé, indexer, chercher. Puis brancher la clé sur un autre PC (ou changer sa lettre dans « Gestion des disques ») : au lancement, le site montre la nouvelle lettre, la recherche trouve tout **tout de suite**, sans réindexation ; « Ouvrir » ouvre bien le fichier.
- [ ] Une recherche enregistrée limitée à un dossier de la clé, et son alerte : toujours valables après le changement de lettre.
- [ ] Un site sur le disque du premier PC : sur l'autre PC, il est signalé comme débranché, ses documents restent (comme un disque débranché).
- [ ] Mise à jour (quand le dépôt existera) : la bannière propose « Télécharger », qui ouvre la page de la nouvelle version, sans rien installer.
- [ ] Page de téléchargement : lien « Version portable (ZIP…) » sous le bouton, dans les 4 langues.
- [ ] Supprimer le fichier `portable` : Prospector reprend les données du PC (installées).

### 7.1 Ligne de commande

- [ ] `cargo build --release -p prospector-cli`, puis dans un terminal : `target\release\prospector-cli.exe --help` (aide claire, 4 commandes, codes de sortie).
- [ ] `prospector-cli sites` : mêmes sites que dans l'application, avec leur nombre de documents.
- [ ] `prospector-cli search "contrat"` : mêmes fichiers que dans l'application ; mots trouvés entre crochets ; résumé « N of M files » à la fin.
- [ ] Les filtres : `--site`, `--kind pdf,word`, `--lang fr`, `--name "*.pdf"`, `--since 2024-01-01 --until 2024-12-31`, `--detect iban`, `--regex`, `--fuzzy` donnent les mêmes résultats que les mêmes réglages dans l'application.
- [ ] `--format csv > r.csv` s'ouvre dans Excel ; `--format json` et `--format jsonl` sont du JSON valide (chemins avec accents et arabe corrects).
- [ ] `prospector-cli scan "facture" D:\UnDossier` : les résultats s'affichent au fur et à mesure.
- [ ] `echo %errorlevel%` (ou `$LASTEXITCODE`) : 0 si trouvé, 1 si rien, 2 pour une option inconnue, 3 pour un site inconnu.
- [ ] Application ouverte : `prospector-cli index --all` refuse avec « Prospector is open… » ; application fermée : il met les sites à jour, puis l'application montre les nouveaux totaux.
- [ ] Installée (`pnpm tauri build --config src-tauri/tauri.release.conf.json`) : `prospector-cli.exe` est à côté de `prospector.exe` ; Réglages → Intégration Windows → **Ligne de commande dans le PATH** : dans un **nouveau** terminal, `prospector-cli sites` marche depuis n'importe quel dossier ; décocher : il n'est plus trouvé.
- [ ] Désinstaller avec la case cochée : l'entrée disparaît du PATH (Paramètres → Variables d'environnement).
- [ ] Version portable : `prospector-cli.exe` est dans le ZIP et utilise les données de la clé ; pas de case PATH dans les Réglages.
- [ ] Guide du site : sections « Ligne de commande » et « Version portable » dans les 4 langues.

### 7.2 Groupes de sites

- [ ] Avec au moins 2 sites : la bande « Groupes » apparaît en haut du panneau « Sites de fouille », avec « Tous 0 » et « + Groupe ».
- [ ] Cocher 2 sites, **+ Groupe**, nommer « Clients », Entrée : la pastille « Clients 1 » apparaît, hachurée (elle correspond aux sites cochés) ; une pastille de sa couleur apparaît sous la case de chacun de ses sites.
- [ ] Créer un 2ᵉ groupe : autre couleur, chiffre 2.
- [ ] Clic sur un groupe : seuls ses sites sont cochés, la recherche se relance ; **Tous** recoche tout.
- [ ] Au clavier : `Ctrl+1`, `Ctrl+2`, `Ctrl+0` (clavier AZERTY : les touches 1, 2, 0 de la rangée du haut, sans Maj ; et le pavé numérique) ; un chiffre sans groupe ne fait rien.
- [ ] Chaque onglet garde ses propres sites cochés : changer de groupe dans un onglet ne change pas les autres.
- [ ] Clic droit sur un groupe : **Renommer** (Entrée valide, Échap annule), **Remplacer par les N sites cochés**, **Supprimer**.
- [ ] Supprimer un site qui est le seul d'un groupe : le groupe disparaît.
- [ ] Relancer Prospector : les groupes sont toujours là, dans le même ordre et les mêmes couleurs.
- [ ] Mise en page « Strates » : pastilles numérotées aux couleurs des groupes dans le rail étroit, survol = nom et sites.
- [ ] Thème Sonar et arabe : bande en miroir, lisible.
- [ ] Ligne de commande : `prospector-cli sites` montre les groupes de chaque site ; `prospector-cli search "contrat" --group Clients` ne cherche que dans ses sites.

### 7.3 Index partagé (deux PC et un dossier partagé)

- [ ] PC A : Réglages → Dossier des index → un dossier sur un partage réseau (lecteur `Z:` ou `\\serveur\partage`) : le chemin affiché est le chemin réseau (`\\serveur\…`), le bandeau « Index partagé · tenu à jour par ce PC » apparaît.
- [ ] PC A : ajouter un site sur le partage, indexer.
- [ ] PC B : choisir le même dossier des index : ses sites apparaissent, bandeau orange « … tenu à jour par PC-A » ; pas de bouton + ni de réindexation / suppression ; la recherche trouve les documents ; l'aperçu fonctionne ; « Ouvrir » ouvre les fichiers du partage.
- [ ] PC A modifie un fichier du partage : quelques secondes après, PC B le trouve avec son nouveau contenu, sans rien faire.
- [ ] PC A ajoute ou supprime un site : PC B le voit dans les 30 secondes.
- [ ] PC A ajoute un site sur son disque local : avertissement « … les autres PC verront ses résultats … sans pouvoir ouvrir ses fichiers ».
- [ ] Fermer Prospector sur A : B prend le relais dans les 30 secondes (bandeau « … par ce PC », boutons de nouveau là) ; rouvrir A : A passe en lecture.
- [ ] Éteindre A sans fermer Prospector (ou couper son réseau) : B prend le relais au bout de 2 à 3 minutes.
- [ ] Recherches enregistrées, alertes et groupes : ceux de A ne sont pas visibles sur B, et inversement ; ceux qui existaient avant sont toujours là sur le PC qui les avait.
- [ ] Ligne de commande sur B : `prospector-cli search "…"` fonctionne ; `prospector-cli index --all` est refusé (« PC-A keeps this shared index up to date »).
- [ ] Temps de recherche depuis B : noter la durée affichée (réseau local, Wi-Fi, VPN).

### 7.4 Liste de termes

- [ ] Préparer `fournisseurs.txt` : quelques noms (dont un avec accents, un de plusieurs mots), une ligne `# commentaire`, une ligne `/FR\d{11}/`, un doublon ; l'enregistrer une fois en UTF-8, une fois en ANSI (Bloc-notes → Enregistrer sous → Encodage).
- [ ] **+ Liste de termes** → choisir le fichier : la pastille « Liste : fournisseurs.txt · N termes » apparaît ; la recherche se relance avec les fichiers qui contiennent au moins un terme.
- [ ] Clic sur la pastille : le panneau montre le fichier, le nombre de termes, les lignes ignorées (commentaire, doublon), les premiers termes ; il tient à l'écran (pastille en fin de rangée comprise).
- [ ] **Tous** : seuls les fichiers qui contiennent chaque terme restent ; **Au moins un** : retour.
- [ ] Taper aussi un mot dans « Je cherche » : il faut ce mot **et** la liste.
- [ ] Les termes trouvés sont surlignés dans l'aperçu ; ↑ / ↓ passent de l'un à l'autre.
- [ ] **Rapport par terme…** ouvre l'export, avec une colonne par terme ; le rapport PDF cite la liste dans ses critères et compte chaque terme.
- [ ] Scan direct : mêmes résultats qu'en recherche indexée.
- [ ] Enregistrer la recherche (★), changer d'onglet, revenir en arrière (Alt+←), enregistrer les résultats (Ctrl+S) puis les rouvrir : la liste est toujours là, même si le fichier a été déplacé.
- [ ] Un fichier CSV (`nom;ville`) : seule la première colonne est prise.
- [ ] Un fichier vide ou de commentaires seulement : message « Aucun terme dans … ».
- [ ] Une liste de plusieurs milliers de termes sur un gros site : noter le temps affiché.
- [ ] Ligne de commande : `prospector-cli search "" --terms-file fournisseurs.txt` (et `--terms-all`).
- [ ] En arabe et en thème Sonar : pastille et panneau lisibles, nom de fichier dans le bon sens.

### 8.1 Module de recherche par le sens

- [ ] `python scripts/sense-module.py` : le script télécharge le modèle et ONNX Runtime dans `target/`, écrit `target/sense-module/prospector-sense-1.zip` (environ 77 Mo) et affiche son SHA-256 ; relancé, il redonne le même SHA-256.
- [ ] Réglages → **Recherche par le sens (IA locale)** : « Télécharger le module (77 Mo) » grisé avec l'explication ; « Installer depuis un fichier… » → choisir le ZIP : avis « Module installé », section « ✓ Module installé · Granite Embedding 97M (IBM) · 133 Mo ».
- [ ] Le module est dans `%APPDATA%\app.prospector.desktop\modules\sense` (en portable : `ProspectorData\modules\sense`).
- [ ] Choisir un ZIP qui n'est pas le module (ou le module modifié) : message d'erreur, rien n'est installé.
- [ ] **Supprimer le module** : confirmation, puis la section revient à « non installé » ; le dossier a disparu (au plus tard au lancement suivant).
- [ ] En arabe et en thème Sonar : section lisible, exemples dans le bon ordre.

### 8.2 Index du sens

- [ ] Installer le module (8.1) : la fenêtre « Quels sites comprendre par le sens ? » s'ouvre ; chaque site affiche son estimation (« Mesure… » puis « ≈ 20 min · 64 800 passages ») ; les sites d'une heure ou moins sont pré-cochés ; « Plus tard » ferme sans rien lancer.
- [ ] **Comprendre les sites cochés** : sous chaque site, « ≈ Sens · 12 % » avec une barre bleue, puis « reste ≈ … » après quelques secondes ; le PC reste utilisable normalement (priorité basse).
- [ ] Gestionnaire des tâches pendant le calcul : Prospector en priorité basse ; environ 200 Mo de mémoire en plus en rythme normal.
- [ ] À la fin : « ≈ Sens à jour · N passages ».
- [ ] Survol d'un site → bouton **≈** : sur un site non compris, une bulle demande « Comprendre « … » par le sens ? » avec le nombre de passages et la durée estimée ; sur un site compris, confirmation puis suppression de son index du sens.
- [ ] Fermer Prospector en plein calcul, le rouvrir : le calcul reprend là où il s'était arrêté (le pourcentage ne repart pas de zéro).
- [ ] Modifier un fichier d'un site compris : après sa mise à jour, seul ce fichier est recalculé (quelques secondes).
- [ ] Réglages → rythme **économe** : le calcul suivant utilise moins le processeur (plus long).
- [ ] Le code source (`.ts`, `.py`…) n'est pas compris (le nombre de passages ne l'inclut pas).
- [ ] Index partagé : seul le PC qui tient l'index calcule ; les autres voient « à jour » une fois le calcul fini.

### 8.3 Recherche par le sens

- [ ] Avec le module installé et au moins un site compris : la pastille **≈ Sens** apparaît à côté de « Tolérance aux fautes » ; grisée en scan direct.
- [ ] Chercher une notion avec des mots absents des documents (ex. « bail d'habitation » alors qu'un document dit « location d'appartement », ou dans une autre langue) : avec ≈ Sens, le document apparaît avec « ≈ Par le sens » et son passage le plus proche ; sans ≈ Sens, il n'apparaît pas.
- [ ] Un document qui contient les mots **et** le sens : il reste en tête, avec une petite marque ≈.
- [ ] Clic sur un résultat « par le sens » : l'aperçu s'ouvre sur le passage, surligné.
- [ ] Les filtres (type, langue, date, site, groupe, nom de fichier) s'appliquent aussi aux résultats trouvés par le sens.
- [ ] Le compteur indique « … · dont N par le sens ».
- [ ] Une recherche sans rapport avec les documents ne ramène pas de résultats « par le sens » farfelus (ou peu).
- [ ] La pastille reste activée d'une recherche à l'autre, dans les onglets et dans une recherche enregistrée.
- [ ] Ligne de commande : `prospector-cli search "bail d'habitation" --meaning --format json` : champ `meaning` sur les résultats.
- [ ] En arabe et en thème Sonar : cartes lisibles, passage dans le bon sens.

### 8.4 Mesure sur un vrai site

- [ ] Sur un de tes vrais dossiers (quelques milliers de documents Word / PDF) : noter la durée affichée avant de cocher « Sens », puis la durée réelle du calcul ; me donner les deux chiffres (et le nombre de cœurs du PC).
- [ ] Pendant le calcul : le PC reste-t-il fluide (navigation, bureautique) ?
