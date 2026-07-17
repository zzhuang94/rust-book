import fs from 'fs';
import path from 'path';

const root = path.resolve('d:/code/lab/rust/async-lab');
const sidebar = fs.readFileSync(path.join(root, '_sidebar.md'), 'utf8');
const titles = new Map();
for (const m of sidebar.matchAll(/\[([^\]]+)\]\(([^)]+\.md)\)/g)) {
  titles.set(path.normalize(path.join(root, m[2].replace(/\\/g, '/'))), m[1]);
}

function walk(d, a = []) {
  for (const n of fs.readdirSync(d)) {
    const p = path.join(d, n);
    if (fs.statSync(p).isDirectory()) walk(p, a);
    else if (n.endsWith('.md')) a.push(p);
  }
  return a;
}

const files = walk(path.join(root, 'docs'));
for (const extra of ['README.md', 'PLAN.md']) {
  const p = path.join(root, extra);
  if (fs.existsSync(p)) files.push(p);
}

let badLink = 0;
let wrongTitle = 0;
const stuck = [];

for (const f of files) {
  let t = fs.readFileSync(f, 'utf8');
  t = t.replace(/```[\s\S]*?```/g, '');
  for (const m of t.matchAll(/(?<!!)\[([^\]]*)\]\(([^)]+\.md[^)]*)\)/g)) {
    const label = m[1];
    const target = m[2].split('#')[0];
    if (!label.startsWith('《') || !label.endsWith('》')) {
      badLink++;
      console.log('no-book:', path.relative(root, f), m[0].slice(0, 80));
      continue;
    }
    const abs = path.normalize(path.join(path.dirname(f), target));
    const expect = titles.get(abs);
    const got = label.slice(1, -1);
    if (expect && expect !== got) {
      wrongTitle++;
      console.log('title:', path.relative(root, f), JSON.stringify(got), '!=', JSON.stringify(expect));
    }
  }
  t = t.replace(/`[^`]*`/g, '');
  let idx = 0;
  while ((idx = t.indexOf('**', idx)) !== -1) {
    const end = t.indexOf('**', idx + 2);
    if (end === -1) break;
    const before = idx > 0 ? t[idx - 1] : '';
    const after = end + 2 < t.length ? t[end + 2] : '';
    const stuckBefore = before && /[\u4e00-\u9fffA-Za-z0-9]/.test(before);
    const stuckAfter = after && /[\u4e00-\u9fffA-Za-z0-9]/.test(after);
    if (stuckBefore || stuckAfter) {
      stuck.push(
        `${stuckBefore ? 'L' : ''}${stuckAfter ? 'R' : ''} ${path.relative(root, f)} |${t
          .slice(Math.max(0, idx - 10), Math.min(t.length, end + 12))
          .replace(/\n/g, '↵')}|`,
      );
    }
    idx = end + 2;
  }
}

console.log({ badLink, wrongTitle, stuck: stuck.length });
stuck.slice(0, 30).forEach((s) => console.log(s));
