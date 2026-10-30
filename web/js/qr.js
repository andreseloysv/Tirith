/* A QR code drawn in the page: byte mode, error correction M, versions 1 to 20 (up to 666 bytes), no network and no
   library. Follows the ISO 18004 steps (data, Reed-Solomon, interleave, function patterns, mask with the lowest penalty).
   T.qrMatrix(text) -> array of rows of booleans (true = dark) or null when the text does not fit; T.qrSvg(text, label). */
(function () {
  var T = window.T || (window.T = {});
  var MAX_VER = 20;
  /* error correction M: codewords per block and number of blocks, indexed by version */
  var ECC_LEN = [0, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26];
  var BLOCKS = [0, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16];

  function rawModules(v) {
    var n = (16 * v + 128) * v + 64, a;
    if (v >= 2) { a = Math.floor(v / 7) + 2; n -= (25 * a - 10) * a - 55; if (v >= 7) n -= 36; }
    return n;
  }
  function dataWords(v) { return Math.floor(rawModules(v) / 8) - ECC_LEN[v] * BLOCKS[v]; }

  /* Reed-Solomon over GF(256), polynomial 0x11D */
  function mul(x, y) { var z = 0, i; for (i = 7; i >= 0; i--) { z = (z << 1) ^ ((z >>> 7) * 0x11D); z ^= ((y >>> i) & 1) * x; } return z; }
  function divisor(deg) {
    var r = [], i, j, root = 1;
    for (i = 0; i < deg - 1; i++) r.push(0);
    r.push(1);
    for (i = 0; i < deg; i++) {
      for (j = 0; j < deg; j++) { r[j] = mul(r[j], root); if (j + 1 < deg) r[j] ^= r[j + 1]; }
      root = mul(root, 2);
    }
    return r;
  }
  function remainder(data, div) {
    var r = div.map(function () { return 0; });
    data.forEach(function (b) {
      var f = b ^ r.shift(); r.push(0);
      div.forEach(function (c, i) { r[i] ^= mul(c, f); });
    });
    return r;
  }

  function utf8(text) {
    var s = unescape(encodeURIComponent(text)), out = [], i;
    for (i = 0; i < s.length; i++) out.push(s.charCodeAt(i));
    return out;
  }

  function encodeData(bytes) {
    var v, bits, cap, i;
    for (v = 1; v <= MAX_VER; v++) {
      cap = dataWords(v) * 8;
      if (4 + (v < 10 ? 8 : 16) + bytes.length * 8 <= cap) break;
    }
    if (v > MAX_VER) return null;
    bits = [];
    function put(val, len) { for (var k = len - 1; k >= 0; k--) bits.push((val >>> k) & 1); }
    put(4, 4); put(bytes.length, v < 10 ? 8 : 16);
    bytes.forEach(function (b) { put(b, 8); });
    put(0, Math.min(4, cap - bits.length));
    put(0, (8 - bits.length % 8) % 8);
    for (i = 0xEC; bits.length < cap; i ^= 0xEC ^ 0x11) put(i, 8);
    var words = [];
    for (i = 0; i < bits.length; i += 8) words.push(parseInt(bits.slice(i, i + 8).join(''), 2));
    return { ver: v, words: words };
  }

  function interleave(ver, data) {
    var nb = BLOCKS[ver], el = ECC_LEN[ver], raw = Math.floor(rawModules(ver) / 8), nshort = nb - raw % nb,
        shortLen = Math.floor(raw / nb), div = divisor(el), blocks = [], k = 0, i, out = [];
    for (i = 0; i < nb; i++) {
      var dat = data.slice(k, k + shortLen - el + (i < nshort ? 0 : 1)); k += dat.length;
      var ecc = remainder(dat, div);
      if (i < nshort) dat.push(0);   /* placeholder so the blocks line up; skipped below */
      blocks.push(dat.concat(ecc));
    }
    for (i = 0; i < blocks[0].length; i++) {
      blocks.forEach(function (b, j) { if (i !== shortLen - el || j >= nshort) out.push(b[i]); });
    }
    return out;
  }

  function build(ver, codewords) {
    var size = ver * 4 + 17, m = [], fn = [], x, y, i;
    for (y = 0; y < size; y++) { m.push([]); fn.push([]); for (x = 0; x < size; x++) { m[y].push(false); fn[y].push(false); } }
    function set(px, py, dark) { if (px >= 0 && px < size && py >= 0 && py < size) { m[py][px] = dark; fn[py][px] = true; } }
    for (i = 0; i < size; i++) { set(6, i, i % 2 === 0); set(i, 6, i % 2 === 0); }
    function finder(cx, cy) {
      for (var dy = -4; dy <= 4; dy++) for (var dx = -4; dx <= 4; dx++) {
        var d = Math.max(Math.abs(dx), Math.abs(dy)); set(cx + dx, cy + dy, d !== 2 && d !== 4);
      }
    }
    finder(3, 3); finder(size - 4, 3); finder(3, size - 4);
    var na = ver === 1 ? 0 : Math.floor(ver / 7) + 2, pos = [], step, a, b;
    if (na) {
      step = Math.ceil((ver * 4 + 4) / (na * 2 - 2)) * 2; pos = [6];
      for (i = size - 7; pos.length < na; i -= step) pos.splice(1, 0, i);
    }
    for (a = 0; a < pos.length; a++) for (b = 0; b < pos.length; b++) {
      if ((a === 0 && b === 0) || (a === 0 && b === pos.length - 1) || (a === pos.length - 1 && b === 0)) continue;
      for (var dy = -2; dy <= 2; dy++) for (var dx = -2; dx <= 2; dx++) set(pos[a] + dx, pos[b] + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1);
    }
    function formatBits(mask) {
      var data = mask, rem = data, k;   /* error correction M is 0b00 */
      for (k = 0; k < 10; k++) rem = (rem << 1) ^ ((rem >>> 9) * 0x537);
      return ((data << 10) | rem) ^ 0x5412;
    }
    function drawFormat(mask) {
      var bits = formatBits(mask), k, bit = function (n) { return ((bits >>> n) & 1) !== 0; };
      for (k = 0; k <= 5; k++) set(8, k, bit(k));
      set(8, 7, bit(6)); set(8, 8, bit(7)); set(7, 8, bit(8));
      for (k = 9; k < 15; k++) set(14 - k, 8, bit(k));
      for (k = 0; k < 8; k++) set(size - 1 - k, 8, bit(k));
      for (k = 8; k < 15; k++) set(8, size - 15 + k, bit(k));
      set(8, size - 8, true);
    }
    drawFormat(0);
    if (ver >= 7) {
      var rem = ver, vb;
      for (i = 0; i < 12; i++) rem = (rem << 1) ^ ((rem >>> 11) * 0x1F25);
      vb = (ver << 12) | rem;
      for (i = 0; i < 18; i++) { var bt = ((vb >>> i) & 1) !== 0, p = size - 11 + i % 3, q = Math.floor(i / 3); set(p, q, bt); set(q, p, bt); }
    }
    /* codewords in the zigzag */
    var n = 0;
    for (var right = size - 1; right >= 1; right -= 2) {
      if (right === 6) right = 5;
      for (var vert = 0; vert < size; vert++) for (var j = 0; j < 2; j++) {
        x = right - j; y = ((right + 1) & 2) === 0 ? size - 1 - vert : vert;
        if (!fn[y][x] && n < codewords.length * 8) { m[y][x] = ((codewords[n >>> 3] >>> (7 - (n & 7))) & 1) !== 0; n++; }
      }
    }
    var MASKS = [
      function (x, y) { return (x + y) % 2 === 0; }, function (x, y) { return y % 2 === 0; },
      function (x, y) { return x % 3 === 0; }, function (x, y) { return (x + y) % 3 === 0; },
      function (x, y) { return (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0; },
      function (x, y) { return x * y % 2 + x * y % 3 === 0; },
      function (x, y) { return (x * y % 2 + x * y % 3) % 2 === 0; },
      function (x, y) { return ((x + y) % 2 + x * y % 3) % 2 === 0; }
    ];
    function applyMask(k) { for (var yy = 0; yy < size; yy++) for (var xx = 0; xx < size; xx++) if (!fn[yy][xx] && MASKS[k](xx, yy)) m[yy][xx] = !m[yy][xx]; }
    var best = 0, bestScore = Infinity, k;
    for (k = 0; k < 8; k++) {
      applyMask(k); drawFormat(k);
      var sc = penalty(m, size);
      if (sc < bestScore) { bestScore = sc; best = k; }
      applyMask(k);   /* undo */
    }
    applyMask(best); drawFormat(best);
    return m;
  }

  function penalty(m, size) {
    var score = 0, x, y, k, dark = 0;
    function line(get) {
      var run = 1, i;
      for (i = 1; i <= size; i++) {
        if (i < size && get(i) === get(i - 1)) run++;
        else { if (run >= 5) score += 3 + run - 5; run = 1; }
      }
      /* finder-like pattern 1011101 with four light modules on a side */
      var s = '', p;
      for (i = 0; i < size; i++) s += get(i) ? '1' : '0';
      p = /(?=(00001011101|10111010000))/g;
      while (p.exec(s)) { score += 40; p.lastIndex++; }
    }
    for (y = 0; y < size; y++) line(function (i) { return m[y][i]; });
    for (x = 0; x < size; x++) line(function (i) { return m[i][x]; });
    for (y = 0; y < size - 1; y++) for (x = 0; x < size - 1; x++) {
      if (m[y][x] === m[y][x + 1] && m[y][x] === m[y + 1][x] && m[y][x] === m[y + 1][x + 1]) score += 3;
    }
    for (y = 0; y < size; y++) for (x = 0; x < size; x++) if (m[y][x]) dark++;
    k = Math.ceil(Math.abs(dark * 20 - size * size * 10) / (size * size)) - 1;
    return score + Math.max(0, k) * 10;
  }

  T.qrMatrix = function (text) {
    var d = encodeData(utf8(String(text)));
    return d ? build(d.ver, interleave(d.ver, d.words)) : null;
  };
  /* inline SVG, black on white with the 4-module quiet zone; the colours do not follow the theme so a camera can read it */
  T.qrSvg = function (text, label) {
    var m = T.qrMatrix(text);
    if (!m) return '';
    var size = m.length, q = 4, path = '', x, y, w;
    for (y = 0; y < size; y++) for (x = 0; x < size; x++) {
      if (!m[y][x]) continue;
      for (w = 1; x + w < size && m[y][x + w]; w++);
      path += 'M' + (x + q) + ' ' + (y + q) + 'h' + w + 'v1h-' + w + 'z'; x += w - 1;
    }
    var side = size + q * 2;
    return '<svg class="qr" viewBox="0 0 ' + side + ' ' + side + '" role="img" aria-label="' + String(label || 'QR code').replace(/[&<>"']/g, '') + '" shape-rendering="crispEdges">' +
      '<rect width="' + side + '" height="' + side + '" fill="#fff"/><path d="' + path + '" fill="#000"/></svg>';
  };
})();
