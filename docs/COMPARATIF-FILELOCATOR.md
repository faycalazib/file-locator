# Prospector face à FileLocator Pro

> Comparaison faite le 27/09/2026, à partir du tableau officiel Lite/Pro de Mythicsoft, de l'aide de la version 9 et d'un test indépendant (sources en bas).
> **Objectif :** Prospector, gratuit, doit faire tout ce que fait FileLocator Pro (payant), puis davantage.
> **Plan d'exécution :** Étapes 5 à 8 de [SUIVI.md](SUIVI.md). Ce tableau est mis à jour à chaque lot livré.

**Légende :** ✅ présent dans Prospector · 🟡 en partie · ❌ absent · 🚫 écarté volontairement · (5.1) = lot du plan qui le traite

## 1. Ce que fait FileLocator Pro, et où en est Prospector

### Moteur de recherche

| FileLocator Pro | Prospector | Lot |
| --- | --- | --- |
| Moteur multi-thread, recherche indexée ou non | ✅ | |
| Lecteurs réseau (UNC, lecteurs mappés) | 🟡 Probablement OK, pas encore testé | 7.3 |
| AND / OR / NOT, regex dans le contenu | ✅ | |
| Regex multi-lignes | ✅ `(?s)` (le point passe les fins de ligne) et `(?m)` (`^`/`$` à chaque ligne), en tête de la regex | 5.5 |
| NEAR (proximité), LINES | ✅ Même syntaxe : `a NEAR b` (100 caractères), `NEAR:20`, chaînes `a NEAR b NEAR c`, `NOT a NEAR b` ; `LINES:3-5`, `LINES:10+` ; `LIKE mot` (faute tolérée pour ce mot seulement). Distance en caractères, l'arabe compte une lettre par lettre | 5.5 |
| LIKE (similarité) | ✅ Tolérance aux fautes | |
| Jokers DOS (`*.pdf`), critère « nom de fichier » séparé, recherche de dossiers | ✅ Plus : sans majuscules ni accents, exclusions `!`, regex, noms dans les archives | 5.1 |
| Dates relatives et absolues, date de création, date d'accès | ✅ Modifié, créé (indexé) ou dernier accès (lu sur le disque) ; préréglages ou période « du … au … » | 5.8 |
| Attributs (caché, lecture seule…) | ✅ Lecture seule, caché, système, lus sur le disque au moment de la recherche ; « caché » fait aussi parcourir les fichiers cachés en scan direct | 5.8 |
| Recherche par empreinte (MD5, SHA) | ✅ MD5 ou SHA-256 collé dans « + Affiner », calculé en dernier sur les seuls fichiers qui passent les autres critères. Plus : bouton « **Trouver les copies** » d'un résultat (même taille par l'index, puis même empreinte) | 5.8 |
| Critères lus dans un fichier texte | ✅ « + Liste de termes » : un fichier texte ou CSV, un terme par ligne (phrases, expressions régulières), « au moins un » ou « tous », combiné aux mots de la recherche ; une colonne et un total par terme dans l'export et le rapport PDF ; `--terms-file` en ligne de commande | 7.4 |
| Dossiers exclus | ✅ | |

### Formats

| FileLocator Pro | Prospector | Lot |
| --- | --- | --- |
| PDF, DOCX, XLSX / XLS / ODS, PPTX, MSG, PST, EML | ✅ | |
| OCR (PDF scannés, images) | ✅ Reconnaissance intégrée à Windows : rien à installer, arabe compris, pages scannées d'un PDF mixte | 5.2 |
| .doc et .ppt (anciens formats Office), RTF, ODT / ODP, EPUB | ✅ Lecteurs en Rust, sans Office ni IFilter ; les notes du présentateur sont écartées | 5.3 |
| OneNote, DWG, MOBI | ❌ Plus tard, selon la demande | |
| Pièces jointes des e-mails, OST, MBOX (Thunderbird) | ✅ Pièces jointes de .msg, .eml, .pst/.ost (y compris un ZIP ou un message joint), MBOX. OST : même lecteur que PST, à confirmer sur de vrais fichiers | 5.4 |
| PDF protégés | 🟡 Les PDF protégés contre la copie ou l'impression sont lus ; ceux qui demandent un mot de passe à l'ouverture restent comptés comme « chiffrés » | 5.3 |
| ZIP, 7z, RAR | ✅ | |
| JAR, TAR, GZ, BZ2 | ✅ Y compris .tar.gz / .tgz / .tar.bz2, et imbriqués | 5.3 |
| CAB, ISO | ❌ Plus tard | |
| Extraire un fichier d'une archive | ✅ « Extraire vers… » et « Ouvrir » le fichier lui-même, depuis une archive (ZIP, 7z, RAR, TAR, GZ, imbriquées comprises) **ou un e-mail** (pièce jointe d'un .msg, .eml, .pst, mbox) ; la copie groupée extrait aussi | 6.7 |

