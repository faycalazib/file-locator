/**
 * Doodle icons (Microsoft Fluent Emoji "Color", MIT licence — see
 * assets/doodles/LICENSE-fluentui-emoji.txt). Bundled by Vite: works offline.
 */
import type { ResultKind } from './types';

import pdf from '../assets/doodles/pdf.svg';
import word from '../assets/doodles/word.svg';
import excel from '../assets/doodles/excel.svg';
import powerpoint from '../assets/doodles/powerpoint.svg';
import text from '../assets/doodles/text.svg';
import code from '../assets/doodles/code.svg';
import archive from '../assets/doodles/archive.svg';
import email from '../assets/doodles/email.svg';
import folderIcon from '../assets/doodles/folder.svg';
import image from '../assets/doodles/image.svg';

export { default as magnifier } from '../assets/doodles/magnifier.svg';
export { default as gem } from '../assets/doodles/gem.svg';
export { default as tulip } from '../assets/doodles/tulip.svg';
export { default as blossom } from '../assets/doodles/blossom.svg';
export { default as star } from '../assets/doodles/star.svg';
export { default as pick } from '../assets/doodles/pick.svg';
export { default as warning } from '../assets/doodles/warning.svg';
export { default as folder } from '../assets/doodles/folder.svg';
export { default as sparkles } from '../assets/doodles/sparkles.svg';

export const kindIcon: Record<ResultKind, string> = { pdf, word, excel, powerpoint, text, code, archive, email, image, folder: folderIcon };
