/**
 * 文档样式修正：
 * 1) 跨文档链接 → [《侧栏标题》](path) + 两侧空格（标点旁不加）
 * 2) 同行加粗两端空格；允许加粗内含行内代码
 * 3) 仅屏蔽 ``` 围栏；侧栏不改
 */
import fs from 'fs';
import path from 'path';

const root = path.resolve('d:/code/lab/rust/async-lab');

function stripBook(s) {
  return s.replace(/^《/, '').replace(/》$/, '');
}

const sidebar = fs.readFileSync(path.join(root, '_sidebar.md'), 'utf8');
const titleByAbs = new Map();
for (const m of sidebar.matchAll(/\[([^\]]+)\]\(([^)]+\.md)\)/g)) {
  const abs = path.normalize(path.join(root, m[2].replace(/\\/g, '/')));
  titleByAbs.set(abs, stripBook(m[1]));
}

const PUNCT_AFTER = new Set([
  '。', '，', '；', '：', '、', '！', '？', '）', '》', '」', '』',
  ')', ']', '}', ',', '.', ';', ':', '!', '?', '（',
]);
const PUNCT_BEFORE = new Set([
  '（', '《', '「', '『', '(', '[', '{', '：', ':',
]);

function needsSpaceBefore(prev) {
  if (!prev || /\s/.test(prev) || PUNCT_BEFORE.has(prev)) return false;
  return true;
}
function needsSpaceAfter(next) {
  if (!next || /\s/.test(next) || PUNCT_AFTER.has(next)) return false;
  return true;
}

function protectFences(text) {
  const bins = [];
  const replaced = text.replace(/```[\s\S]*?```/g, (block) => {
    const i = bins.length;
    bins.push(block);
    return `\u0000FENCE${i}\u0000`;
  });
  return { replaced, bins };
}
function restoreFences(text, bins) {
  return text.replace(/\u0000FENCE(\d+)\u0000/g, (_, n) => bins[Number(n)]);
}

function emitWrapped(out, piece, nextChar) {
  const prev = out.length ? out[out.length - 1] : '';
  let s = '';
  if (needsSpaceBefore(prev)) s += ' ';
  s += piece;
  if (needsSpaceAfter(nextChar)) s += ' ';
  return s;
}

function resolveLink(fromFile, target) {
  const hashIdx = target.indexOf('#');
  const filePart = (hashIdx === -1 ? target : target.slice(0, hashIdx)).replace(/\\/g, '/');
  const hash = hashIdx === -1 ? '' : target.slice(hashIdx);
  const abs = path.normalize(path.join(path.dirname(fromFile), filePart));
  return { title: titleByAbs.get(abs), hash, filePart, abs };
}

/** true if index is inside inline `...` (not fence placeholders) */
function inInlineCode(text, index) {
  let inCode = false;
  for (let i = 0; i < index; i++) {
    if (text[i] === '\u0000') {
      // skip placeholder
      const end = text.indexOf('\u0000', i + 1);
      i = end === -1 ? text.length : end;
      continue;
    }
    if (text[i] === '`') inCode = !inCode;
  }
  return inCode;
}

function fixLinks(text, fromFile, report) {
  const re = /(!?)\[([^\]]*)\]\(([^)\s]+\.md(?:#[^)\s]*)?)\)/g;
  let out = '';
  let last = 0;
  let m;
  while ((m = re.exec(text)) !== null) {
    const [full, bang] = m;
    const start = m.index;
    const end = start + full.length;
    out += text.slice(last, start);
    const next = end < text.length ? text[end] : '';

    if (bang === '!' || inInlineCode(text, start)) {
      out += full;
      last = end;
      continue;
    }

    const { title, hash, filePart, abs } = resolveLink(fromFile, m[3]);
    if (!title) {
      report.invalid.push({
        file: path.relative(root, fromFile),
        target: m[3],
        abs: path.relative(root, abs),
      });
      out += emitWrapped(out, full, next);
    } else {
      out += emitWrapped(out, `[《${title}》](${filePart}${hash})`, next);
    }
    last = end;
  }
  out += text.slice(last);
  return out;
}

function fixBold(text) {
  const re = /\*\*((?:(?!\*\*)[^\n])+?)\*\*/g;
  let out = '';
  let last = 0;
  let m;
  while ((m = re.exec(text)) !== null) {
    const start = m.index;
    const end = start + m[0].length;
    out += text.slice(last, start);
    if (inInlineCode(text, start)) {
      out += m[0]; // 行内代码内的 ** 原样保留
    } else {
      const inner = m[1].replace(/^\s+|\s+$/g, '');
      const next = end < text.length ? text[end] : '';
      out += emitWrapped(out, `**${inner}**`, next);
    }
    last = end;
  }
  out += text.slice(last);
  return out;
}

function cleanupSpaces(t) {
  // 只收回「加粗/链接后误加在中文标点前」的空格；不动 ASCII 标点（避免 `serve .` → `serve.`）
  t = t.replace(/ ([。，；：、！？）》」』])/g, '$1');
  // 用 [^\s]：不要把 \r 当成内容，否则 CRLF 文件的行尾双空格硬折行会被压成单空格
  t = t.replace(/([^\s]) {2,}([^\s])/g, '$1 $2');
  return t;
}

function processFile(filePath, report) {
  const original = fs.readFileSync(filePath, 'utf8');
  const { replaced, bins } = protectFences(original);
  let t = fixLinks(replaced, filePath, report);
  t = fixBold(t);
  t = cleanupSpaces(t);
  t = restoreFences(t, bins);
  if (t !== original) {
    fs.writeFileSync(filePath, t, 'utf8');
    report.changed.push(path.relative(root, filePath));
  }
}

function walkMd(dir, out = []) {
  for (const name of fs.readdirSync(dir)) {
    const p = path.join(dir, name);
    if (fs.statSync(p).isDirectory()) walkMd(p, out);
    else if (name.endsWith('.md')) out.push(p);
  }
  return out;
}

// 先恢复 docs 到干净状态再跑？若已半处理，先 checkout 再跑
const report = { changed: [], invalid: [] };
const files = walkMd(path.join(root, 'docs'));
for (const extra of ['README.md', 'PLAN.md']) {
  const p = path.join(root, extra);
  if (fs.existsSync(p)) files.push(p);
}
for (const f of files) processFile(f, report);

console.log('Changed:', report.changed.length);
for (const f of report.changed.sort()) console.log(' -', f);
console.log('Invalid:', report.invalid.length);
for (const x of report.invalid) console.log(' !', x.file, '->', x.target);