### Index

| FileLocator Pro | Prospector | Lot |
| --- | --- | --- |
| Plusieurs index, mise à jour automatique, suivi en temps réel | ✅ Suivi en direct automatique, sans planificateur à régler | |
| Groupes d'index | ✅ Groupes de sites nommés et colorés en haut du panneau, `Ctrl+1…9` (et `Ctrl+0` = tous), clic droit pour renommer / mettre à jour / supprimer ; aussi `--group` en ligne de commande | 7.2 |
| Index partagé sur le réseau | ✅ Dossier des index sur un partage : un PC le tient à jour, les autres le lisent et voient ses mises à jour aussitôt ; relais automatique si ce PC s'éteint ; lecteurs réseau convertis en chemins `\\serveur\…` ; recherches, alertes et groupes propres à chaque PC | 7.3 |

### Exploiter les résultats

| FileLocator Pro | Prospector | Lot |
| --- | --- | --- |
| Surlignage, lignes autour, fichier entier, favoris | ✅ | |
| Chercher dans les résultats | ✅ « Chercher dans ces résultats » : les recherches suivantes ne regardent que ces fichiers (moteur : index et scan direct), jusqu'à ce qu'on retire la pastille | 5.6 |
| Filtre instantané | ✅ Champ au-dessus de la liste : filtre par nom ou dossier pendant la frappe | 5.6 |
| Historique ← →, enregistrer et rouvrir des résultats | ✅ Alt+← / Alt+→ (résultats compris, sans relancer), fichier `.prospector` (Ctrl+S / Ctrl+O) qui garde la recherche et les passages | 5.6 |
| Onglets (plusieurs recherches) | ✅ Jusqu'à 12 onglets, chacun avec son historique ; un scan direct continue dans un onglet caché | 5.6 |
| Miniatures des images | ✅ Miniatures dans la liste, grande image dans l'aperçu, y compris pour une image dans une archive ou jointe à un e-mail (6.7). Plus : **les mots trouvés sont entourés sur l'image** (OCR de Windows), texte lu dessous | 6.6 |

### Rapports et export

| FileLocator Pro | Prospector | Lot |
| --- | --- | --- |
| Impression, CSV | ✅ Plus JSON et le rapport PDF | |
| Export HTML, CSV compatible Excel, choix des colonnes | ✅ Excel (séparateur « ; » ou « , » selon la langue, UTF-8), page HTML autonome (extraits surlignés, arabe de droite à gauche), JSON ; colonnes au choix, dont une par mot-clé et par donnée détectée ; fichiers affichés ou tous les fichiers trouvés | 6.4 |
| Rapport par mot-clé | ✅ Occurrences de chaque terme (mot, phrase, regex, LIKE) dans chaque fichier, totaux par terme ; en Excel, HTML, JSON et dans le rapport PDF | 6.4 |
| Copie groupée vers un dossier ou un ZIP | ✅ Arborescence gardée ou à plat, aucun fichier écrasé (« nom (2) »), progression et arrêt, bilan des fichiers illisibles | 6.4 |

### Programmation et intégration

