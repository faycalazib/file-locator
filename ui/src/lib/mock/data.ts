/**
 * Étape 0 mock data — replaced by Tauri commands at Étape 1.
 * The corpus is deliberately multilingual (FR / ES / AR / EN) and mixes
 * formats, archives and mailboxes to exercise the layout, RTL and bidi.
 * Matches are delimited by ⟦ ⟧ (see `text.ts`).
 */
import type { IndexSite, PreviewDoc, SavedSearch, SearchHit } from '../types';
import { plain } from '../text';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;
const MB = 1024 * 1024;
const ago = (ms: number) => new Date(Date.now() - ms);

export const DEFAULT_QUERY = 'contrat OR contrato OR عقد';

export const sites: IndexSite[] = [
  {
    id: 'clients',
    name: 'Clients & RH',
    roots: ['D:\\Clients', 'D:\\RH'],
    docCount: 48_213,
    sizeBytes: 31.2 * 1024 * MB,
    status: 'watching',
    lastIndexed: ago(2 * MIN),
  },
  {
    id: 'mail',
    name: 'Mail 2019–2024',
    roots: ['E:\\Mail\\Archive 2019-2024.pst'],
    docCount: 126_480,
    sizeBytes: 18.7 * 1024 * MB,
    status: 'indexing',
    phase: 'reading',
    progress: 0.62,
    lastIndexed: ago(3 * DAY),
  },
  {
    id: 'code',
    name: 'Development',
    roots: ['H:\\Development'],
    docCount: 93_117,
    sizeBytes: 6.4 * 1024 * MB,
    status: 'ready',
    lastIndexed: ago(3 * DAY + 4 * HOUR),
  },
  {
    id: 'archives',
    name: 'Archives',
    roots: ['D:\\Archives', 'F:\\Scans'],
    docCount: 8_904,
    sizeBytes: 12.9 * 1024 * MB,
    status: 'error',
    lastIndexed: ago(41 * DAY),
    error: { code: 'rootUnavailable', path: 'F:\\Scans' },
  },
];

export const recentSearches: string[] = [
  DEFAULT_QUERY,
  '"clause de confidentialité"',
  '/INV-\\d{4}-\\d{3}/',
  'résiliation NOT avenant',
];

export const savedSearches: SavedSearch[] = [
  { id: 's1', label: 'Factures en attente', query: 'facture AND "en attente"' },
  { id: 's2', label: 'Contratos a renovar', query: 'contrato AND renovación' },
  { id: 's3', label: 'TODO / FIXME', query: '/TODO|FIXME/' },
];

