/**
 * Image thumbnails (lot 6.6): made by Rust (Windows imaging), kept in memory
 * for the session, at most 3 made at a time.
 */
import { api, inTauri } from './api';
import type { SearchHit } from './types';

const MAX_PARALLEL = 3;
const cache = new Map<string, Promise<string | null>>();
const queue: (() => void)[] = [];
let running = 0;

function next() {
  if (running >= MAX_PARALLEL) return;
  const job = queue.shift();
  if (job) job();
}

/** A drawn receipt, for the browser demo (screenshots). */
export const DEMO_IMAGE =
  'data:image/svg+xml;utf8,' +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="800" height="1000" viewBox="0 0 800 1000">
      <rect width="800" height="1000" fill="#f7f3ea"/>
      <text x="60" y="120" font-family="Georgia" font-size="56" fill="#222">FACTURE n° 2024-117</text>
      <text x="60" y="200" font-family="Georgia" font-size="30" fill="#333">Dupont SARL — 12 rue des Lilas, Lyon</text>
      <text x="60" y="300" font-family="Georgia" font-size="32" fill="#222">Réf. contrat CP-2024-031</text>
      <text x="60" y="360" font-family="Georgia" font-size="32" fill="#222">Maintenance annuelle — 4 800,00 €</text>
      <line x1="60" y1="420" x2="740" y2="420" stroke="#999" stroke-width="2"/>
      <text x="60" y="480" font-family="Georgia" font-size="28" fill="#444">Selon les termes du contrat de prestation,</text>
      <text x="60" y="525" font-family="Georgia" font-size="28" fill="#444">paiement à 30 jours fin de mois.</text>
      <text x="520" y="880" font-family="Georgia" font-size="30" fill="#222">Total : 4 800,00 €</text>
    </svg>`,
  );

/** Boxes of "contrat" on the demo receipt (fractions). */
export const DEMO_BOXES = [
  { x: 0.15, y: 0.272, w: 0.127, h: 0.036, fuzzy: false },
  { x: 0.396, y: 0.453, w: 0.104, h: 0.034, fuzzy: false },
];

/** An image on disk, or inside an archive or an e-mail (lot 6.7). */
export function isImageHit(hit: SearchHit): boolean {
  return hit.kind === 'image' || hit.innerKind === 'image';
}

/**
 * Whether the list shows the picture of this result: not for an image inside
 * a mailbox (each picture would read the whole mailbox again); its preview
 * still shows it.
 */
export function thumbInList(hit: SearchHit): boolean {
  return isImageHit(hit) && !/\.(pst|ost|mbox) › /i.test(hit.path);
}

/** The thumbnail of an image file (`null` if it cannot be read). */
export function thumbnail(path: string, size: number): Promise<string | null> {
  const key = `${size}\u0000${path}`;
  const known = cache.get(key);
  if (known) return known;
  const made = !inTauri
    ? Promise.resolve(DEMO_IMAGE)
    : new Promise<string | null>((resolve) => {
        queue.push(() => {
          running++;
          api
            .thumbnail(path, size)
            .then(resolve, () => resolve(null))
            .finally(() => {
              running--;
              next();
            });
        });
        next();
      });
  cache.set(key, made);
  return made;
}

/**
 * Svelte action: the image gets its thumbnail (the doodle stays if it
 * cannot be read). Asked at once: the list holds at most a few hundred
 * results, and at most 3 thumbnails are made at a time.
 */
export function lazyThumb(node: HTMLImageElement, path: string) {
  let current = path;
  const load = () => {
    const asked = current;
    void thumbnail(asked, 96).then((url) => {
      if (url && asked === current) {
        node.src = url;
        node.classList.add('thumb');
      }
    });
  };
  load();
  return {
    update(path: string) {
      if (path === current) return;
      current = path;
      load();
    },
  };
}
