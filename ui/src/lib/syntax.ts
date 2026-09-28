/**
 * Lightweight syntax coloring for the code preview (goal.md §3). Line by line,
 * language-agnostic: comments, strings, numbers and the keywords of the
 * common languages. Colors come from the theme (`--syn-*`).
 */

export type SyntaxClass = 'kw' | 'str' | 'com' | 'num' | null;

export interface SyntaxToken {
  text: string;
  cls: SyntaxClass;
}

const KEYWORDS = [
  // JS / TS
  'const', 'let', 'var', 'function', 'return', 'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'break',
  'continue', 'class', 'interface', 'type', 'enum', 'import', 'export', 'from', 'as', 'default', 'new', 'this',
  'super', 'extends', 'implements', 'public', 'private', 'protected', 'static', 'async', 'await', 'yield', 'try',
  'catch', 'finally', 'throw', 'typeof', 'instanceof', 'void', 'null', 'undefined', 'true', 'false', 'of', 'in',
  // Python
  'def', 'elif', 'lambda', 'pass', 'None', 'True', 'False', 'with', 'raise', 'except', 'global', 'nonlocal', 'and',
  'or', 'not', 'is',
  // Rust / Go / C-family
  'fn', 'pub', 'use', 'mod', 'impl', 'struct', 'trait', 'match', 'loop', 'where', 'mut', 'self', 'Self', 'crate',
  'package', 'func', 'go', 'defer', 'select', 'chan', 'map', 'range', 'int', 'float', 'double', 'char', 'bool',
  'string', 'long', 'short', 'unsigned', 'final', 'abstract', 'namespace', 'using', 'override', 'virtual',
  // SQL (case-insensitive below)
  'SELECT', 'FROM', 'WHERE', 'INSERT', 'INTO', 'UPDATE', 'DELETE', 'CREATE', 'TABLE', 'ALTER', 'DROP', 'JOIN',
  'LEFT', 'RIGHT', 'INNER', 'ON', 'GROUP', 'BY', 'ORDER', 'HAVING', 'LIMIT', 'VALUES', 'SET', 'AND', 'OR', 'NOT',
  'NULL', 'PRIMARY', 'KEY', 'INDEX', 'AS',
];

const SQL_KEYWORDS = new Set(KEYWORDS.filter((k) => k === k.toUpperCase() && /^[A-Z]+$/.test(k)));
const KEYWORD_SET = new Set(KEYWORDS);

// 1 comment · 2 string · 3 number · 4 word
const TOKEN = /(\/\/.*$|#\s.*$|--\s.*$|\/\*.*?\*\/|<!--.*?-->)|("(?:\\.|[^"\\])*"?|'(?:\\.|[^'\\])*'?|`(?:\\.|[^`\\])*`?)|\b(\d+(?:\.\d+)?)\b|\b([A-Za-z_]\w*)\b/g;

/** Splits a line of code into colored tokens. */
export function tokenizeCode(line: string): SyntaxToken[] {
  const out: SyntaxToken[] = [];
  let last = 0;
  const push = (text: string, cls: SyntaxClass) => {
    if (!text) return;
    const prev = out[out.length - 1];
    if (prev && prev.cls === cls) prev.text += text;
    else out.push({ text, cls });
  };
  for (const m of line.matchAll(TOKEN)) {
    const start = m.index ?? 0;
    const word = m[4];
    const cls: SyntaxClass = m[1]
      ? 'com'
      : m[2]
        ? 'str'
        : m[3]
          ? 'num'
          : word && (KEYWORD_SET.has(word) || SQL_KEYWORDS.has(word.toUpperCase()))
            ? 'kw'
            : null;
    push(line.slice(last, start), null);
    push(m[0], cls);
    last = start + m[0].length;
  }
  push(line.slice(last), null);
  return out;
}

/**
 * Colors the plain parts of a line that also carries match marks: the whole
 * line is tokenized once (a string may contain a match), then each segment
 * gets the classes of the characters it covers.
 */
export function colorSegments<S extends { text: string; hit: boolean }>(
  segs: S[],
): (S & { parts: SyntaxToken[] })[] {
  const tokens = tokenizeCode(segs.map((s) => s.text).join(''));
  // Class of every UTF-16 offset of the line.
  const classes: SyntaxClass[] = [];
  for (const tok of tokens) for (let i = 0; i < tok.text.length; i++) classes.push(tok.cls);
  let offset = 0;
  return segs.map((seg) => {
    const parts: SyntaxToken[] = [];
    for (let i = 0; i < seg.text.length; i++) {
      const cls = classes[offset + i] ?? null;
      const prev = parts[parts.length - 1];
      if (prev && prev.cls === cls) prev.text += seg.text[i];
      else parts.push({ text: seg.text[i] ?? '', cls });
    }
    offset += seg.text.length;
    return { ...seg, parts };
  });
}
