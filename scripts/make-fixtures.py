"""Generates test_fixtures/ (goal.md §8: one sample file per format).

Run: python scripts/make-fixtures.py
PDFs are printed by headless Chrome from HTML (real fonts, Arabic included):
the script prints the Chrome commands and runs them if Chrome is found.
"""
import io
import os
import subprocess
import struct
import zipfile
import zlib

ROOT = os.path.join(os.path.dirname(__file__), '..', 'test_fixtures')
ROOT = os.path.abspath(ROOT)

FR = ("Le présent contrat de prestation prend effet à la date de signature par les deux parties.\n"
      "Toute résiliation anticipée du contrat entraîne le versement d'une indemnité.\n"
      "Les contrats signés sont archivés pendant dix ans.")
ES = ("El presente contrato de prestación de servicios se regirá por la legislación española.\n"
      "La rescisión del contrato deberá notificarse con treinta días de antelación.\n"
      "Los contratos firmados se conservan en el archivo.")
AR = ("يبدأ سريان هذا العَقْدُ اعتبارًا من تاريخ توقيعه من قبل الطرفين.\n"
      "في حال فسخ العقد قبل انتهاء مدته يلتزم المستأجر بدفع إيجار شهرين.\n"
      "تُحفظ العقود الموقعة في الأرشيف.")
EN = ("This service agreement takes effect on the date it is signed by both parties.\n"
      "Any early termination of the agreement requires thirty days of notice.")


def write(rel, data, mode='w', encoding='utf-8'):
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if 'b' in mode:
        with open(path, mode) as f:
            f.write(data)
    else:
        with open(path, mode, encoding=encoding, newline='\n') as f:
            f.write(data)
    return path


def docx_raw(paragraphs):
    body = ''.join(
        '<w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>'.format(
            p.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;'))
        for p in paragraphs)
    document = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
                f'<w:body>{body}</w:body></w:document>')
    content_types = ('<?xml version="1.0" encoding="UTF-8"?>'
                     '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                     '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                     '<Default Extension="xml" ContentType="application/xml"/>'
                     '<Override PartName="/word/document.xml" '
                     'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
                     '</Types>')
    rels = ('<?xml version="1.0" encoding="UTF-8"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
            'Target="word/document.xml"/></Relationships>')
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr('[Content_Types].xml', content_types)
        z.writestr('_rels/.rels', rels)
        z.writestr('word/document.xml', document)
    return buf.getvalue()


def docx(rel, paragraphs):
    write(rel, docx_raw(paragraphs), 'wb')


def zip_bytes(entries):
    """entries: {name: bytes}"""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, data in entries.items():
            z.writestr(name, data)
    return buf.getvalue()


def docx_bytes(paragraphs):
    return docx_raw(paragraphs)


def xlsx(rel, sheets):
    """sheets: [(name, [[cell, ...], ...])] — inline strings, no Excel needed."""
    esc = lambda v: str(v).replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
    files = {
        '[Content_Types].xml': '<?xml version="1.0" encoding="UTF-8"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
        + ''.join(f'<Override PartName="/xl/worksheets/sheet{i + 1}.xml" '
                  'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
                  for i in range(len(sheets))) + '</Types>',
        '_rels/.rels': '<?xml version="1.0" encoding="UTF-8"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>'
        '</Relationships>',
        'xl/workbook.xml': '<?xml version="1.0" encoding="UTF-8"?>'
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>'
        + ''.join(f'<sheet name="{esc(n)}" sheetId="{i + 1}" r:id="rId{i + 1}"/>' for i, (n, _) in enumerate(sheets))
        + '</sheets></workbook>',
        'xl/_rels/workbook.xml.rels': '<?xml version="1.0" encoding="UTF-8"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        + ''.join(f'<Relationship Id="rId{i + 1}" '
                  'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" '
                  f'Target="worksheets/sheet{i + 1}.xml"/>' for i in range(len(sheets)))
        + '</Relationships>',
    }
    for i, (_, rows) in enumerate(sheets):
        body = ''
        for r, row in enumerate(rows, start=1):
            cells = ''
            for c, value in enumerate(row):
                ref = chr(ord('A') + c) + str(r)
                if isinstance(value, (int, float)):
                    cells += f'<c r="{ref}"><v>{value}</v></c>'
                else:
                    cells += f'<c r="{ref}" t="inlineStr"><is><t>{esc(value)}</t></is></c>'
            body += f'<row r="{r}">{cells}</row>'
        files[f'xl/worksheets/sheet{i + 1}.xml'] = ('<?xml version="1.0" encoding="UTF-8"?>'
                                                    '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
                                                    f'<sheetData>{body}</sheetData></worksheet>')
    write(rel, zip_bytes(files), 'wb')


