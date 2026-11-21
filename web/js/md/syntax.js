// What starts each kind of block: the line patterns, a table row's cells, and whether a line opens a block.

export const RE = {
  fence: /^\s*(```+|~~~+)\s*([\w+-]*)\s*$/,
  heading: /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/,
  hr: /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/,
  quote: /^\s{0,3}>\s?(.*)$/,
  item: /^(\s*)([-*+]|\d+[.)])\s+(.*)$/,
  tableSep: /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/,
};
export function cells(line) {
  const t = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  return t.split(/(?<!\\)\|/).map((c) => c.trim().replace(/\\\|/g, '|'));
}
export function startsBlock(line, next) {
  return RE.fence.test(line) || RE.heading.test(line) || RE.hr.test(line) || RE.quote.test(line) || RE.item.test(line) ||
    (line.indexOf('|') >= 0 && next != null && line.trim() !== '' && RE.tableSep.test(next) && next.indexOf('-') >= 0 && next.indexOf('|') >= 0);
}