| FileLocator Pro | Prospector | Lot |
| --- | --- | --- |
| Ligne de commande, outil console | ✅ `prospector-cli` : `search` (dans les index, instantané), `scan` (sans index, résultats au fil de l'eau), `index`, `sites` ; tous les filtres de l'application ; sortie texte, JSON, JSON Lines ou CSV ; codes de sortie pour les scripts ; ajout au PATH en un clic dans les Réglages | 7.1 |
| Ouvrir dans l'éditeur à la bonne ligne | ✅ Bouton « Ouvrir à la ligne N » (occurrence active) et numéros de ligne cliquables ; VS Code, VSCodium, Cursor, Notepad++, Sublime Text détectés, ou commande personnalisée `{file}` / `{line}` | 5.7 |
| Clic droit dans l'Explorateur | ✅ « Chercher avec Prospector » sur un dossier, le fond d'un dossier ou un disque. Plus : **instantané** si le dossier est dans un site indexé (FileLocator relit toujours le disque). Windows 11 : sous « Afficher plus d'options » (le menu court exige un paquet signé) | 5.7 |
| Une seule instance de l'application | ✅ Un 2ᵉ lancement ramène la fenêtre ; double-clic sur un fichier `.prospector` = ouvert dans la fenêtre existante | 5.7 |
| Alertes sur événements | ✅ Remplacé par mieux : alertes sur une recherche enregistrée (notification Windows dès qu'un fichier se met à correspondre, même pour les changements faits Prospector fermé), Prospector veille près de l'horloge | 6.1 |
| Interfaces de programmation .NET / COM | 🚫 Écarté : la ligne de commande couvre l'automatisation | |
| Règles imposées par un administrateur, modèles d'export XSLT | 🚫 Écarté : réservé aux services informatiques d'entreprise | |

### Langues

| FileLocator Pro | Prospector |
| --- | --- |
| Interface en allemand, français, espagnol, hindi, chinois | ✅ Anglais, français, espagnol et **arabe (de droite à gauche)** |
| Aide traduite | ✅ Guide en 4 langues |

## 2. Ce que Prospector a déjà en plus

- **Recherche multilingue intelligente** :
  - accents et voyelles arabes ignorés ;
  - pluriels regroupés par langue ;
  - langue de chaque document détectée, et filtrable ;
  - fautes de frappe tolérées, mais signalées à part.
- **Arabe et écriture de droite à gauche** dans l'interface, le guide et l'installeur.
- **Résultats classés par pertinence.**
- **Suivi en direct** sans rien régler, avec rattrapage des changements faits pendant que l'application était fermée.
- **Aperçu riche** : vrais tableaux Excel, diapositives, code en couleur.
- **Doublons exacts et presque identiques** : fichiers identiques (même sous un autre nom) et documents dont le texte se ressemble, même entre formats (un .docx et sa version .txt) ; place récupérable, mise à la Corbeille sûre (6.5).
- **Compteurs par critère** : combien de fichiers par type, langue, année et site, sur tout ce qui est trouvé, directement sur les filtres ; un clic filtre (6.3).
- **Mots entourés sur les images** : dans l'aperçu d'une photo ou d'un scan, les mots trouvés sont encadrés à leur place (6.6), même pour une image jointe à un e-mail ou rangée dans une archive (6.7).
- **Mode portable avec index** (6.8) : Prospector et ses index sur une clé USB, sans rien écrire sur le PC ; si la clé change de lettre d'un PC à l'autre, les chemins suivent sans rien réindexer.
- **Pièces jointes extraites en un clic** : « Extraire vers… » fonctionne aussi pour une pièce jointe d'e-mail, y compris dans une boîte Outlook PST ou Thunderbird (6.7).
- **Détecteurs validés et audit des données personnelles** : 13 types (IBAN, carte, RIB, BIC, e-mail, téléphone, IP, montant, TVA, SIREN/SIRET, sécurité sociale, DNI/NIE, passeport), vérifiés par leur clé de contrôle ; totaux par type, rapport PDF aux numéros masqués (6.2). FileLocator Pro ne propose que des expressions régulières à écrire soi-même.
- **Alertes sur une recherche enregistrée** : notification Windows dès qu'un nouveau fichier correspond, sans rien planifier (6.1).
- **Raccourci global** à la Spotlight.
- **« Trouver les copies » en un clic** depuis un résultat, rapide même sur de gros sites (la taille, déjà dans l'index, élimine presque tout avant le calcul d'empreinte).
- **Clic droit « Chercher avec Prospector » instantané** : un dossier déjà indexé est cherché dans l'index, pas relu sur le disque.
- **Mises à jour automatiques.** Le test indépendant note qu'elles sont désactivées par défaut dans FileLocator.
- **Interface moderne** : 2 thèmes, 3 mises en page, affichage net sur écran haute résolution (un défaut relevé chez FileLocator).
- **Gratuit, en une seule version**, sans formule Lite ou Pro.

## 3. Recherche par le sens (Étape 8)

- **Recherche par le sens et entre langues**, par une IA locale optionnelle (module de 77 Mo, rien ne quitte le PC) : « bail d'habitation » trouve aussi *عقد إيجار* et *lease agreement* sans mot commun ; les résultats trouvés par le sens montrent leur passage le plus proche ; calculée en arrière-plan, site par site ; aucun équivalent chez FileLocator Pro.

## Sources

- [FileLocator Pro, page d'accueil](https://www.mythicsoft.com/filelocatorpro/)
- [FileLocator Pro, informations et tableau Lite / Pro](https://www.mythicsoft.com/filelocatorpro/information/)
- [Aide de FileLocator Pro v9](https://help.mythicsoft.com/filelocatorpro/v9/en/hmcontent.htm)
- [Aide : Advanced Interface](https://help.mythicsoft.com/filelocatorpro/en/advanced_criteria.htm)
- [Test d'Agent Ransack et FileLocator Pro, ctrl.blog](https://www.ctrl.blog/entry/review-filelocator/)
- [Mythicsoft, Lite Mode](https://blog.mythicsoft.com/lite-mode/)