const rawHits: Omit<SearchHit, 'created'>[] = [
  {
    id: 'h1',
    siteId: 'clients',
    path: 'D:\\Clients\\Dupont SARL\\2024\\contrat-prestation-v3.pdf',
    kind: 'pdf',
    lang: 'fr',
    sizeBytes: 2.4 * MB,
    modified: ago(3 * DAY),
    matchCount: 7,
    exactCount: 7,
    score: 0.97,
    snippets: [
      { line: 12, text: '…le présent ⟦contrat⟧ de prestation prend effet à la date de signature par les deux parties…' },
      { line: 48, text: '…toute résiliation anticipée du ⟦contrat⟧ entraîne le versement d’une indemnité égale à trois mois…' },
    ],
  },
  {
    id: 'h2',
    siteId: 'clients',
    path: 'D:\\Clients\\García Hermanos\\contrato_servicios_2024.docx',
    kind: 'word',
    lang: 'es',
    sizeBytes: 184 * 1024,
    modified: ago(9 * DAY),
    matchCount: 5,
    exactCount: 5,
    score: 0.94,
    snippets: [
      { line: 3, text: '…el presente ⟦contrato⟧ de prestación de servicios se regirá por la legislación española…' },
      { line: 27, text: '…la rescisión del ⟦contrato⟧ deberá notificarse con treinta días de antelación…' },
    ],
  },
  {
    id: 'h3',
    siteId: 'archives',
    path: 'D:\\Archives\\عقود\\عقد_إيجار_2023.pdf',
    kind: 'pdf',
    lang: 'ar',
    sizeBytes: 1.1 * MB,
    modified: ago(210 * DAY),
    matchCount: 4,
    exactCount: 4,
    score: 0.91,
    snippets: [
      { line: 5, text: '…يبدأ سريان هذا ⟦العقد⟧ اعتبارًا من تاريخ توقيعه من قبل الطرفين…' },
      { line: 31, text: '…في حال فسخ ⟦العقد⟧ قبل انتهاء مدته يلتزم المستأجر بدفع إيجار شهرين…' },
    ],
  },
  {
    id: 'h4',
    siteId: 'code',
    path: 'H:\\Development\\billing-api\\src\\contracts\\ContractService.ts',
    kind: 'code',
    lang: 'en',
    sizeBytes: 14 * 1024,
    modified: ago(5 * HOUR),
    matchCount: 6,
    exactCount: 0,
    score: 0.88,
    snippets: [
      { line: 14, text: 'export class ⟪Contract⟫Service {' },
      { line: 57, text: '  async renew(⟪contract⟫: Contract, months: number): Promise<Contract> {' },
    ],
  },
  {
    id: 'h5',
    siteId: 'mail',
    path: 'E:\\Mail\\Archive 2019-2024.pst › Inbox › Re: signature du contrat',
    kind: 'email',
    lang: 'fr',
    sizeBytes: 36 * 1024,
    modified: ago(16 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.86,
    snippets: [{ line: 4, text: 'Bonjour Julien, vous trouverez ci-joint le ⟦contrat⟧ signé ainsi que l’annexe tarifaire.' }],
  },
  {
    id: 'h6',
    siteId: 'clients',
    path: 'D:\\RH\\modèles\\contrat-travail-CDI.docx',
    kind: 'word',
    lang: 'fr',
    sizeBytes: 96 * 1024,
    modified: ago(62 * DAY),
    matchCount: 2,
    exactCount: 2,
    score: 0.84,
    snippets: [
      { line: 1, text: '⟦CONTRAT⟧ DE TRAVAIL À DURÉE INDÉTERMINÉE — Entre les soussignés…' },
      { line: 34, text: '…le présent ⟦contrat⟧ est soumis à la convention collective SYNTEC…' },
    ],
  },
  {
    id: 'h7',
    siteId: 'clients',
    path: 'D:\\Clients\\García Hermanos\\anexos.zip › anexo_II_condiciones.pdf',
    kind: 'archive',
    innerKind: 'pdf',
    lang: 'es',
    sizeBytes: 640 * 1024,
    modified: ago(9 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.8,
    snippets: [{ line: 8, text: '…las condiciones generales del ⟦contrato⟧ marco firmado el 12 de marzo…' }],
  },
  {
    id: 'h8',
    siteId: 'archives',
    path: 'D:\\Archives\\عقود\\ملحق_العقد_رقم_3.docx',
    kind: 'word',
    lang: 'ar',
    sizeBytes: 58 * 1024,
    modified: ago(190 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.77,
    snippets: [{ line: 2, text: '…يُعدّ هذا الملحق جزءًا لا يتجزأ من ⟦العقد⟧ الأصلي المبرم بين الطرفين…' }],
  },
  {
    id: 'h9',
    siteId: 'clients',
    path: 'D:\\Clients\\Dupont SARL\\factures\\facture-2024-117.xlsx',
    kind: 'excel',
    lang: 'fr',
    sizeBytes: 42 * 1024,
    modified: ago(1 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.71,
    snippets: [{ line: 6, text: 'Réf. ⟦contrat⟧ : CP-2024-031 — Maintenance annuelle — 4 800,00 €' }],
  },
  {
    id: 'h10',
    siteId: 'code',
    path: 'H:\\Development\\billing-api\\docs\\README.md',
    kind: 'text',
    lang: 'en',
    sizeBytes: 8 * 1024,
    modified: ago(20 * DAY),
    matchCount: 1,
    exactCount: 0,
    score: 0.66,
    snippets: [{ line: 22, text: 'Each ⟪contract⟫ has a renewal policy: `auto`, `manual` or `none`.' }],
  },
  {
    id: 'h11',
    siteId: 'clients',
    path: 'D:\\Clients\\Leclerc\\2023\\presentation-offre.pptx',
    kind: 'powerpoint',
    lang: 'fr',
    sizeBytes: 148 * MB,
    modified: ago(300 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.6,
    snippets: [{ line: 4, text: 'Diapositive 4 — Durée du ⟦contrat⟧ : 36 mois, reconduction tacite' }],
  },
  {
    id: 'h12',
    siteId: 'clients',
    path: 'D:\\Clients\\Dupont SARL\\2024\\scans\\facture-2024-117.jpg',
    kind: 'image',
    lang: 'fr',
    sizeBytes: 2 * MB,
    modified: ago(4 * DAY),
    matchCount: 2,
    exactCount: 2,
    score: 0.58,
    snippets: [{ line: 3, text: 'Réf. ⟦contrat⟧ CP-2024-031 — Maintenance annuelle — 4 800,00 €' }],
  },
];

/** Created a month before their last change (the demo has no real dates). */
/** Detected data of a few demo hits (lot 6.2 screenshots). */
const DEMO_DETECTIONS: Record<string, SearchHit['detections']> = {
  h1: { iban: 1, email: 2, phone: 1 },
  h2: { amount: 4, vat: 1 },
  h5: { email: 3, phone: 2 },
  h6: { card: 1, nir: 1 },
};

export const hits: SearchHit[] = rawHits.map((h) => ({
  ...h,
  created: new Date(h.modified.getTime() - 30 * DAY),
  detections: DEMO_DETECTIONS[h.id],
}));

const previews: Record<string, PreviewDoc> = {
  h1: {
    hitId: 'h1',
    layout: 'prose',
    lines: [
      { n: 10, text: 'ARTICLE 2 — OBJET ET DURÉE' },
      { n: 11, text: 'Le Prestataire s’engage à réaliser pour le Client les prestations décrites en annexe 1.' },
      { n: 12, text: 'Le présent ⟦contrat⟧ de prestation prend effet à la date de signature par les deux parties et est conclu pour une durée de vingt-quatre (24) mois.' },
      { n: 13, text: 'À l’issue de cette période, le ⟦contrat⟧ pourra être renouvelé par avenant écrit.' },
      { n: 20, text: 'ARTICLE 5 — PRIX ET MODALITÉS DE PAIEMENT' },
      { n: 21, text: 'Les prix figurant au ⟦contrat⟧ sont exprimés hors taxes et révisables annuellement.' },
      { n: 22, text: 'Les factures sont payables à trente (30) jours à compter de leur date d’émission.' },
      { n: 47, text: 'ARTICLE 11 — RÉSILIATION' },
      { n: 48, text: 'Toute résiliation anticipée du ⟦contrat⟧ entraîne le versement d’une indemnité égale à trois mois de prestations.' },
      { n: 49, text: 'En cas de manquement grave, le ⟦contrat⟧ peut être résilié de plein droit après mise en demeure restée sans effet.' },
      { n: 58, text: 'Fait à Lyon, en deux exemplaires originaux. Chaque partie reconnaît avoir reçu un exemplaire du ⟦contrat⟧.' },
      { n: 60, text: 'Annexe 1 : description des prestations — Annexe 2 : grille tarifaire — Annexe 3 : ⟦contrat⟧ de sous-traitance.' },
    ],
  },
  h2: {
    hitId: 'h2',
    layout: 'prose',
    lines: [
      { n: 1, text: 'CONTRATO DE PRESTACIÓN DE SERVICIOS' },
      { n: 3, text: 'El presente ⟦contrato⟧ de prestación de servicios se regirá por la legislación española vigente.' },
      { n: 9, text: 'La duración del ⟦contrato⟧ será de doce meses, prorrogable por acuerdo expreso de ambas partes.' },
      { n: 14, text: 'El precio acordado incluye todos los desplazamientos y gastos de gestión.' },
      { n: 27, text: 'La rescisión del ⟦contrato⟧ deberá notificarse con treinta días de antelación y por escrito.' },
      { n: 33, text: 'Cualquier modificación del ⟦contrato⟧ requerirá la firma de un anexo.' },
      { n: 40, text: 'Y en prueba de conformidad, firman el presente ⟦contrato⟧ en Sevilla, a 4 de febrero de 2024.' },
    ],
  },
  h3: {
    hitId: 'h3',
    layout: 'prose',
    lines: [
      { n: 1, text: 'عقد إيجار محل تجاري' },
      { n: 5, text: 'يبدأ سريان هذا ⟦العقد⟧ اعتبارًا من تاريخ توقيعه من قبل الطرفين ولمدة ثلاث سنوات.' },
      { n: 12, text: 'يلتزم المستأجر بدفع قيمة الإيجار في بداية كل شهر ميلادي.' },
      { n: 18, text: 'لا يجوز للمستأجر التنازل عن ⟦العقد⟧ أو تأجير المحل من الباطن دون موافقة خطية.' },
      { n: 31, text: 'في حال فسخ ⟦العقد⟧ قبل انتهاء مدته يلتزم المستأجر بدفع إيجار شهرين كتعويض.' },
      { n: 40, text: 'حُرّر هذا ⟦العقد⟧ من نسختين أصليتين بيد كل طرف نسخة للعمل بموجبها.' },
    ],
  },
  h9: {
    hitId: 'h9',
    layout: 'sheet',
    lines: [
      { n: 1, text: '— Factures 2024 —' },
      { n: 2, text: 'Référence\tClient\tObjet\tMontant HT\tÉchéance' },
      { n: 3, text: 'INV-2024-115\tDupont SARL\tAudit sécurité\t2 400,00 €\t15/03/2024' },
      { n: 4, text: 'INV-2024-116\tGarcía Hermanos\t\t1 250,00 €\t30/03/2024' },
      { n: 5, text: 'INV-2024-117\tDupont SARL\tRéf. ⟦contrat⟧ CP-2024-031 — Maintenance annuelle\t4 800,00 €\t30/04/2024' },
      { n: 6, text: 'INV-2024-118\tLeclerc\tFormation\t950,00 €\t15/05/2024' },
      { n: 7, text: '— Relances —' },
      { n: 8, text: 'Client\tÉtape\tDate' },
      { n: 9, text: 'García Hermanos\t1ʳᵉ relance\t12/04/2024' },
    ],
  },
  h11: {
    hitId: 'h11',
    layout: 'slides',
    lines: [
      { n: 1, text: '— 1 —' },
      { n: 2, text: 'Offre de maintenance 2023–2026' },
      { n: 3, text: 'Leclerc · proposition commerciale' },
      { n: 10, text: '— 4 —' },
      { n: 11, text: 'Durée du ⟦contrat⟧ : 36 mois, reconduction tacite' },
      { n: 12, text: 'Révision annuelle des prix selon l’indice Syntec' },
      { n: 13, text: 'Résiliation avec un préavis de 3 mois' },
      { n: 14, text: '— 5 —' },
      { n: 15, text: 'Engagements de service' },
      { n: 16, text: 'Intervention sous 4 h ouvrées · disponibilité 99,5 %' },
    ],
  },
  h4: {
    hitId: 'h4',
    layout: 'code',
    lines: [
      { n: 1, text: "import { Injectable } from '@nestjs/common';" },
      { n: 2, text: "import type { ⟪Contract⟫ } from './contract.entity';" },
      { n: 3, text: '' },
      { n: 13, text: '@Injectable()' },
      { n: 14, text: 'export class ⟪Contract⟫Service {' },
      { n: 15, text: '  constructor(private readonly repo: ⟪Contract⟫Repository) {}' },
      { n: 16, text: '' },
      { n: 56, text: '  /** Extends a contract by `months`, keeping its renewal policy. */' },
      { n: 57, text: '  async renew(⟪contract⟫: Contract, months: number): Promise<Contract> {' },
      { n: 58, text: '    const end = addMonths(⟪contract⟫.endsAt, months);' },
      { n: 59, text: '    return this.repo.save({ ...⟪contract⟫, endsAt: end });' },
      { n: 60, text: '  }' },
      { n: 61, text: '}' },
    ],
  },
};

/** Explicit preview when available, otherwise one built from the snippets. */
export function previewFor(hit: SearchHit): PreviewDoc {
  const explicit = previews[hit.id];
  if (explicit) return explicit;
  return {
    hitId: hit.id,
    layout: hit.kind === 'code' ? 'code' : 'prose',
    lines: hit.snippets.map((s) => ({ n: s.line, text: s.text.replace(/^…|…$/g, '') })),
  };
}

/** Text searched by the mock engine (preview + snippets + file name). */
export function searchableText(hit: SearchHit): string {
  const doc = previewFor(hit);
  return [hit.path, ...hit.snippets.map((s) => plain(s.text)), ...doc.lines.map((l) => plain(l.text))].join('\n');
}
