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

/** A drawn recipe card, for the browser demo (screenshots). */
export const DEMO_IMAGE =
  'data:image/svg+xml;utf8,' +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="800" height="1000" viewBox="0 0 800 1000">
      <rect width="800" height="1000" fill="#f7f3ea"/>
      <text x="60" y="120" font-family="Georgia" font-size="56" fill="#222">Fluffy pancakes</text>
      <text x="60" y="200" font-family="Georgia" font-size="32" fill="#333">Recipe card · makes 8</text>
      <line x1="60" y1="250" x2="740" y2="250" stroke="#999" stroke-width="2"/>
      <text x="60" y="320" font-family="Georgia" font-size="30" fill="#222">200 g flour · 2 eggs · 300 ml milk</text>
      <text x="60" y="370" font-family="Georgia" font-size="30" fill="#222">1 tbsp sugar · a pinch of salt</text>
      <text x="60" y="450" font-family="Georgia" font-size="28" fill="#444">Whisk, rest 10 minutes, then cook</text>
      <text x="60" y="495" font-family="Georgia" font-size="28" fill="#444">about one minute on each side.</text>
      <text x="500" y="880" font-family="Georgia" font-size="30" fill="#222">Serve warm!</text>
    </svg>`,
  );

/** Boxes of "recipe" on the demo card (fractions). */
export const DEMO_BOXES = [{ x: 0.072, y: 0.172, w: 0.116, h: 0.036, fuzzy: false }];

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
