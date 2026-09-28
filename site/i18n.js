/*
 * Texts of the download page and the user guide, in the 4 languages of the
 * app. Language: ?lang= in the address, then the browser language, then
 * English. Elements carry data-i18n="key" (text) or data-i18n-html="key"
 * (HTML from these strings only, never from the network).
 */
const STRINGS = {
  en: {
    'meta.title': 'Prospector — find any word in your files',
    'meta.guideTitle': 'Prospector — user guide',
    'nav.download': 'Download',
    'nav.guide': 'User guide',
    ribbon: 'My dig notebook',
    tagline: 'Find any word in your files. In milliseconds, without the Internet.',
    'download.button': 'Download for Windows',
    'download.meta': 'Windows 10 and 11 · 64-bit · free, no account',
    'download.version': 'Version {version} · {size}',
    'download.portable': 'Portable version (ZIP, nothing to install, for a USB drive)',
    'download.all': 'All versions and the MSI installers',
    'warning.title': 'The first time you open the installer',
    'warning.text': 'Windows may show “Windows protected your PC”, because Prospector is not yet signed by a paid certificate. Click <b>More info</b>, then <b>Run anyway</b>. This only happens once.',
    'f1.title': 'All your documents',
    'f1.text': 'PDF, Word (.docx, .doc, .rtf, .odt), Excel, PowerPoint (.pptx, .ppt, .odp), EPUB books, text and code, e-mails with their attachments (Outlook .msg, .pst, .ost, .eml, Thunderbird mbox), ZIP, RAR, 7z, TAR and GZ archives. Text in images and scanned PDFs is read too (OCR).',
    'f2.title': 'Instant and up to date',
    'f2.text': 'Results in a few milliseconds. Watched folders stay up to date by themselves.',
    'f3.title': 'Four languages',
    'f3.text': 'English, French, Spanish and Arabic, for the interface and for the search: accents and Arabic vowels do not matter.',
    'f4.title': 'Private',
    'f4.text': 'Everything stays on your computer. Nothing is sent anywhere.',
    'footer.licence': 'Prospector is free software (MIT licence).',
    'footer.notices': 'Third-party licences',
    'guide.title': 'User guide',
    'guide.lead': 'Everything Prospector can do, in a few minutes of reading.',
    'guide.toc': 'Contents',
    's.install.t': 'Install Prospector',
    's.install.b': '<ol><li>Download the installer from the <a href="index.html">download page</a> and open it.</li><li>If Windows shows “Windows protected your PC”, click <b>More info</b>, then <b>Run anyway</b> (Prospector is not yet signed by a paid certificate).</li><li>Choose the language of the installer, then follow the steps. No administrator rights are needed: Prospector is installed for your user account only.</li></ol><p>Prospector uses Microsoft Edge WebView2, already present on Windows 10 and 11. If it is missing, the installer adds it (Internet needed once).</p>',
    's.site.t': 'Your first dig site',
    's.site.b': '<p>A <b>dig site</b> is a folder that Prospector indexes (reads once, to answer instantly afterwards). Click <b>+</b> in <i>Dig sites</i>, then choose a folder: your documents, a network share, a USB disk…</p><p>The first indexing reads every file; its progress is shown under the site. Afterwards the site shows <b>Live watch</b>: added, changed or deleted files are taken into account within seconds, and changes made while Prospector was closed are caught up at startup.</p><p><b>Groups</b>: tick some sites, then <b>+ Group</b> at the top of the panel gives them a name (“Clients”, “Code”). A click on a group ticks exactly its sites; <kbd>Ctrl</kbd>+<kbd>1</kbd>…<kbd>9</kbd> does the same from the keyboard, <kbd>Ctrl</kbd>+<kbd>0</kbd> ticks every site. Right-click a group to rename it, give it the ticked sites or remove it. The command line uses them too: <code>--group Clients</code>.</p>',
    's.search.t': 'Search',
    's.search.b': '<p>Type your words and press <kbd>Enter</kbd> (or <b>Dig!</b>). Results show the file, its folder and the passages where the words appear, <mark>highlighted</mark>.</p><table><tr><th>You type</th><th>Prospector finds</th></tr><tr><td><code>contract renewal</code></td><td>files containing both words</td></tr><tr><td><code>"notice period"</code> or <code>«…»</code></td><td>the exact phrase</td></tr><tr><td><code>contract OR agreement</code></td><td>one word or the other</td></tr><tr><td><code>contract NOT draft</code> or <code>-draft</code></td><td>files without “draft”</td></tr><tr><td><code>/INV-\\d{4}/</code></td><td>a regular expression</td></tr><tr><td><code>contract NEAR termination</code>, <code>NEAR:20</code></td><td>both words within 100 (or 20) characters</td></tr><tr><td><code>LIKE necessary</code></td><td>this word even with a typo, option off</td></tr><tr><td><code>LINES:1-5 invoice</code>, <code>LINES:10+</code></td><td>only in those lines of each file</td></tr></table><p>Operators are written in capitals: <code>OR</code>, <code>AND</code>, <code>NOT</code>, <code>NEAR</code>, <code>LIKE</code>, <code>LINES</code>. Accents and Arabic vowels are ignored: “resume” finds “résumé”, “عقد” finds “عَقْد”.</p>',
    's.options.t': 'Search options',
    's.options.b': '<ul><li><b>Typo tolerance</b>: also finds words with a typing mistake. These matches are only underlined, and counted apart.</li><li><kbd>Aa</kbd> <b>Match case</b>: “Contract” no longer finds “contract”.</li><li><kbd>ab</kbd> <b>Whole word</b> (on by default): turn it off to find “contract” inside “subcontracts”.</li><li><kbd>.*</kbd> <b>Regular expression</b>: the whole input is read as a regular expression.</li></ul>',
    's.filters.t': 'Filters',
    's.filters.b': '<p>The chips under the search box keep only some kinds of files (PDF, Word, Excel…). <b>+ Refine</b> adds the size, the modification date, the language of the document and the <b>excluded folders</b> (a folder path, or a pattern such as <code>**\\node_modules</code>).</p><p><b>+ File name</b> adds a second box, <i>Named</i>: <code>*.pdf</code>, <code>invoice-2024-??.xlsx</code>, several patterns separated by <code>;</code>, <code>!draft*</code> to exclude, or <code>/regex/</code>. Case and accents do not matter. Leave the text box empty to list files by name only (without opening them). The <b>Folders</b> option looks for folders instead of files. A document inside an archive answers to its own name and to the archive’s (<code>*.rar</code> finds the documents of every RAR).</p><p><b>+ Term list</b> reads a text or CSV file, one term per line (the first column of a CSV; <code>#</code> for comments; <code>/…/</code> for a regular expression): files holding <b>at least one</b> of them, or <b>all</b> of them, besides the words of the search. Click the chip to see the terms or change the mode; the export and the PDF report count each term. From the command line: <code>--terms-file list.txt</code>.</p>',
    's.live.t': 'Live scan',
    's.live.b': '<p>The <b>Live scan</b> chip searches the folders directly, without the index: slower, but it always reads the current content and works on folders that are not indexed yet. Results arrive as they are found; <b>Stop</b> interrupts the scan.</p>',
    's.preview.t': 'Preview',
    's.preview.b': '<p>Select a result to read it on the right, with every match highlighted. Move from one match to the next with <kbd>↑</kbd> <kbd>↓</kbd> or <kbd>F3</kbd> / <kbd>Shift+F3</kbd>. Excel sheets are shown as tables, PowerPoint slides as cards, code with colors.</p><p><b>Open</b> opens the file with its usual program: for a file inside an archive or attached to an e-mail, that file itself (a message of a mailbox opens the mailbox). <b>Extract to…</b> saves such a file wherever you like. <b>Show in folder</b> opens the Explorer on the file, or on its archive or mailbox.</p>',
    's.saved.t': 'Saved searches',
    's.saved.b': '<p>The <b>★</b> button saves the search on screen with a name: the words, the mode, the options, the filters and the sites. Saved searches are listed in the side panel: one click runs one again. Your last searches are listed there too.</p>',
    's.results.t': 'Tabs, history and results',
    's.results.b': '<ul><li><b>Tabs</b>: <kbd>Ctrl+T</kbd> opens a new search next to the current one, <kbd>Ctrl+Tab</kbd> switches, <kbd>Ctrl+W</kbd> closes. A live scan goes on in a tab you leave.</li><li><kbd>Alt+←</kbd> / <kbd>Alt+→</kbd> go back to the previous search and its results, without searching again.</li><li><b>Filter the list</b>: the field above the results keeps only the files whose name or folder contains what you type.</li><li><b>Search within these results</b>: the next searches only look at the files listed, until you remove the chip.</li><li><kbd>Ctrl+S</kbd> saves the results to a <code>.prospector</code> file; <kbd>Ctrl+O</kbd> reopens it in a tab, passages included.</li></ul>',
    's.shortcut.t': 'Global shortcut',
    's.shortcut.b': '<p>From any application, <kbd>Ctrl+Shift+Space</kbd> brings Prospector to the front, ready to type. Press it again to send it back. Change it (or turn it off) in <b>Settings → Global shortcut</b>.</p>',
    's.export.t': 'Export and PDF report',
    's.export.b': '<p>The <b>⤓</b> button exports the results as CSV (spreadsheet) or JSON, or prints a <b>PDF report</b>: the search, its options and filters, then every file with its highlighted passages. In the print window, choose <b>Microsoft Print to PDF</b>.</p>',
    's.settings.t': 'Settings',
    's.settings.b': '<p>The gear at the bottom of the side panel: language of the interface, layout (Log, Ledger, Strata), theme (Colored pencils or Sonar), global shortcut, updates, and the <b>index folder</b>. Indexes can take space: move them to the disk of your choice, your sites are kept.</p><p><b>Text in images (OCR)</b>: Prospector reads the text of images and scanned PDFs with the text recognition built into Windows, in the languages installed in Windows. Turn it off to index folders full of photos faster.</p><p><b>Shared index</b>: choose a folder on a network share as the index folder on several PCs. The first Prospector open keeps it up to date; the others only read it (“Shared index · kept up to date by …” in the side panel) and see its updates at once. If that PC stays off, another one takes over within 2 minutes. Saved searches, alerts and groups stay on each PC. Add sites with their network path (a mapped drive is converted), so that every PC can open the files.</p>',
    's.cli.t': 'Command line',
    's.cli.b': '<p><code>prospector-cli</code> searches the same sites as the app, from a script or a terminal. To use it anywhere, tick <b>Command line in the PATH</b> in Settings → Windows integration (for your account only), then open a new terminal.</p><pre><code>prospector-cli search "contrat NEAR signé" --site Clients --kind pdf --format csv &gt; resultats.csv\nprospector-cli scan "facture" D:\\Archives --name "*.pdf" --since 2024-01-01\nprospector-cli index --all\nprospector-cli sites</code></pre><p><code>search</code> uses the indexes, <code>scan</code> reads folders directly, <code>index</code> brings sites up to date (close the app first: it already does it), <code>sites</code> lists them. Every filter of the app has its option (<code>--help</code>). Output: <code>--format text</code>, <code>json</code>, <code>jsonl</code> or <code>csv</code>. Exit code 0 when something is found, 1 when nothing is, 2 for a wrong command, 3 when the data cannot be read.</p>',
    's.portable.t': 'Portable version',
    's.portable.b': '<p>The portable ZIP (download page) runs from a USB drive without installing anything: settings and indexes stay in <b>ProspectorData</b> next to the program, nothing is written on the PC. If the drive gets another letter on another PC, Prospector follows at once, without indexing again. The Explorer menu, starting with Windows and automatic updates are not offered; a new version is announced with a link to download it.</p>',
    's.updates.t': 'Updates',
    's.updates.b': '<p>At startup, Prospector checks whether a new version exists. If so, a card offers to install it and restart. You can check by hand, or turn the automatic check off, in <b>Settings → Updates</b>.</p>',
    's.antivirus.t': 'Slow indexing and antivirus',
    's.antivirus.b': '<p>An antivirus scans every file written by Prospector, which can slow indexing down. To speed it up, add the index folder (shown in <b>Settings → Index folder</b>) to the exclusions: <b>Windows Security → Virus &amp; threat protection → Manage settings → Exclusions → Add a folder</b>. Only do so if you trust the documents you index.</p><p>On an external hard disk, indexing is limited by the disk itself (about 500 small files per second).</p>',
    's.privacy.t': 'Privacy',
    's.privacy.b': '<p>Prospector works entirely offline: your documents and indexes never leave your computer. The only connection is the update check (GitHub), which you can turn off.</p>',
    's.uninstall.t': 'Uninstall',
    's.uninstall.b': '<p><b>Settings → Apps → Installed apps → Prospector → Uninstall</b>. The index folder is kept, so a new installation finds your sites again; delete it by hand to free the space.</p>',
  },

  fr: {
    'meta.title': 'Prospector — retrouvez n’importe quel mot dans vos fichiers',
    'meta.guideTitle': 'Prospector — guide d’utilisation',
    'nav.download': 'Télécharger',
    'nav.guide': 'Guide',
    ribbon: 'Mon carnet de fouille',
    tagline: 'Retrouvez n’importe quel mot dans vos fichiers. En quelques millisecondes, sans Internet.',
    'download.button': 'Télécharger pour Windows',
    'download.meta': 'Windows 10 et 11 · 64 bits · gratuit, sans compte',
    'download.version': 'Version {version} · {size}',
    'download.portable': 'Version portable (ZIP, rien à installer, pour une clé USB)',
    'download.all': 'Toutes les versions et les installeurs MSI',
    'warning.title': 'À la première ouverture de l’installeur',
    'warning.text': 'Windows peut afficher « Windows a protégé votre ordinateur », car Prospector n’est pas encore signé par un certificat payant. Cliquez sur <b>Informations complémentaires</b>, puis sur <b>Exécuter quand même</b>. Cela n’arrive qu’une fois.',
    'f1.title': 'Tous vos documents',
    'f1.text': 'PDF, Word (.docx, .doc, .rtf, .odt), Excel, PowerPoint (.pptx, .ppt, .odp), livres EPUB, texte et code, e-mails et leurs pièces jointes (Outlook .msg, .pst, .ost, .eml, Thunderbird mbox), archives ZIP, RAR, 7z, TAR et GZ. Le texte des images et des PDF scannés est lu aussi (OCR).',
    'f2.title': 'Instantané et toujours à jour',
    'f2.text': 'Des résultats en quelques millisecondes. Les dossiers suivis se mettent à jour tout seuls.',
    'f3.title': 'Quatre langues',
    'f3.text': 'Français, anglais, espagnol et arabe, pour l’interface comme pour la recherche : accents et voyelles arabes n’y changent rien.',
    'f4.title': 'Confidentiel',
    'f4.text': 'Tout reste sur votre ordinateur. Rien n’est envoyé nulle part.',
    'footer.licence': 'Prospector est un logiciel libre et gratuit (licence MIT).',
    'footer.notices': 'Licences des composants',
    'guide.title': 'Guide d’utilisation',
    'guide.lead': 'Tout ce que Prospector sait faire, en quelques minutes de lecture.',
    'guide.toc': 'Sommaire',
    's.install.t': 'Installer Prospector',
    's.install.b': '<ol><li>Téléchargez l’installeur depuis la <a href="index.html">page de téléchargement</a> et ouvrez-le.</li><li>Si Windows affiche « Windows a protégé votre ordinateur », cliquez sur <b>Informations complémentaires</b>, puis sur <b>Exécuter quand même</b> (Prospector n’est pas encore signé par un certificat payant).</li><li>Choisissez la langue de l’installeur, puis suivez les étapes. Aucun droit administrateur n’est nécessaire : Prospector s’installe pour votre compte uniquement.</li></ol><p>Prospector utilise Microsoft Edge WebView2, déjà présent sur Windows 10 et 11. S’il manque, l’installeur l’ajoute (Internet nécessaire une fois).</p>',
    's.site.t': 'Votre premier site de fouille',
    's.site.b': '<p>Un <b>site de fouille</b> est un dossier que Prospector indexe (il le lit une fois, pour répondre instantanément ensuite). Cliquez sur <b>+</b> dans <i>Sites de fouille</i>, puis choisissez un dossier : vos documents, un partage réseau, un disque USB…</p><p>La première indexation lit tous les fichiers ; sa progression s’affiche sous le site. Ensuite, le site affiche <b>Suivi en direct</b> : les fichiers ajoutés, modifiés ou supprimés sont pris en compte en quelques secondes, et les changements faits pendant que Prospector était fermé sont rattrapés au démarrage.</p><p><b>Groupes</b> : cochez quelques sites, puis <b>+ Groupe</b> en haut du panneau leur donne un nom (« Clients », « Code »). Un clic sur un groupe coche exactement ses sites ; <kbd>Ctrl</kbd>+<kbd>1</kbd>…<kbd>9</kbd> fait de même au clavier, <kbd>Ctrl</kbd>+<kbd>0</kbd> coche tous les sites. Clic droit sur un groupe : le renommer, lui donner les sites cochés ou le supprimer. La ligne de commande s’en sert aussi : <code>--group Clients</code>.</p>',
    's.search.t': 'Chercher',
    's.search.b': '<p>Tapez vos mots et appuyez sur <kbd>Entrée</kbd> (ou <b>Fouiller !</b>). Chaque résultat montre le fichier, son dossier et les passages où les mots apparaissent, <mark>surlignés</mark>.</p><table><tr><th>Vous tapez</th><th>Prospector trouve</th></tr><tr><td><code>contrat renouvellement</code></td><td>les fichiers qui contiennent les deux mots</td></tr><tr><td><code>"préavis de trois mois"</code> ou <code>«…»</code></td><td>la phrase exacte</td></tr><tr><td><code>contrat OR convention</code></td><td>l’un ou l’autre mot</td></tr><tr><td><code>contrat NOT brouillon</code> ou <code>-brouillon</code></td><td>les fichiers sans « brouillon »</td></tr><tr><td><code>/INV-\\d{4}/</code></td><td>une expression régulière</td></tr><tr><td><code>contrat NEAR résiliation</code>, <code>NEAR:20</code></td><td>les deux mots à moins de 100 (ou 20) caractères</td></tr><tr><td><code>LIKE nécessaire</code></td><td>ce mot même avec une faute, option désactivée</td></tr><tr><td><code>LINES:1-5 facture</code>, <code>LINES:10+</code></td><td>seulement dans ces lignes de chaque fichier</td></tr></table><p>Les opérateurs s’écrivent en majuscules : <code>OR</code>, <code>AND</code>, <code>NOT</code>, <code>NEAR</code>, <code>LIKE</code>, <code>LINES</code>. Les accents et les voyelles arabes sont ignorés : « resume » trouve « résumé », « عقد » trouve « عَقْد ».</p>',
    's.options.t': 'Options de recherche',
    's.options.b': '<ul><li><b>Tolérance aux fautes</b> : trouve aussi les mots avec une faute de frappe. Ces correspondances sont seulement soulignées, et comptées à part.</li><li><kbd>Aa</kbd> <b>Respecter la casse</b> : « Contrat » ne trouve plus « contrat ».</li><li><kbd>ab</kbd> <b>Mot entier</b> (activé par défaut) : désactivez-le pour trouver « contrat » dans « sous-contrats ».</li><li><kbd>.*</kbd> <b>Expression régulière</b> : toute la saisie est lue comme une expression régulière.</li></ul>',
    's.filters.t': 'Filtres',
    's.filters.b': '<p>Les pastilles sous la zone de recherche ne gardent que certains types de fichiers (PDF, Word, Excel…). <b>+ Affiner</b> ajoute la taille, la date de modification, la langue du document et les <b>dossiers exclus</b> (un chemin de dossier, ou un motif comme <code>**\\node_modules</code>).</p><p><b>+ Nom de fichier</b> ajoute une 2ᵉ case, <i>Nommé</i> : <code>*.pdf</code>, <code>facture-2024-??.xlsx</code>, plusieurs motifs séparés par <code>;</code>, <code>!brouillon*</code> pour exclure, ou <code>/regex/</code>. Majuscules et accents n’y changent rien. Laissez la zone de texte vide pour lister des fichiers par leur nom seul (sans les ouvrir). L’option <b>Dossiers</b> cherche des dossiers au lieu de fichiers. Un document dans une archive répond à son nom et à celui de l’archive (<code>*.rar</code> trouve les documents de tous les RAR).</p><p><b>+ Liste de termes</b> lit un fichier texte ou CSV, un terme par ligne (la première colonne d’un CSV ; <code>#</code> pour un commentaire ; <code>/…/</code> pour une expression régulière) : les fichiers qui contiennent <b>au moins un</b> de ces termes, ou <b>tous</b>, en plus des mots de la recherche. Un clic sur la pastille montre les termes et change le mode ; l’export et le rapport PDF comptent chaque terme. En ligne de commande : <code>--terms-file liste.txt</code>.</p>',
    's.live.t': 'Scan direct',
    's.live.b': '<p>La pastille <b>Scan direct</b> cherche directement dans les dossiers, sans l’index : plus lent, mais toujours à jour, et possible sur des dossiers pas encore indexés. Les résultats arrivent au fur et à mesure ; <b>Arrêter</b> interrompt le scan.</p>',
    's.preview.t': 'Aperçu',
    's.preview.b': '<p>Sélectionnez un résultat pour le lire à droite, chaque correspondance surlignée. Passez de l’une à l’autre avec <kbd>↑</kbd> <kbd>↓</kbd> ou <kbd>F3</kbd> / <kbd>Maj+F3</kbd>. Les feuilles Excel s’affichent en tableaux, les diapositives PowerPoint en cartes, le code en couleurs.</p><p><b>Ouvrir</b> ouvre le fichier avec son programme habituel : pour un fichier dans une archive ou joint à un e-mail, ce fichier lui-même (un message d’une boîte mail ouvre la boîte). <b>Extraire vers…</b> enregistre un tel fichier où vous voulez. <b>Afficher dans le dossier</b> ouvre l’Explorateur sur le fichier, ou sur son archive ou sa boîte mail.</p>',
    's.saved.t': 'Recherches enregistrées',
    's.saved.b': '<p>Le bouton <b>★</b> enregistre la recherche affichée sous un nom : les mots, le mode, les options, les filtres et les sites. Les recherches enregistrées sont listées dans le panneau latéral : un clic en relance une. Vos dernières recherches y sont aussi.</p>',
    's.results.t': 'Onglets, historique et résultats',
    's.results.b': '<ul><li><b>Onglets</b> : <kbd>Ctrl+T</kbd> ouvre une nouvelle recherche à côté de l’actuelle, <kbd>Ctrl+Tab</kbd> passe de l’une à l’autre, <kbd>Ctrl+W</kbd> ferme. Un scan direct continue dans un onglet que vous quittez.</li><li><kbd>Alt+←</kbd> / <kbd>Alt+→</kbd> reviennent à la recherche précédente et à ses résultats, sans relancer.</li><li><b>Filtrer la liste</b> : le champ au-dessus des résultats ne garde que les fichiers dont le nom ou le dossier contient ce que vous tapez.</li><li><b>Chercher dans ces résultats</b> : les recherches suivantes ne regardent que les fichiers affichés, jusqu’à ce que vous retiriez la pastille.</li><li><kbd>Ctrl+S</kbd> enregistre les résultats dans un fichier <code>.prospector</code> ; <kbd>Ctrl+O</kbd> le rouvre dans un onglet, passages compris.</li></ul>',
    's.shortcut.t': 'Raccourci global',
    's.shortcut.b': '<p>Depuis n’importe quelle application, <kbd>Ctrl+Maj+Espace</kbd> fait passer Prospector au premier plan, prêt à taper. Appuyez de nouveau pour le renvoyer. Changez-le (ou désactivez-le) dans <b>Réglages → Raccourci global</b>.</p>',
    's.export.t': 'Export et rapport PDF',
    's.export.b': '<p>Le bouton <b>⤓</b> exporte les résultats en CSV (tableur) ou JSON, ou imprime un <b>rapport PDF</b> : la recherche, ses options et ses filtres, puis chaque fichier avec ses passages surlignés. Dans la fenêtre d’impression, choisissez <b>Microsoft Print to PDF</b>.</p>',
    's.settings.t': 'Réglages',
    's.settings.b': '<p>L’engrenage en bas du panneau latéral : langue de l’interface, mise en page (Journal, Registre, Strates), thème (Crayons de couleur ou Sonar), raccourci global, mises à jour, et le <b>dossier des index</b>. Les index peuvent prendre de la place : déplacez-les sur le disque de votre choix, vos sites sont conservés.</p><p><b>Texte des images (OCR)</b> : Prospector lit le texte des images et des PDF scannés avec la reconnaissance de texte intégrée à Windows, dans les langues installées dans Windows. Désactivez-la pour indexer plus vite des dossiers pleins de photos.</p><p><b>Index partagé</b> : choisissez un dossier d’un partage réseau comme dossier des index sur plusieurs PC. Le premier Prospector ouvert le tient à jour ; les autres le lisent seulement (« Index partagé · tenu à jour par … » dans le panneau latéral) et voient ses mises à jour aussitôt. Si ce PC reste éteint, un autre prend le relais sous 2 minutes. Recherches enregistrées, alertes et groupes restent propres à chaque PC. Ajoutez les sites par leur chemin réseau (un lecteur réseau est converti), pour que chaque PC puisse ouvrir les fichiers.</p>',
    's.cli.t': 'Ligne de commande',
    's.cli.b': '<p><code>prospector-cli</code> cherche dans les mêmes sites que l’application, depuis un script ou un terminal. Pour l’utiliser partout, cochez <b>Ligne de commande dans le PATH</b> dans Réglages → Intégration Windows (pour votre compte seulement), puis ouvrez un nouveau terminal.</p><pre><code>prospector-cli search "contrat NEAR signé" --site Clients --kind pdf --format csv &gt; resultats.csv\nprospector-cli scan "facture" D:\\Archives --name "*.pdf" --since 2024-01-01\nprospector-cli index --all\nprospector-cli sites</code></pre><p><code>search</code> utilise les index, <code>scan</code> lit les dossiers directement, <code>index</code> met les sites à jour (fermez d’abord l’application : elle le fait déjà), <code>sites</code> les liste. Chaque filtre de l’application a son option (<code>--help</code>). Sortie : <code>--format text</code>, <code>json</code>, <code>jsonl</code> ou <code>csv</code>. Code de sortie 0 si quelque chose est trouvé, 1 sinon, 2 pour une commande erronée, 3 si les données sont illisibles. Les messages de l’outil sont en anglais.</p>',
    's.portable.t': 'Version portable',
    's.portable.b': '<p>Le ZIP portable (page de téléchargement) fonctionne depuis une clé USB sans rien installer : réglages et index restent dans <b>ProspectorData</b>, à côté du programme, rien n’est écrit sur le PC. Si la clé change de lettre sur un autre PC, Prospector suit aussitôt, sans réindexer. Le menu de l’Explorateur, le lancement au démarrage et les mises à jour automatiques ne sont pas proposés ; une nouvelle version est annoncée avec un lien pour la télécharger.</p>',
    's.updates.t': 'Mises à jour',
    's.updates.b': '<p>Au démarrage, Prospector vérifie si une nouvelle version existe. Si oui, une carte propose de l’installer et de redémarrer. Vous pouvez vérifier à la main, ou désactiver la vérification automatique, dans <b>Réglages → Mises à jour</b>.</p>',
    's.antivirus.t': 'Indexation lente et antivirus',
    's.antivirus.b': '<p>Un antivirus analyse chaque fichier écrit par Prospector, ce qui peut ralentir l’indexation. Pour l’accélérer, ajoutez le dossier des index (indiqué dans <b>Réglages → Dossier des index</b>) aux exclusions : <b>Sécurité Windows → Protection contre les virus et menaces → Gérer les paramètres → Exclusions → Ajouter un dossier</b>. Ne le faites que si vous avez confiance dans les documents indexés.</p><p>Sur un disque dur externe, l’indexation est limitée par le disque lui-même (environ 500 petits fichiers par seconde).</p>',
    's.privacy.t': 'Confidentialité',
    's.privacy.b': '<p>Prospector fonctionne entièrement hors ligne : vos documents et vos index ne quittent jamais votre ordinateur. La seule connexion est la vérification des mises à jour (GitHub), que vous pouvez désactiver.</p>',
    's.uninstall.t': 'Désinstaller',
    's.uninstall.b': '<p><b>Paramètres → Applications → Applications installées → Prospector → Désinstaller</b>. Le dossier des index est conservé, pour qu’une nouvelle installation retrouve vos sites ; supprimez-le à la main pour libérer la place.</p>',
  },

  es: {
    'meta.title': 'Prospector — encuentre cualquier palabra en sus archivos',
    'meta.guideTitle': 'Prospector — guía de uso',
    'nav.download': 'Descargar',
    'nav.guide': 'Guía',
    ribbon: 'Mi cuaderno de excavación',
    tagline: 'Encuentre cualquier palabra en sus archivos. En milisegundos, sin Internet.',
    'download.button': 'Descargar para Windows',
    'download.meta': 'Windows 10 y 11 · 64 bits · gratis, sin cuenta',
    'download.version': 'Versión {version} · {size}',
    'download.portable': 'Versión portátil (ZIP, sin instalar, para una memoria USB)',
    'download.all': 'Todas las versiones y los instaladores MSI',
    'warning.title': 'La primera vez que abra el instalador',
    'warning.text': 'Windows puede mostrar «Windows protegió su PC», porque Prospector todavía no está firmado con un certificado de pago. Haga clic en <b>Más información</b> y luego en <b>Ejecutar de todas formas</b>. Solo ocurre una vez.',
    'f1.title': 'Todos sus documentos',
    'f1.text': 'PDF, Word (.docx, .doc, .rtf, .odt), Excel, PowerPoint (.pptx, .ppt, .odp), libros EPUB, texto y código, correos y sus adjuntos (Outlook .msg, .pst, .ost, .eml, Thunderbird mbox), archivos ZIP, RAR, 7z, TAR y GZ. También se lee el texto de las imágenes y los PDF escaneados (OCR).',
    'f2.title': 'Instantáneo y al día',
    'f2.text': 'Resultados en milisegundos. Las carpetas vigiladas se actualizan solas.',
    'f3.title': 'Cuatro idiomas',
    'f3.text': 'Español, inglés, francés y árabe, para la interfaz y para la búsqueda: los acentos y las vocales árabes no importan.',
    'f4.title': 'Privado',
    'f4.text': 'Todo se queda en su ordenador. No se envía nada a ninguna parte.',
    'footer.licence': 'Prospector es software libre y gratuito (licencia MIT).',
    'footer.notices': 'Licencias de terceros',
    'guide.title': 'Guía de uso',
    'guide.lead': 'Todo lo que Prospector sabe hacer, en unos minutos de lectura.',
    'guide.toc': 'Índice',
    's.install.t': 'Instalar Prospector',
    's.install.b': '<ol><li>Descargue el instalador desde la <a href="index.html">página de descarga</a> y ábralo.</li><li>Si Windows muestra «Windows protegió su PC», haga clic en <b>Más información</b> y luego en <b>Ejecutar de todas formas</b> (Prospector todavía no está firmado con un certificado de pago).</li><li>Elija el idioma del instalador y siga los pasos. No hacen falta permisos de administrador: Prospector se instala solo para su cuenta.</li></ol><p>Prospector usa Microsoft Edge WebView2, ya presente en Windows 10 y 11. Si falta, el instalador lo añade (se necesita Internet una vez).</p>',
    's.site.t': 'Su primer sitio de excavación',
    's.site.b': '<p>Un <b>sitio de excavación</b> es una carpeta que Prospector indexa (la lee una vez para responder al instante después). Haga clic en <b>+</b> en <i>Sitios de excavación</i> y elija una carpeta: sus documentos, una carpeta de red, un disco USB…</p><p>La primera indexación lee todos los archivos; su progreso aparece bajo el sitio. Después, el sitio muestra <b>Seguimiento en vivo</b>: los archivos añadidos, modificados o eliminados se tienen en cuenta en segundos, y los cambios hechos con Prospector cerrado se recuperan al iniciar.</p><p><b>Grupos</b>: marque algunos sitios y <b>+ Grupo</b>, arriba del panel, les da un nombre («Clientes», «Código»). Un clic en un grupo marca exactamente sus sitios; <kbd>Ctrl</kbd>+<kbd>1</kbd>…<kbd>9</kbd> hace lo mismo con el teclado y <kbd>Ctrl</kbd>+<kbd>0</kbd> marca todos los sitios. Clic derecho en un grupo: cambiarle el nombre, darle los sitios marcados o eliminarlo. La línea de comandos también los usa: <code>--group Clientes</code>.</p>',
    's.search.t': 'Buscar',
    's.search.b': '<p>Escriba sus palabras y pulse <kbd>Intro</kbd> (o <b>¡Excavar!</b>). Cada resultado muestra el archivo, su carpeta y los pasajes donde aparecen las palabras, <mark>resaltados</mark>.</p><table><tr><th>Usted escribe</th><th>Prospector encuentra</th></tr><tr><td><code>contrato renovación</code></td><td>los archivos que contienen las dos palabras</td></tr><tr><td><code>"plazo de preaviso"</code> o <code>«…»</code></td><td>la frase exacta</td></tr><tr><td><code>contrato OR convenio</code></td><td>una palabra u otra</td></tr><tr><td><code>contrato NOT borrador</code> o <code>-borrador</code></td><td>los archivos sin «borrador»</td></tr><tr><td><code>/INV-\\d{4}/</code></td><td>una expresión regular</td></tr><tr><td><code>contrato NEAR rescisión</code>, <code>NEAR:20</code></td><td>las dos palabras a menos de 100 (o 20) caracteres</td></tr><tr><td><code>LIKE necesario</code></td><td>esta palabra incluso con una errata, opción desactivada</td></tr><tr><td><code>LINES:1-5 factura</code>, <code>LINES:10+</code></td><td>solo en esas líneas de cada archivo</td></tr></table><p>Los operadores se escriben en mayúsculas: <code>OR</code>, <code>AND</code>, <code>NOT</code>, <code>NEAR</code>, <code>LIKE</code>, <code>LINES</code>. Se ignoran los acentos y las vocales árabes: «cancion» encuentra «canción», «عقد» encuentra «عَقْد».</p>',
    's.options.t': 'Opciones de búsqueda',
    's.options.b': '<ul><li><b>Tolerancia a errores</b>: encuentra también palabras con una errata. Esas coincidencias solo se subrayan y se cuentan aparte.</li><li><kbd>Aa</kbd> <b>Distinguir mayúsculas</b>: «Contrato» ya no encuentra «contrato».</li><li><kbd>ab</kbd> <b>Palabra completa</b> (activada por defecto): desactívela para encontrar «contrato» dentro de «subcontratos».</li><li><kbd>.*</kbd> <b>Expresión regular</b>: todo el texto se lee como una expresión regular.</li></ul>',
    's.filters.t': 'Filtros',
    's.filters.b': '<p>Las pastillas bajo el cuadro de búsqueda conservan solo algunos tipos de archivos (PDF, Word, Excel…). <b>+ Afinar</b> añade el tamaño, la fecha de modificación, el idioma del documento y las <b>carpetas excluidas</b> (una ruta de carpeta o un patrón como <code>**\\node_modules</code>).</p><p><b>+ Nombre de archivo</b> añade una 2.ª casilla, <i>Llamado</i>: <code>*.pdf</code>, <code>factura-2024-??.xlsx</code>, varios patrones separados por <code>;</code>, <code>!borrador*</code> para excluir, o <code>/regex/</code>. Las mayúsculas y los acentos no importan. Deje vacío el cuadro de texto para listar archivos solo por su nombre (sin abrirlos). La opción <b>Carpetas</b> busca carpetas en lugar de archivos. Un documento dentro de un archivo comprimido responde a su nombre y al del archivo (<code>*.rar</code> encuentra los documentos de todos los RAR).</p><p><b>+ Lista de términos</b> lee un archivo de texto o CSV, un término por línea (la primera columna de un CSV; <code>#</code> para un comentario; <code>/…/</code> para una expresión regular): los archivos que contienen <b>al menos uno</b> de esos términos, o <b>todos</b>, además de las palabras de la búsqueda. Un clic en la etiqueta muestra los términos y cambia el modo; la exportación y el informe PDF cuentan cada término. En línea de comandos: <code>--terms-file lista.txt</code>.</p>',
    's.live.t': 'Escaneo directo',
    's.live.b': '<p>La pastilla <b>Escaneo directo</b> busca directamente en las carpetas, sin el índice: más lento, pero siempre al día, y posible en carpetas aún no indexadas. Los resultados llegan a medida que se encuentran; <b>Detener</b> interrumpe el escaneo.</p>',
    's.preview.t': 'Vista previa',
    's.preview.b': '<p>Seleccione un resultado para leerlo a la derecha, con cada coincidencia resaltada. Pase de una a otra con <kbd>↑</kbd> <kbd>↓</kbd> o <kbd>F3</kbd> / <kbd>Mayús+F3</kbd>. Las hojas de Excel se muestran como tablas, las diapositivas de PowerPoint como tarjetas y el código en colores.</p><p><b>Abrir</b> abre el archivo con su programa habitual: para un archivo dentro de un archivo comprimido o adjunto a un correo, ese mismo archivo (un mensaje de un buzón abre el buzón). <b>Extraer en…</b> guarda ese archivo donde quiera. <b>Mostrar en la carpeta</b> abre el Explorador en el archivo, o en su archivo comprimido o buzón.</p>',
    's.saved.t': 'Búsquedas guardadas',
    's.saved.b': '<p>El botón <b>★</b> guarda la búsqueda actual con un nombre: las palabras, el modo, las opciones, los filtros y los sitios. Las búsquedas guardadas aparecen en el panel lateral: un clic vuelve a lanzar una. Sus últimas búsquedas también están ahí.</p>',
    's.results.t': 'Pestañas, historial y resultados',
    's.results.b': '<ul><li><b>Pestañas</b>: <kbd>Ctrl+T</kbd> abre una nueva búsqueda junto a la actual, <kbd>Ctrl+Tab</kbd> cambia de una a otra, <kbd>Ctrl+W</kbd> cierra. Un escaneo directo continúa en una pestaña que deja.</li><li><kbd>Alt+←</kbd> / <kbd>Alt+→</kbd> vuelven a la búsqueda anterior y a sus resultados, sin volver a buscar.</li><li><b>Filtrar la lista</b>: el campo sobre los resultados solo conserva los archivos cuyo nombre o carpeta contiene lo que escribe.</li><li><b>Buscar en estos resultados</b>: las búsquedas siguientes solo miran los archivos mostrados, hasta que quite la etiqueta.</li><li><kbd>Ctrl+S</kbd> guarda los resultados en un archivo <code>.prospector</code>; <kbd>Ctrl+O</kbd> lo vuelve a abrir en una pestaña, con los pasajes.</li></ul>',
    's.shortcut.t': 'Atajo global',
    's.shortcut.b': '<p>Desde cualquier aplicación, <kbd>Ctrl+Mayús+Espacio</kbd> pone Prospector en primer plano, listo para escribir. Púlselo de nuevo para ocultarlo. Cámbielo (o desactívelo) en <b>Ajustes → Atajo global</b>.</p>',
    's.export.t': 'Exportación e informe PDF',
    's.export.b': '<p>El botón <b>⤓</b> exporta los resultados en CSV (hoja de cálculo) o JSON, o imprime un <b>informe PDF</b>: la búsqueda, sus opciones y filtros, y luego cada archivo con sus pasajes resaltados. En la ventana de impresión, elija <b>Microsoft Print to PDF</b>.</p>',
    's.settings.t': 'Ajustes',
    's.settings.b': '<p>El engranaje al pie del panel lateral: idioma de la interfaz, diseño (Diario, Registro, Estratos), tema (Lápices de colores o Sonar), atajo global, actualizaciones y la <b>carpeta de índices</b>. Los índices pueden ocupar espacio: muévalos al disco que prefiera, sus sitios se conservan.</p><p><b>Texto de las imágenes (OCR)</b>: Prospector lee el texto de las imágenes y los PDF escaneados con el reconocimiento de texto integrado en Windows, en los idiomas instalados en Windows. Desactívelo para indexar más rápido carpetas llenas de fotos.</p><p><b>Índice compartido</b>: elija una carpeta de un recurso de red como carpeta de índices en varios PC. El primer Prospector abierto lo mantiene actualizado; los demás solo lo leen («Índice compartido · actualizado por …» en el panel lateral) y ven sus actualizaciones al instante. Si ese PC sigue apagado, otro toma el relevo en 2 minutos. Las búsquedas guardadas, alertas y grupos son propios de cada PC. Añada los sitios por su ruta de red (una unidad de red se convierte) para que cada PC pueda abrir los archivos.</p>',
    's.cli.t': 'Línea de comandos',
    's.cli.b': '<p><code>prospector-cli</code> busca en los mismos sitios que la aplicación, desde un script o una terminal. Para usarlo en cualquier lugar, marque <b>Línea de comandos en el PATH</b> en Ajustes → Integración con Windows (solo para su cuenta) y abra una terminal nueva.</p><pre><code>prospector-cli search "contrat NEAR signé" --site Clients --kind pdf --format csv &gt; resultats.csv\nprospector-cli scan "facture" D:\\Archives --name "*.pdf" --since 2024-01-01\nprospector-cli index --all\nprospector-cli sites</code></pre><p><code>search</code> usa los índices, <code>scan</code> lee las carpetas directamente, <code>index</code> actualiza los sitios (cierre antes la aplicación: ya lo hace), <code>sites</code> los lista. Cada filtro de la aplicación tiene su opción (<code>--help</code>). Salida: <code>--format text</code>, <code>json</code>, <code>jsonl</code> o <code>csv</code>. Código de salida 0 si se encuentra algo, 1 si no, 2 para un comando erróneo, 3 si los datos no se pueden leer. Los mensajes de la herramienta están en inglés.</p>',
    's.portable.t': 'Versión portátil',
    's.portable.b': '<p>El ZIP portátil (página de descarga) funciona desde una memoria USB sin instalar nada: ajustes e índices se quedan en <b>ProspectorData</b>, junto al programa, y no se escribe nada en el PC. Si la unidad cambia de letra en otro PC, Prospector la sigue al instante, sin reindexar. No se ofrecen el menú del Explorador, el inicio con Windows ni las actualizaciones automáticas; una nueva versión se anuncia con un enlace para descargarla.</p>',
    's.updates.t': 'Actualizaciones',
    's.updates.b': '<p>Al iniciar, Prospector comprueba si existe una versión nueva. Si la hay, una tarjeta propone instalarla y reiniciar. Puede comprobarlo a mano, o desactivar la comprobación automática, en <b>Ajustes → Actualizaciones</b>.</p>',
    's.antivirus.t': 'Indexación lenta y antivirus',
    's.antivirus.b': '<p>Un antivirus analiza cada archivo que escribe Prospector, lo que puede ralentizar la indexación. Para acelerarla, añada la carpeta de índices (indicada en <b>Ajustes → Carpeta de índices</b>) a las exclusiones: <b>Seguridad de Windows → Protección antivirus y contra amenazas → Administrar la configuración → Exclusiones → Agregar una carpeta</b>. Hágalo solo si confía en los documentos indexados.</p><p>En un disco duro externo, la indexación está limitada por el propio disco (unos 500 archivos pequeños por segundo).</p>',
    's.privacy.t': 'Privacidad',
    's.privacy.b': '<p>Prospector funciona completamente sin conexión: sus documentos e índices nunca salen de su ordenador. La única conexión es la comprobación de actualizaciones (GitHub), que puede desactivar.</p>',
    's.uninstall.t': 'Desinstalar',
    's.uninstall.b': '<p><b>Configuración → Aplicaciones → Aplicaciones instaladas → Prospector → Desinstalar</b>. La carpeta de índices se conserva, para que una nueva instalación encuentre sus sitios; bórrela a mano para liberar espacio.</p>',
  },

  ar: {
    'meta.title': 'Prospector — اعثر على أي كلمة في ملفاتك',
    'meta.guideTitle': 'Prospector — دليل الاستخدام',
    'nav.download': 'تنزيل',
    'nav.guide': 'الدليل',
    ribbon: 'دفتر التنقيب',
    tagline: 'اعثر على أي كلمة في ملفاتك. في أجزاء من الثانية، ودون إنترنت.',
    'download.button': 'تنزيل لنظام Windows',
    'download.meta': 'Windows 10 / 11 · 64 بت · مجاني ودون حساب',
    'download.version': 'الإصدار {version} · {size}',
    'download.portable': 'النسخة المحمولة (ZIP، دون تثبيت، لذاكرة USB)',
    'download.all': 'كل الإصدارات ومثبّتات MSI',
    'warning.title': 'عند فتح المثبّت أول مرة',
    'warning.text': 'قد يعرض Windows رسالة «قام Windows بحماية الكمبيوتر»، لأن Prospector غير موقَّع بعدُ بشهادة مدفوعة. انقر <b>مزيد من المعلومات</b> ثم <b>التشغيل على أي حال</b>. يحدث هذا مرة واحدة فقط.',
    'f1.title': 'كل مستنداتك',
    'f1.text': 'PDF، Word (docx، doc، rtf، odt)، Excel، PowerPoint (pptx، ppt، odp)، كتب EPUB، النصوص والشيفرات، رسائل البريد ومرفقاتها (Outlook msg، pst، ost، eml، Thunderbird mbox)، أرشيفات ZIP، RAR، 7z، TAR، GZ. ويُقرأ أيضًا نص الصور وملفات PDF الممسوحة ضوئيًا (OCR).',
    'f2.title': 'فوري ومحدَّث دائمًا',
    'f2.text': 'نتائج في أجزاء من الثانية. المجلدات المراقَبة تتحدّث من تلقاء نفسها.',
    'f3.title': 'أربع لغات',
    'f3.text': 'العربية والإنجليزية والفرنسية والإسبانية، للواجهة وللبحث: الحركات والتشكيل لا تؤثّر في النتائج.',
    'f4.title': 'خصوصية تامة',
    'f4.text': 'يبقى كل شيء على حاسوبك. لا يُرسَل أي شيء إلى أي مكان.',
    'footer.licence': 'Prospector برنامج حر ومجاني (رخصة MIT).',
    'footer.notices': 'تراخيص المكوّنات',
    'guide.title': 'دليل الاستخدام',
    'guide.lead': 'كل ما يستطيع Prospector فعله، في دقائق من القراءة.',
    'guide.toc': 'المحتويات',
    's.install.t': 'تثبيت Prospector',
    's.install.b': '<ol><li>نزّل المثبّت من <a href="index.html">صفحة التنزيل</a> وافتحه.</li><li>إذا عرض Windows رسالة «قام Windows بحماية الكمبيوتر»، فانقر <b>مزيد من المعلومات</b> ثم <b>التشغيل على أي حال</b> (Prospector غير موقَّع بعدُ بشهادة مدفوعة).</li><li>اختر لغة المثبّت ثم اتبع الخطوات. لا حاجة إلى صلاحيات المسؤول: يُثبَّت Prospector لحسابك فقط.</li></ol><p>يستخدم Prospector مكوّن Microsoft Edge WebView2 الموجود أصلًا في Windows 10 و11. إن لم يكن موجودًا يضيفه المثبّت (يلزم الاتصال بالإنترنت مرة واحدة).</p>',
    's.site.t': 'موقع التنقيب الأول',
    's.site.b': '<p><b>موقع التنقيب</b> مجلد يفهرسه Prospector (يقرؤه مرة واحدة ليجيب بعدها فورًا). انقر <b>+</b> في <i>مواقع التنقيب</i> ثم اختر مجلدًا: مستنداتك، أو مجلدًا على الشبكة، أو قرص USB…</p><p>تقرأ الفهرسة الأولى كل الملفات، ويظهر تقدّمها تحت الموقع. بعد ذلك يعرض الموقع <b>مراقبة مباشرة</b>: تُحتسَب الملفات المضافة أو المعدَّلة أو المحذوفة خلال ثوانٍ، وتُستدرَك التغييرات التي جرت أثناء إغلاق Prospector عند تشغيله.</p><p><b>المجموعات</b>: حدّد بعض المواقع، ثم يمنحها زر <b>+ مجموعة</b> أعلى اللوحة اسمًا («العملاء»، «البرمجة»). النقر على مجموعة يحدد مواقعها بالضبط؛ ويفعل <kbd>Ctrl</kbd>+<kbd>1</kbd>…<kbd>9</kbd> الشيء نفسه من لوحة المفاتيح، ويحدد <kbd>Ctrl</kbd>+<kbd>0</kbd> كل المواقع. انقر بزر الفأرة الأيمن على مجموعة لإعادة تسميتها أو منحها المواقع المحددة أو حذفها. ويستخدمها سطر الأوامر أيضًا: <code>--group Clients</code>.</p>',
    's.search.t': 'البحث',
    's.search.b': '<p>اكتب كلماتك ثم اضغط <kbd>Enter</kbd> (أو <b>نقّب!</b>). تعرض كل نتيجة الملف ومجلده والمقاطع التي تظهر فيها الكلمات <mark>مظلَّلة</mark>.</p><table><tr><th>تكتب</th><th>يجد Prospector</th></tr><tr><td><code>عقد تجديد</code></td><td>الملفات التي تحتوي الكلمتين</td></tr><tr><td><code>"مدة الإشعار"</code> أو <code>«…»</code></td><td>العبارة كما هي</td></tr><tr><td><code>عقد OR اتفاقية</code></td><td>إحدى الكلمتين</td></tr><tr><td><code>عقد NOT مسودة</code> أو <code>-مسودة</code></td><td>الملفات الخالية من «مسودة»</td></tr><tr><td><code>/INV-\\d{4}/</code></td><td>تعبيرًا نمطيًا</td></tr><tr><td><code>عقد NEAR فسخ</code>، <code>NEAR:20</code></td><td>الكلمتان على بعد أقل من 100 (أو 20) حرف</td></tr><tr><td><code>LIKE ضروري</code></td><td>هذه الكلمة حتى مع خطأ إملائي، والخيار معطَّل</td></tr><tr><td><code>LINES:1-5 فاتورة</code>، <code>LINES:10+</code></td><td>في هذه الأسطر فقط من كل ملف</td></tr></table><p>تُكتب العوامل بأحرف لاتينية كبيرة: <code>OR</code>، <code>AND</code>، <code>NOT</code>، <code>NEAR</code>، <code>LIKE</code>، <code>LINES</code>. تُهمَل الحركات والتشكيل: «عقد» تجد «عَقْد»، و«resume» تجد «résumé».</p>',
    's.options.t': 'خيارات البحث',
    's.options.b': '<ul><li><b>تحمّل الأخطاء الإملائية</b>: يجد أيضًا الكلمات التي فيها خطأ كتابة. تُسطَّر هذه التطابقات فقط وتُحسَب على حدة.</li><li><kbd>Aa</kbd> <b>مطابقة حالة الأحرف</b> للّغات اللاتينية: «Contrat» لا تجد «contrat».</li><li><kbd>ab</kbd> <b>كلمة كاملة</b> (مفعَّل افتراضيًا): أوقفه لتجد الكلمة داخل كلمات أطول.</li><li><kbd>.*</kbd> <b>تعبير نمطي</b>: يُقرأ النص المدخَل كله تعبيرًا نمطيًا.</li></ul>',
    's.filters.t': 'عوامل التصفية',
    's.filters.b': '<p>تُبقي الأزرار تحت خانة البحث أنواعًا معيّنة من الملفات فقط (PDF، Word، Excel…). يضيف <b>+ تحسين النتائج</b> الحجم وتاريخ التعديل ولغة المستند و<b>المجلدات المستثناة</b> (مسار مجلد، أو نمط مثل <code>**\\node_modules</code>).</p><p>يضيف <b>+ اسم الملف</b> خانة ثانية <i>باسم</i>: <code>*.pdf</code>، <code>فاتورة-2024-??.xlsx</code>، أو عدة أنماط تفصل بينها <code>;</code>، و<code>!مسودة*</code> للاستبعاد، أو <code>/تعبير نمطي/</code>. لا تؤثّر حالة الأحرف ولا الحركات. اترك خانة النص فارغة لسرد الملفات بأسمائها فقط (دون فتحها). ويبحث خيار <b>مجلدات</b> عن المجلدات بدلًا من الملفات. يُطابق المستند داخل الأرشيف باسمه وباسم الأرشيف (<code>*.rar</code> يجد مستندات كل ملفات RAR).</p><p>يقرأ <b>+ قائمة مصطلحات</b> ملفًا نصيًا أو CSV، مصطلحًا في كل سطر (العمود الأول في CSV؛ <code>#</code> للتعليق؛ <code>/…/</code> للتعبير النمطي): الملفات التي تحتوي على <b>واحد على الأقل</b> من هذه المصطلحات، أو <b>كلها</b>، إضافة إلى كلمات البحث. النقر على الشارة يعرض المصطلحات ويغيّر الوضع؛ ويحسب التصدير وتقرير PDF كل مصطلح. من سطر الأوامر: <code>--terms-file list.txt</code>.</p>',
    's.live.t': 'الفحص المباشر',
    's.live.b': '<p>يبحث زر <b>فحص مباشر</b> في المجلدات مباشرةً دون الفهرس: أبطأ، لكنه يقرأ المحتوى الحالي دائمًا ويعمل على مجلدات لم تُفهرَس بعد. تصل النتائج تباعًا، ويوقف زر <b>إيقاف</b> الفحص.</p>',
    's.preview.t': 'المعاينة',
    's.preview.b': '<p>اختر نتيجة لقراءتها في الجهة المقابلة، وكل تطابق مظلَّل. انتقل من تطابق إلى آخر بـ<kbd>↑</kbd> <kbd>↓</kbd> أو <kbd>F3</kbd> / <kbd>Shift+F3</kbd>. تظهر أوراق Excel جداولَ، وشرائح PowerPoint بطاقاتٍ، والشيفرة ملوّنة.</p><p>يفتح زر <b>فتح</b> الملف ببرنامجه المعتاد: وبالنسبة لملف داخل أرشيف أو مرفق برسالة، يُفتح ذلك الملف نفسه (أما رسالة من صندوق بريد فتفتح الصندوق). ويحفظ <b>استخراج إلى…</b> هذا الملف حيث تشاء. ويفتح <b>إظهار في المجلد</b> مستكشف الملفات على الملف، أو على أرشيفه أو صندوق بريده.</p>',
    's.saved.t': 'عمليات البحث المحفوظة',
    's.saved.b': '<p>يحفظ زر <b>★</b> البحث المعروض باسم تختاره: الكلمات والوضع والخيارات وعوامل التصفية والمواقع. تظهر عمليات البحث المحفوظة في اللوحة الجانبية، ونقرة واحدة تعيد تشغيل أيٍّ منها. آخر عمليات بحثك موجودة هناك أيضًا.</p>',
    's.results.t': 'علامات التبويب والسجل والنتائج',
    's.results.b': '<ul><li><b>علامات التبويب</b>: <kbd>Ctrl+T</kbd> يفتح بحثًا جديدًا بجانب البحث الحالي، و<kbd>Ctrl+Tab</kbd> ينتقل بينها، و<kbd>Ctrl+W</kbd> يغلقها. يستمر الفحص المباشر في علامة تبويب تغادرها.</li><li><kbd>Alt+←</kbd> / <kbd>Alt+→</kbd> يعيدان البحث السابق ونتائجه، دون إعادة البحث.</li><li><b>تصفية القائمة</b>: الحقل فوق النتائج لا يُبقي إلا الملفات التي يحتوي اسمها أو مجلدها على ما تكتبه.</li><li><b>البحث داخل هذه النتائج</b>: لا تنظر عمليات البحث التالية إلا في الملفات المعروضة، حتى تزيل الشارة.</li><li><kbd>Ctrl+S</kbd> يحفظ النتائج في ملف <code>.prospector</code>، و<kbd>Ctrl+O</kbd> يعيد فتحه في علامة تبويب مع المقاطع.</li></ul>',
    's.shortcut.t': 'الاختصار العام',
    's.shortcut.b': '<p>من أي تطبيق، يُظهر <kbd>Ctrl+Shift+Space</kbd> برنامج Prospector في المقدّمة جاهزًا للكتابة، واضغطه مرة أخرى لإخفائه. غيّره (أو أوقفه) من <b>الإعدادات ← اختصار عام</b>.</p>',
    's.export.t': 'التصدير وتقرير PDF',
    's.export.b': '<p>يصدّر زر <b>⤓</b> النتائج بصيغة CSV (جدول بيانات) أو JSON، أو يطبع <b>تقرير PDF</b>: البحث وخياراته وعوامل تصفيته، ثم كل ملف مع مقاطعه المظلَّلة. في نافذة الطباعة اختر <b>Microsoft Print to PDF</b>.</p>',
    's.settings.t': 'الإعدادات',
    's.settings.b': '<p>الترس أسفل اللوحة الجانبية: لغة الواجهة، والتخطيط (السجل، الجدول، الطبقات)، والمظهر (أقلام التلوين أو سونار)، والاختصار العام، والتحديثات، و<b>مجلد الفهارس</b>. قد تشغل الفهارس مساحة: انقلها إلى القرص الذي تختاره، وتبقى مواقعك كما هي.</p><p><b>نص الصور (OCR)</b>: يقرأ Prospector نص الصور وملفات PDF الممسوحة ضوئيًا بالتعرّف على النص المدمج في Windows، باللغات المثبّتة فيه. أوقفه لفهرسة المجلدات المليئة بالصور بسرعة أكبر.</p><p><b>الفهرس المشترك</b>: اختر مجلدًا على مشاركة شبكية كمجلد للفهارس على عدة حواسيب. أول Prospector يُفتح يحدّثه؛ والبقية تقرؤه فقط («فهرس مشترك · يحدّثه …» في اللوحة الجانبية) وترى تحديثاته فورًا. إذا بقي ذلك الحاسوب مطفأً، يتولى آخر المهمة خلال دقيقتين. تبقى عمليات البحث المحفوظة والتنبيهات والمجموعات خاصة بكل حاسوب. أضف المواقع بمسارها الشبكي (تُحوَّل محركات الشبكة)، ليتمكن كل حاسوب من فتح الملفات.</p>',
    's.cli.t': 'سطر الأوامر',
    's.cli.b': '<p>يبحث <code>prospector-cli</code> في المواقع نفسها التي يبحث فيها التطبيق، من سكربت أو من نافذة طرفية. لاستخدامه من أي مكان، فعّل <b>سطر الأوامر في PATH</b> في الإعدادات ← التكامل مع Windows (لحسابك فقط)، ثم افتح نافذة طرفية جديدة.</p><pre><code>prospector-cli search "contrat NEAR signé" --site Clients --kind pdf --format csv &gt; resultats.csv\nprospector-cli scan "facture" D:\\Archives --name "*.pdf" --since 2024-01-01\nprospector-cli index --all\nprospector-cli sites</code></pre><p>يستخدم <code>search</code> الفهارس، ويقرأ <code>scan</code> المجلدات مباشرة، ويحدّث <code>index</code> المواقع (أغلق التطبيق أولًا: فهو يقوم بذلك)، ويعرض <code>sites</code> قائمتها. لكل مرشّح في التطبيق خياره (<code>--help</code>). المخرجات: <code>--format text</code> أو <code>json</code> أو <code>jsonl</code> أو <code>csv</code>. رمز الخروج 0 إذا وُجد شيء، و1 إن لم يوجد، و2 لأمر خاطئ، و3 إذا تعذّرت قراءة البيانات. رسائل الأداة بالإنجليزية.</p>',
    's.portable.t': 'النسخة المحمولة',
    's.portable.b': '<p>يعمل ملف ZIP المحمول (صفحة التنزيل) من ذاكرة USB دون تثبيت أي شيء: تبقى الإعدادات والفهارس في <b>ProspectorData</b> بجوار البرنامج، ولا يُكتب شيء على الحاسوب. إذا تغيّر حرف الوحدة على حاسوب آخر، يتبعه Prospector فورًا دون إعادة فهرسة. لا تُعرض قائمة المستكشف ولا التشغيل مع Windows ولا التحديثات التلقائية؛ ويُعلَن عن الإصدار الجديد مع رابط لتنزيله.</p>',
    's.updates.t': 'التحديثات',
    's.updates.b': '<p>عند التشغيل يتحقق Prospector من وجود إصدار جديد، فإن وُجد تعرض بطاقةٌ تثبيته وإعادة التشغيل. يمكنك التحقق يدويًا أو إيقاف التحقق التلقائي من <b>الإعدادات ← التحديثات</b>.</p>',
    's.antivirus.t': 'بطء الفهرسة ومضاد الفيروسات',
    's.antivirus.b': '<p>يفحص مضاد الفيروسات كل ملف يكتبه Prospector، مما قد يبطئ الفهرسة. لتسريعها أضف مجلد الفهارس (المذكور في <b>الإعدادات ← مجلد الفهارس</b>) إلى الاستثناءات: <b>أمان Windows ← الحماية من الفيروسات والمخاطر ← إدارة الإعدادات ← الاستثناءات ← إضافة مجلد</b>. لا تفعل ذلك إلا إذا كنت تثق بالمستندات المفهرَسة.</p><p>على قرص صلب خارجي، يحدّ القرص نفسه من سرعة الفهرسة (نحو 500 ملف صغير في الثانية).</p>',
    's.privacy.t': 'الخصوصية',
    's.privacy.b': '<p>يعمل Prospector دون اتصال تمامًا: لا تغادر مستنداتك ولا فهارسك حاسوبك أبدًا. الاتصال الوحيد هو التحقق من التحديثات (GitHub)، ويمكنك إيقافه.</p>',
    's.uninstall.t': 'إلغاء التثبيت',
    's.uninstall.b': '<p><b>الإعدادات ← التطبيقات ← التطبيقات المثبّتة ← Prospector ← إلغاء التثبيت</b>. يُحتفَظ بمجلد الفهارس كي يجد التثبيت الجديد مواقعك؛ احذفه يدويًا لتحرير المساحة.</p>',
  },
};

