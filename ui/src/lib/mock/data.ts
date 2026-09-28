/**
 * Étape 0 mock data — replaced by Tauri commands at Étape 1.
 * The corpus is deliberately multilingual (EN / FR / ES / AR) and mixes
 * formats, archives and mailboxes to exercise the layout, RTL and bidi.
 * Everything is invented and harmless (cooking recipes): it is what the
 * public site shows, so no names, contracts, invoices or bank data.
 * Matches are delimited by ⟦ ⟧ (see `text.ts`).
 */
import type { IndexSite, PreviewDoc, SavedSearch, SearchHit } from '../types';
import { plain } from '../text';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;
const MB = 1024 * 1024;
const ago = (ms: number) => new Date(Date.now() - ms);

export const DEFAULT_QUERY = 'recipe OR recette OR receta OR وصفة';

export const sites: IndexSite[] = [
  {
    id: 'home',
    name: 'Home & Hobbies',
    roots: ['D:\\Documents', 'D:\\Photos'],
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
    roots: ['D:\\Code'],
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
  '"slow cooker"',
  '/v\\d+\\.\\d+\\.\\d+/',
  'soup NOT tomato',
];

export const savedSearches: SavedSearch[] = [
  { id: 's1', label: 'Desserts to try', query: 'dessert AND "to try"' },
  { id: 's2', label: 'Recetas de verano', query: 'receta AND verano' },
  { id: 's3', label: 'TODO / FIXME', query: '/TODO|FIXME/' },
];

const rawHits: Omit<SearchHit, 'created'>[] = [
  {
    id: 'h1',
    siteId: 'home',
    path: 'D:\\Documents\\Cooking\\weeknight-dinners.pdf',
    kind: 'pdf',
    lang: 'en',
    sizeBytes: 2.4 * MB,
    modified: ago(3 * DAY),
    matchCount: 6,
    exactCount: 6,
    score: 0.97,
    snippets: [
      { line: 12, text: '…this ⟦recipe⟧ serves four and takes about thirty minutes from start to finish…' },
      { line: 48, text: '…if you double the ⟦recipe⟧, use a wider tray so the vegetables roast instead of steaming…' },
    ],
  },
  {
    id: 'h2',
    siteId: 'home',
    path: 'D:\\Documents\\Cocina\\recetas_de_verano.docx',
    kind: 'word',
    lang: 'es',
    sizeBytes: 184 * 1024,
    modified: ago(9 * DAY),
    matchCount: 5,
    exactCount: 5,
    score: 0.94,
    snippets: [
      { line: 3, text: '…esta ⟦receta⟧ de gazpacho se sirve muy fría, con un chorrito de aceite de oliva…' },
      { line: 27, text: '…para la ⟦receta⟧ de la tortilla, deja reposar las patatas con la cebolla diez minutos…' },
    ],
  },
  {
    id: 'h3',
    siteId: 'archives',
    path: 'D:\\Archives\\مطبخ\\وصفات_الحلويات.pdf',
    kind: 'pdf',
    lang: 'ar',
    sizeBytes: 1.1 * MB,
    modified: ago(210 * DAY),
    matchCount: 4,
    exactCount: 4,
    score: 0.91,
    snippets: [
      { line: 5, text: '…تكفي هذه ⟦الوصفة⟧ لستة أشخاص ويمكن تحضيرها قبل يوم من التقديم…' },
      { line: 31, text: '…سرّ نجاح ⟦الوصفة⟧ هو تحميص السميد على نار هادئة حتى يصبح ذهبي اللون…' },
    ],
  },
  {
    id: 'h4',
    siteId: 'code',
    path: 'D:\\Code\\meal-planner\\src\\recipes\\RecipeService.ts',
    kind: 'code',
    lang: 'en',
    sizeBytes: 14 * 1024,
    modified: ago(5 * HOUR),
    matchCount: 6,
    exactCount: 6,
    score: 0.88,
    snippets: [
      { line: 14, text: 'export class ⟦Recipe⟧Service {' },
      { line: 57, text: '  scale(⟦recipe⟧: Recipe, servings: number): Recipe {' },
    ],
  },
  {
    id: 'h5',
    siteId: 'mail',
    path: 'E:\\Mail\\Archive 2019-2024.pst › Inbox › Re: the lemon cake',
    kind: 'email',
    lang: 'en',
    sizeBytes: 36 * 1024,
    modified: ago(16 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.86,
    snippets: [{ line: 4, text: 'Here is the ⟦recipe⟧ you asked for — I used less sugar and it still works.' }],
  },
  {
    id: 'h6',
    siteId: 'home',
    path: 'D:\\Documents\\Cuisine\\ratatouille.docx',
    kind: 'word',
    lang: 'fr',
    sizeBytes: 96 * 1024,
    modified: ago(62 * DAY),
    matchCount: 2,
    exactCount: 2,
    score: 0.84,
    snippets: [
      { line: 1, text: '⟦RECETTE⟧ DE LA RATATOUILLE — pour six personnes' },
      { line: 34, text: '…la ⟦recette⟧ est encore meilleure réchauffée le lendemain…' },
    ],
  },
  {
    id: 'h7',
    siteId: 'home',
    path: 'D:\\Documents\\Cocina\\recetas_escaneadas.zip › pan_de_pueblo.pdf',
    kind: 'archive',
    innerKind: 'pdf',
    lang: 'es',
    sizeBytes: 640 * 1024,
    modified: ago(9 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.8,
    snippets: [{ line: 8, text: '…para esta ⟦receta⟧ basta con harina, agua, sal y un poco de levadura…' }],
  },
  {
    id: 'h8',
    siteId: 'archives',
    path: 'D:\\Archives\\مطبخ\\الكسكس.docx',
    kind: 'word',
    lang: 'ar',
    sizeBytes: 58 * 1024,
    modified: ago(190 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.77,
    snippets: [{ line: 2, text: '…تُعدّ هذه ⟦الوصفة⟧ من أشهر الأطباق التقليدية وتُحضَّر بالخضار والحمص…' }],
  },
  {
    id: 'h9',
    siteId: 'home',
    path: 'D:\\Documents\\Cooking\\meal-plan-march.xlsx',
    kind: 'excel',
    lang: 'en',
    sizeBytes: 42 * 1024,
    modified: ago(1 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.71,
    snippets: [{ line: 4, text: 'Tuesday — Lentil soup — ⟦recipe⟧ p. 42 — 25 min' }],
  },
  {
    id: 'h10',
    siteId: 'code',
    path: 'D:\\Code\\meal-planner\\README.md',
    kind: 'text',
    lang: 'en',
    sizeBytes: 8 * 1024,
    modified: ago(20 * DAY),
    matchCount: 1,
    exactCount: 0,
    score: 0.66,
    snippets: [{ line: 22, text: 'All ⟪recipes⟫ are plain Markdown files in `data/`, one per dish.' }],
  },
  {
    id: 'h11',
    siteId: 'home',
    path: 'D:\\Documents\\Atelier cuisine\\atelier-pain.pptx',
    kind: 'powerpoint',
    lang: 'fr',
    sizeBytes: 148 * MB,
    modified: ago(300 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.6,
    snippets: [{ line: 4, text: 'Diapositive 4 — ⟦Recette⟧ du pain de campagne : 500 g de farine, 350 g d’eau' }],
  },
  {
    id: 'h12',
    siteId: 'home',
    path: 'D:\\Photos\\Kitchen\\pancakes-card.jpg',
    kind: 'image',
    lang: 'en',
    sizeBytes: 2 * MB,
    modified: ago(4 * DAY),
    matchCount: 1,
    exactCount: 1,
    score: 0.58,
    snippets: [{ line: 2, text: 'Fluffy pancakes — ⟦Recipe⟧ card · makes 8' }],
  },
];

/** Created a month before their last change (the demo has no real dates). */
/** Detected data of a few demo hits (lot 6.2 screenshots). */
const DEMO_DETECTIONS: Record<string, SearchHit['detections']> = {
  h5: { email: 1 },
  h9: { amount: 2 },
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
      { n: 10, text: 'ROASTED VEGETABLE TRAY BAKE' },
      { n: 11, text: 'Heat the oven to 200 °C and line a large tray with baking paper.' },
      { n: 12, text: 'This ⟦recipe⟧ serves four and takes about thirty minutes from start to finish.' },
      { n: 13, text: 'Any vegetables work: this ⟦recipe⟧ is a good way to finish what is left in the fridge.' },
      { n: 20, text: 'INGREDIENTS' },
      { n: 21, text: 'Two courgettes, one red pepper, one red onion, a handful of cherry tomatoes.' },
      { n: 22, text: 'Olive oil, a pinch of salt, dried thyme and a little smoked paprika.' },
      { n: 47, text: 'TIPS' },
      { n: 48, text: 'If you double the ⟦recipe⟧, use a wider tray so the vegetables roast instead of steaming.' },
      { n: 49, text: 'For a vegan version of the ⟦recipe⟧, swap the feta for toasted chickpeas.' },
      { n: 58, text: 'Serve warm with rice or crusty bread. The ⟦recipe⟧ keeps two days in the fridge.' },
      { n: 60, text: 'See also: lentil soup (p. 42) — pancakes (p. 57) — the basic bread ⟦recipe⟧ (p. 63).' },
    ],
  },
  h2: {
    hitId: 'h2',
    layout: 'prose',
    lines: [
      { n: 1, text: 'RECETAS DE VERANO' },
      { n: 3, text: 'Esta ⟦receta⟧ de gazpacho se sirve muy fría, con un chorrito de aceite de oliva.' },
      { n: 9, text: 'Tritura los tomates, el pepino y el pimiento, y deja enfriar al menos una hora.' },
      { n: 14, text: 'Un poco de pan del día anterior le da una textura más suave.' },
      { n: 27, text: 'Para la ⟦receta⟧ de la tortilla, deja reposar las patatas con la cebolla diez minutos.' },
      { n: 33, text: 'Esta ⟦receta⟧ también sale bien con calabacín en lugar de patata.' },
      { n: 40, text: 'Cada ⟦receta⟧ de este cuaderno es para cuatro personas.' },
    ],
  },
  h3: {
    hitId: 'h3',
    layout: 'prose',
    lines: [
      { n: 1, text: 'البسبوسة بجوز الهند' },
      { n: 5, text: 'تكفي هذه ⟦الوصفة⟧ لستة أشخاص ويمكن تحضيرها قبل يوم من التقديم.' },
      { n: 12, text: 'يُخلط السميد مع جوز الهند والزبدة الذائبة والحليب حتى يتجانس الخليط.' },
      { n: 18, text: 'تُسكب ⟦الوصفة⟧ في صينية مدهونة وتُترك نصف ساعة قبل الخَبز.' },
      { n: 31, text: 'سرّ نجاح ⟦الوصفة⟧ هو تحميص السميد على نار هادئة حتى يصبح ذهبي اللون.' },
      { n: 40, text: 'تُسقى بالقطر البارد فور خروجها من الفرن وتُزيَّن باللوز.' },
    ],
  },
  h9: {
    hitId: 'h9',
    layout: 'sheet',
    lines: [
      { n: 1, text: '— Meal plan · March —' },
      { n: 2, text: 'Day\tDish\tSource\tTime\tServes' },
      { n: 3, text: 'Monday\tVegetable tray bake\tp. 12\t30 min\t4' },
      { n: 4, text: 'Tuesday\tLentil soup\t⟦recipe⟧ p. 42\t25 min\t4' },
      { n: 5, text: 'Wednesday\tGazpacho\trecetas_de_verano\t15 min\t4' },
      { n: 6, text: 'Thursday\tPancakes\tphoto card\t20 min\t3' },
      { n: 7, text: '— Shopping list —' },
      { n: 8, text: 'Item\tQuantity\tAisle' },
      { n: 9, text: 'Red lentils\t500 g\tDry goods' },
    ],
  },
  h11: {
    hitId: 'h11',
    layout: 'slides',
    lines: [
      { n: 1, text: '— 1 —' },
      { n: 2, text: 'Atelier pain maison' },
      { n: 3, text: 'Les bases de la panification' },
      { n: 10, text: '— 4 —' },
      { n: 11, text: '⟦Recette⟧ du pain de campagne : 500 g de farine, 350 g d’eau' },
      { n: 12, text: '10 g de sel, 5 g de levure sèche' },
      { n: 13, text: 'Pétrir 10 minutes, laisser lever 2 heures' },
      { n: 14, text: '— 5 —' },
      { n: 15, text: 'La cuisson' },
      { n: 16, text: 'Four à 240 °C avec un bol d’eau · 35 minutes' },
    ],
  },
  h4: {
    hitId: 'h4',
    layout: 'code',
    lines: [
      { n: 1, text: "import { Injectable } from '@nestjs/common';" },
      { n: 2, text: "import type { ⟦Recipe⟧ } from './recipe.entity';" },
      { n: 3, text: '' },
      { n: 13, text: '@Injectable()' },
      { n: 14, text: 'export class ⟦Recipe⟧Service {' },
      { n: 15, text: '  constructor(private readonly repo: ⟦Recipe⟧Repository) {}' },
      { n: 16, text: '' },
      { n: 56, text: '  /** Scales every ingredient to `servings`, keeping the units. */' },
      { n: 57, text: '  scale(⟦recipe⟧: Recipe, servings: number): Recipe {' },
      { n: 58, text: '    const factor = servings / ⟦recipe⟧.servings;' },
      { n: 59, text: '    return { ...⟦recipe⟧, servings, ingredients: ⟦recipe⟧.ingredients.map((i) => i.times(factor)) };' },
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