def pptx(rel, slides):
    """slides: [[paragraph, ...], ...] — text of each slide."""
    esc = lambda v: v.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
    files = {'[Content_Types].xml': '<?xml version="1.0" encoding="UTF-8"?>'
             '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
             '<Default Extension="xml" ContentType="application/xml"/></Types>'}
    for i, paragraphs in enumerate(slides, start=1):
        paras = ''.join(f'<a:p><a:r><a:t>{esc(p)}</a:t></a:r></a:p>' for p in paragraphs)
        files[f'ppt/slides/slide{i}.xml'] = ('<?xml version="1.0" encoding="UTF-8"?>'
                                             '<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
                                             'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">'
                                             f'<p:cSld><p:spTree><p:sp><p:txBody>{paras}</p:txBody></p:sp></p:spTree></p:cSld></p:sld>')
    write(rel, zip_bytes(files), 'wb')


def copy_from_cargo(crate, rel_src, rel_dst):
    """Sample files shipped with MIT-licensed crates (Outlook formats)."""
    registry = os.path.join(os.path.expanduser('~'), '.cargo', 'registry', 'src')
    for index in os.listdir(registry):
        src = os.path.join(registry, index, crate, rel_src)
        if os.path.exists(src):
            with open(src, 'rb') as f:
                write(rel_dst, f.read(), 'wb')
            return
    print('not found in cargo registry:', crate, rel_src)