const LANGS = ['en', 'fr', 'es', 'ar'];

function pickLang() {
  const asked = new URLSearchParams(location.search).get('lang');
  if (LANGS.includes(asked)) return asked;
  try {
    const saved = localStorage.getItem('prospector.site.lang');
    if (LANGS.includes(saved)) return saved;
  } catch {
    /* no storage: browser language */
  }
  const browser = (navigator.language || 'en').slice(0, 2).toLowerCase();
  return LANGS.includes(browser) ? browser : 'en';
}

function tr(lang, key, params = {}) {
  const text = STRINGS[lang][key] ?? STRINGS.en[key] ?? key;
  // Only known parameters: `{4}` in a regular expression stays as is.
  return text.replace(/\{(\w+)\}/g, (match, name) => (name in params ? params[name] : match));
}

function applyLang(lang) {
  const root = document.documentElement;
  root.lang = lang;
  root.dir = lang === 'ar' ? 'rtl' : 'ltr';
  document.querySelectorAll('[data-i18n]').forEach((el) => (el.textContent = tr(lang, el.dataset.i18n)));
  document.querySelectorAll('[data-i18n-html]').forEach((el) => (el.innerHTML = tr(lang, el.dataset.i18nHtml)));
  const title = document.querySelector('title[data-key]');
  if (title) document.title = tr(lang, title.dataset.key);
  document.querySelectorAll('.langs button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.lang === lang)));
  document.querySelectorAll('a[data-keep-lang]').forEach((a) => {
    const url = new URL(a.getAttribute('href'), location.href);
    url.searchParams.set('lang', lang);
    a.href = url.pathname.split('/').pop() + url.search + url.hash;
  });
  window.dispatchEvent(new CustomEvent('langchange', { detail: lang }));
}

let currentLang = pickLang();

window.i18n = {
  get lang() {
    return currentLang;
  },
  t: (key, params) => tr(currentLang, key, params),
};

document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('.langs button').forEach((button) =>
    button.addEventListener('click', () => {
      currentLang = button.dataset.lang;
      try {
        localStorage.setItem('prospector.site.lang', currentLang);
      } catch {
        /* not remembered */
      }
      applyLang(currentLang);
    }),
  );
  applyLang(currentLang);
});