def epub_bytes(title, chapters):
    """A minimal valid EPUB 3: mimetype first and stored, container, package, chapters."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w') as z:
        z.writestr(zipfile.ZipInfo('mimetype'), 'application/epub+zip', compress_type=zipfile.ZIP_STORED)
        z.writestr('META-INF/container.xml',
                   '<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">'
                   '<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>')
        items = ''.join(f'<item id="c{i}" href="chapitre%20{i}.xhtml" media-type="application/xhtml+xml"/>' for i in range(1, len(chapters) + 1))
        spine = ''.join(f'<itemref idref="c{i}"/>' for i in range(1, len(chapters) + 1))
        z.writestr('OEBPS/content.opf',
                   f'<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata>'
                   f'<dc:title xmlns:dc="http://purl.org/dc/elements/1.1/">{title}</dc:title></metadata>'
                   f'<manifest>{items}</manifest><spine>{spine}</spine></package>')
        for i, (heading, text) in enumerate(chapters, 1):
            z.writestr(f'OEBPS/chapitre {i}.xhtml',
                       f'<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>{heading}</title>'
                       f'<style>p {{ margin: 0 }}</style></head><body><h1>{heading}</h1><p>{text}</p></body></html>')
    return buf.getvalue()


def tar_gz_bytes(entries):
    import tarfile
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode='w:gz') as t:
        for name, data in entries.items():
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mtime = 1_710_498_600
            t.addfile(info, io.BytesIO(data))
    return buf.getvalue()


def crc32(data):
    return zlib.crc32(data) & 0xFFFFFFFF


def vint(n):
    """RAR5 variable-length integer: 7 bits per byte, low bits first."""
    out = bytearray()
    while True:
        byte = n & 0x7F
        n >>= 7
        out.append(byte | (0x80 if n else 0))
        if not n:
            return bytes(out)


def rar5_block(header_type, flags, fields, data=b''):
    """One RAR5 block: CRC32, size, type, flags, (data size), fields, data."""
    body = vint(header_type) + vint(flags | (0x02 if data else 0))
    if data:
        body += vint(len(data))
    body += fields
    sized = vint(len(body)) + body
    return struct.pack('<I', crc32(sized)) + sized + data


def rar5_bytes(entries, mtime=1_710_498_600):
    """A real RAR 5 archive in "store" mode (no compression), written from
    the official format description (rarlab.com/technote.htm): creating a RAR
    otherwise needs WinRAR. UnRAR reads it like any other archive."""
    out = bytearray(b'Rar!\x1a\x07\x01\x00')
    out += rar5_block(1, 0, vint(0))  # main archive header, no flags
    for name, data in entries.items():
        encoded = name.encode('utf-8')
        fields = (vint(0x02 | 0x04)          # file flags: Unix mtime + CRC32 present
                  + vint(len(data))          # unpacked size
                  + vint(0x20)               # attributes: archive
                  + struct.pack('<I', mtime)
                  + struct.pack('<I', crc32(data))
                  + vint(0)                  # compression: version 0, store
                  + vint(0)                  # host OS: Windows
                  + vint(len(encoded)) + encoded)
        out += rar5_block(2, 0, fields, data)
    out += rar5_block(5, 0, vint(0))  # end of archive
    return bytes(out)


def pdf_from_html(rel, html, lang='fr', rtl=False):
    # The HTML source goes to target/tmp in the project, never to the system
    # temp folder on C: (BUG-024).
    tmp_dir = os.path.join(ROOT, '..', 'target', 'tmp')
    os.makedirs(tmp_dir, exist_ok=True)
    html_path = os.path.join(tmp_dir, 'prospector-' + os.path.basename(rel).replace('.pdf', '.html'))
    with open(html_path, 'w', encoding='utf-8') as f:
        f.write(f'<!doctype html><html lang="{lang}" dir="{"rtl" if rtl else "ltr"}"><meta charset="utf-8">'
                      f'<body style="font:16px sans-serif">{html}</body></html>')
    pdf_path = os.path.join(ROOT, rel)
    chrome = r'C:\Program Files\Google\Chrome\Application\chrome.exe'
    if os.path.exists(chrome):
        subprocess.run([chrome, '--headless=new', '--disable-gpu', '--no-pdf-header-footer',
                        f'--print-to-pdf={pdf_path}', 'file:///' + html_path.replace('\\', '/')],
                       check=True, capture_output=True)
    else:
        print('Chrome not found: print', html_path, 'to', pdf_path)


def png_from_html(rel, html, width=900, height=420, lang='fr', rtl=False):
    """A screenshot of some HTML: text rendered in pixels, for the OCR tests
    (Chrome shapes Arabic correctly, unlike a bare image library)."""
    tmp_dir = os.path.join(ROOT, '..', 'target', 'tmp')
    os.makedirs(tmp_dir, exist_ok=True)
    html_path = os.path.join(tmp_dir, 'prospector-' + os.path.basename(rel) + '.html')
    with open(html_path, 'w', encoding='utf-8') as f:
        f.write(f'<!doctype html><html lang="{lang}" dir="{"rtl" if rtl else "ltr"}"><meta charset="utf-8">'
                f'<body style="margin:0;padding:40px;background:#fff;color:#111;font:40px/1.4 Arial,sans-serif">{html}</body></html>')
    out = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    chrome = r'C:\Program Files\Google\Chrome\Application\chrome.exe'
    subprocess.run([chrome, '--headless=new', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=1',
                    f'--window-size={width},{height}', f'--screenshot={out}', 'file:///' + html_path.replace('\\', '/')],
                   check=True, capture_output=True)
    return out


if __name__ == '__main__':
    # Files are overwritten, never deleted (project rule).
    # Text & code
    write('fr/notes-contrat.txt', FR)
    # NEAR / LINES (lot 5.5): the same two words, close in one file, far
    # apart (more than 100 characters, 4 lines) in the other.
    write('fr/proximite-proche.txt', 'Charpente du hangar nord.\nLa solive repose sur une entretoise métallique.\nFin du relevé.\n')
    write('fr/proximite-loin.txt', "Une solive a été remplacée côté est.\nLe couvreur a ensuite contrôlé les gouttières, les descentes pluviales\net la totalité des tuiles plates du versant ouest, sans rien signaler\nd'anormal, puis a rangé son échafaudage avant la pluie.\nPlus tard, pose d'une entretoise en chêne.\n")
    write('es/notas.md', '# Notas\n\n' + ES)
    write('ar/ملاحظات.txt', AR)
    write('en/readme.md', '# Agreement\n\n' + EN)
    write('code/ContractService.ts',
          "export class ContractService {\n  async renew(contract: Contract, months: number) {\n"
          "    return this.repo.save({ ...contract, months });\n  }\n}\n")
    write('encodings/windows-1252.txt', 'Résumé du contrat : échéance en décembre.', encoding='cp1252')
    write('encodings/utf16-bom.txt', '\ufeffCanción del contrato firmado en España.', encoding='utf-16-le')
    write('mail/signature.eml',
          'From: julien@example.com\nSubject: Re: signature du contrat\n\n'
          'Bonjour, vous trouverez ci-joint le contrat signé ainsi que l\'annexe tarifaire.\n')

    # Word
    docx('fr/contrat-prestation.docx', FR.split('\n'))
    docx('es/contrato_servicios.docx', ES.split('\n'))
    docx('ar/عقد_إيجار.docx', AR.split('\n'))

    # PDF (printed by Chrome)
    pdf_from_html('fr/contrat-prestation.pdf', ''.join(f'<p>{p}</p>' for p in FR.split('\n')))
    pdf_from_html('en/service-agreement.pdf', ''.join(f'<p>{p}</p>' for p in EN.split('\n')), 'en')

    # Étape 2 — Excel, PowerPoint
    xlsx('fr/factures-2024.xlsx', [
        ('Janvier', [['Référence', 'Client', 'Montant'], ['INV-2024-117', 'Dupont SARL', 4800], ['INV-2024-118', 'García Hermanos', 1250]]),
        ('Février', [['Référence', 'Objet'], ['INV-2024-131', 'Renouvellement du contrat de maintenance']]),
    ])
    pptx('es/presentacion-oferta.pptx', [
        ['Oferta comercial', 'Grupo García'],
        ['Duración del contrato: 36 meses', 'Renovación tácita'],
    ])

    # Étape 2 — ZIP with a nested ZIP
    inner_zip = zip_bytes({'vieux-contrat-1998.txt': 'Ancien contrat de bail signé en 1998.'.encode('utf-8')})
    write('archives/dossier-clients.zip', zip_bytes({
        'notes/devis.txt': 'Devis pour le contrat de maintenance annuelle.'.encode('utf-8'),
        'annexes/annexe-tarifaire.docx': docx_bytes(['Annexe tarifaire du contrat', 'Tarif horaire : 85 €']),
        'archives/ancien.zip': inner_zip,
    }), 'wb')
    # Zip bomb guard: 20 MB of zeros compressed to a few KB must be skipped.
    write('broken/bomb.zip', zip_bytes({'zeros.txt': b'0' * (20 * 1024 * 1024)}), 'wb')

    # Étape 2 — Outlook samples (MIT-licensed crates msg_parser / outlook-pst)
    copy_from_cargo('msg_parser-0.3.6', os.path.join('data', 'test_email.msg'), 'mail/test_email.msg')
    copy_from_cargo('msg_parser-0.3.6', os.path.join('data', 'unicode.msg'), 'mail/unicode.msg')
    copy_from_cargo('outlook-pst-1.2.0', os.path.join('examples', 'Empty.pst'), 'mail/Empty.pst')

    # Étape 3 — RAR and 7z
    write('archives/devis-chantier.rar', rar5_bytes({
        'devis/terrassement.txt': 'Devis n° 2024-18 : terrassement et évacuation des déblais, 3 200 € HT.'.encode('utf-8'),
        'notes/lisez-moi.md': '# À faire\n\nLa bétonnière doit être rendue avant vendredi.'.encode('utf-8'),
    }), 'wb')
    # Solid LZMA2 7z, written by the dev-only Rust example (sevenz-rust2).
    seven = os.path.join(ROOT, 'archives', 'courrier-chantier.7z')
    subprocess.run(['cargo', 'run', '-q', '-p', 'prospector-core', '--example', 'make_7z', '--', seven],
                   cwd=os.path.join(ROOT, '..'), check=True)
    # Password-protected RAR (sample of the MIT-licensed `unrar` crate): counted as "encrypted".
    copy_from_cargo('unrar-0.5.8', os.path.join('data', 'crypted.rar'), 'archives/verrouille.rar')

    # Étape 5.2 — OCR: text only present as pixels
    png_from_html('images/facture-scannee.png',
                  '<p>FACTURE N° OCR-4271</p><p>Nettoyage de la vitrerie du hall</p>')
    receipt = png_from_html('images/recu.png', '<p>RECEIPT 2024-88</p><p>Horticulture services, total 120 dollars</p>')
    from PIL import Image  # JPEG version: the DCT path of the OCR
    Image.open(receipt).convert('RGB').save(os.path.join(ROOT, 'images', 'recu.jpg'), quality=92)
    os.replace(receipt, os.path.join(ROOT, '..', 'target', 'tmp', 'recu.png'))
    png_from_html('images/اعلان.png', '<p>إعلان صيانة المصعد</p><p>يوم الاثنين من التاسعة صباحا</p>', lang='ar', rtl=True)
    # Icons are never read (smaller than 200 pixels).
    Image.new('RGB', (64, 64), 'white').save(os.path.join(ROOT, 'images', 'icone.png'))
    # A scanned letter: a PDF made of one image, without any text layer.
    scan = png_from_html('scans/lettre.png', '<p>Relance pour la toiture</p><p>Merci de confirmer la date</p>', 1200, 600)
    pdf_from_html('scans/courrier-scanne.pdf',
                  f'<img src="file:///{scan.replace(chr(92), "/")}" style="width:100%">')
    os.replace(scan, os.path.join(ROOT, '..', 'target', 'tmp', 'lettre.png'))

    # Étape 5.3 — older and other formats
    # Word 97 and PowerPoint 97 files: real ones from the Apache POI test corpus
    # (Apache License 2.0). Their text is asserted by POI's own tests. Writing
    # them with Office by automation hung (BUG-030).
    poi = 'https://raw.githubusercontent.com/apache/poi/trunk/test-data/'
    for rel in ['document/test2.doc', 'document/HeaderFooterUnicode.doc', 'document/rasp.doc',
                'slideshow/basic_test_ppt_file.ppt', 'slideshow/with_textbox.ppt']:
        target = os.path.join(ROOT, 'legacy', os.path.basename(rel))
        if not os.path.exists(target):
            import urllib.request
            with urllib.request.urlopen(poi + rel) as r:
                write('legacy/' + os.path.basename(rel), r.read(), 'wb')
    write('legacy/README.md',
          '# Legacy Office samples\n\n'
          'The `.doc` and `.ppt` files come from the Apache POI test corpus '
          '(https://github.com/apache/poi/tree/trunk/test-data, Apache License 2.0).\n'
          'The `.rtf`, `.odt` and `.odp` files are written by scripts/make-fixtures.py.\n')
    # RTF as Word writes it: font and color tables, code page, \\'hh accents, \\uN Arabic.
    rtf_arabic = ''.join(f'\\u{ord(c)}?' if c != ' ' else ' ' for c in 'محضر الورشة')
    write('legacy/proces-verbal.rtf',
          '{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fswiss Arial;}{\\f1\\fnil Arial;}}'
          '{\\colortbl ;\\red255\\green0\\blue0;}{\\*\\generator Riched20 10.0;}\\viewkind4\\uc1\n'
          '\\pard\\f0\\fs22 Proc\\\'e8s-verbal : le g\\\'e9om\\\'e8tre confirme le {\\b bornage} de la parcelle.\\par\n'
          '\\cf1 Prochaine visite\\tab jeudi\\cf0\\par\n'
          '\\f1 ' + rtf_arabic + '\\par\n}', 'w')
    # OpenDocument text and presentation, as LibreOffice stores them.
    odf_manifest = lambda mime: ('<?xml version="1.0" encoding="UTF-8"?><manifest:manifest '
                                 'xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">'
                                 f'<manifest:file-entry manifest:full-path="/" manifest:media-type="{mime}"/>'
                                 '<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>'
                                 '</manifest:manifest>')
    ns = ('xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
          'xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" '
          'xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" '
          'xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"')
    write('legacy/compte-rendu.odt', zip_bytes({
        'mimetype': b'application/vnd.oasis.opendocument.text',
        'META-INF/manifest.xml': odf_manifest('application/vnd.oasis.opendocument.text').encode('utf-8'),
        'content.xml': (f'<?xml version="1.0" encoding="UTF-8"?><office:document-content {ns}><office:body><office:text>'
                        '<text:h>Compte rendu</text:h><text:p>Le couvreur remplace les <text:span>ardoises</text:span>'
                        '<text:s text:c="2"/>du pignon.</text:p></office:text></office:body></office:document-content>').encode('utf-8'),
    }), 'wb')
    write('legacy/avancement.odp', zip_bytes({
        'mimetype': b'application/vnd.oasis.opendocument.presentation',
        'META-INF/manifest.xml': odf_manifest('application/vnd.oasis.opendocument.presentation').encode('utf-8'),
        'content.xml': (f'<?xml version="1.0" encoding="UTF-8"?><office:document-content {ns}><office:body><office:presentation>'
                        '<draw:page><draw:frame><draw:text-box><text:p>Avancement</text:p></draw:text-box></draw:frame></draw:page>'
                        '<draw:page><draw:frame><draw:text-box><text:p>Pose de la zinguerie terminée</text:p></draw:text-box></draw:frame>'
                        '<presentation:notes><draw:frame><draw:text-box><text:p>notes privées</text:p></draw:text-box></draw:frame>'
                        '</presentation:notes></draw:page></office:presentation></office:body></office:document-content>').encode('utf-8'),
    }), 'wb')
    write('books/le-phare.epub', epub_bytes('Le phare', [
        ('Le gardien', 'Le gardien du phare allume le sémaphore chaque soir.'),
        ('La tempête', 'Les vagues frappent la digue jusqu’au matin.'),
    ]), 'wb')
    write('archives/sauvegarde.tar.gz', tar_gz_bytes({
        'rapports/bilan.txt': 'Bilan annuel : remplacement des lampadaires de la cour.'.encode('utf-8'),
        'rapports/equipe.md': '# Équipe\n\nLe chef de quai coordonne le déchargement.'.encode('utf-8'),
    }), 'wb')
    import gzip
    import bz2
    write('logs/serveur.log.gz', gzip.compress('2024-03-15 10:30 erreur de connexion au pare-feu du siège'.encode('utf-8')), 'wb')
    write('logs/notes.txt.bz2', bz2.compress('Visite de l’orangeraie prévue au printemps.'.encode('utf-8')), 'wb')
    write('archives/outils.jar', zip_bytes({'META-INF/README.txt': 'Classeur javanais des outils internes.'.encode('utf-8')}), 'wb')
    # A PDF "protected" against copying only (owner password): opens without a password.
    pdf_from_html('fr/protege.pdf', '<p>Clause de non-concurrence quinquennale.</p>')
    from pypdf import PdfReader, PdfWriter
    protected = os.path.join(ROOT, 'fr', 'protege.pdf')
    writer = PdfWriter(clone_from=PdfReader(protected))
    writer.encrypt(user_password='', owner_password='prospector-owner', algorithm='RC4-128')
    with open(protected, 'wb') as f:
        writer.write(f)

    # Étape 5.4 — e-mail attachments, mbox
    copy_from_cargo('msg_parser-0.3.6', os.path.join('data', 'attachment.msg'), 'mail/avec-piece-jointe.msg')
    from email.message import EmailMessage
    from email.utils import format_datetime
    import datetime
    when = datetime.datetime(2024, 3, 15, 10, 30, tzinfo=datetime.timezone.utc)
    devis = EmailMessage()
    devis['From'] = 'Atelier Martin <atelier@example.com>'
    devis['To'] = 'client@example.com'
    devis['Subject'] = 'Devis charpente'
    devis['Date'] = format_datetime(when)
    devis.set_content('Bonjour, vous trouverez le devis en pièce jointe.')
    devis.add_attachment(docx_bytes(['Devis de charpente', 'Chevronnage en douglas']), maintype='application',
                         subtype='vnd.openxmlformats-officedocument.wordprocessingml.document', filename='devis-charpente.docx')
    devis.add_attachment(zip_bytes({'plans/notice.txt': 'Pose des voliges avant la couverture.'.encode('utf-8')}),
                         maintype='application', subtype='zip', filename='plans.zip')
    write('mail/devis-charpente.eml', devis.as_bytes(), 'wb')

    def mbox_message(subject, body, attachment=None):
        m = EmailMessage()
        m['From'] = 'Fournisseur <ventes@example.com>'
        m['To'] = 'chantier@example.com'
        m['Subject'] = subject
        m['Date'] = format_datetime(when)
        m.set_content(body)
        if attachment:
            name, data = attachment
            m.add_attachment(data, maintype='text', subtype='plain', filename=name)
        return b'From ventes@example.com Fri Mar 15 10:30:00 2024\n' + m.as_bytes().replace(b'\r\n', b'\n') + b'\n'
    write('mail/fournisseur.mbox',
          mbox_message('Commande de tuiles', 'La livraison des tuiles est prévue lundi.')
          + mbox_message('Commande de tuiles', 'Complément : les tuiles de rive suivront.', ('bon.txt', 'Bon de livraison du faîtage'.encode('utf-8'))),
          'wb')

    # Must be skipped or excluded
    write('code/node_modules/lib/contract.js', 'const contract = "must never be indexed (node_modules)";')
    write('broken/not-really-text.txt', b'\x7fELF\x00\x01\x02 contract', 'wb')
    write('broken/damaged.pdf', b'%PDF-1.7 this is not a real pdf contrat', 'wb')
    write('broken/damaged.docx', b'PK not a zip contrat', 'wb')

    print('fixtures written to', ROOT)
