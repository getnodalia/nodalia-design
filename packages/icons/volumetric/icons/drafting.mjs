// 2D drafting: the drawing, modify, annotation and block tools laid on a drawing board.
import { sc, badge } from "../motifs.mjs";
import * as S from "../shapes.mjs";

const A = "accent";
const Z = -3; // the top of the board

// A thin drawing board with the drawing laid on it.
const board = (o = {}) => {
  const s = sc({ cy: 36, ...o });
  s.box([-17, -17, -6], [17, 17, Z]);
  return s;
};
// A drawn line: an ink outline under the colour, at board height.
const ln = (s, pts, { role = "accent-3", w = 3, closed = false, dash = null } = {}) => {
  const q = pts.map(([x, y]) => [x, y, Z]);
  if (!dash) s.line(q, { role: "ink", width: w + 2.2, closed });
  return s.line(q, { role, width: w, closed, dash });
};
const fillPoly = (s, pts, role = "accent-1", stroke = "ink") => s.plane(pts.map(([x, y]) => [x, y, Z]), { role, stroke, width: 2, layer: "over" });
const node = (s, x, y, r = 2.8, role = "accent-1") => s.dot([x, y, Z], { r, role, ring: "ink" });
const marks = (s, x0, y0, x1, y1) => ln(s, [[x0, y0], [x1, y1]], { role: "face-3", w: 1.6 });
const arrowHead = (s, [x, y], [dx, dy], size = 4) => {
  const l = Math.hypot(dx, dy) || 1, ux = dx / l, uy = dy / l;
  return ln(s, [[x - ux * size - uy * size * 0.6, y - uy * size + ux * size * 0.6], [x, y], [x - ux * size + uy * size * 0.6, y - uy * size - ux * size * 0.6]], { role: "accent-3", w: 2.2 });
};
// A dimension along p0→p1, drawn offset by `off` to its left, with end ticks.
const dim = (s, [x0, y0], [x1, y1], off = 7) => {
  const l = Math.hypot(x1 - x0, y1 - y0) || 1, nx = -(y1 - y0) / l, ny = (x1 - x0) / l;
  const a = [x0 + nx * off, y0 + ny * off], b = [x1 + nx * off, y1 + ny * off];
  ln(s, [[x0, y0], [x0 + nx * (off + 2), y0 + ny * (off + 2)]], { role: "face-3", w: 1.6 });
  ln(s, [[x1, y1], [x1 + nx * (off + 2), y1 + ny * (off + 2)]], { role: "face-3", w: 1.6 });
  ln(s, [a, b], { w: 2.4 });
  arrowHead(s, a, [x0 - x1, y0 - y1], 3.6);
  arrowHead(s, b, [x1 - x0, y1 - y0], 3.6);
  return s;
};
const ring = (s, cx, cy, r) => ln(s, S.arcPts(cx, cy, r, 0, 360, 36), { role: "face-3", w: 2.2, closed: true });
const leader = (s) => {
  ln(s, [[-13, 11], [-2, -4], [5, -4]]);
  arrowHead(s, [-13, 11], [-1, 1.4], 4.5);
  return textLines(s, 7, -8, [8, 6], 5);
};
const textLines = (s, x, y, widths, gap = 5) => { widths.forEach((w, i) => ln(s, [[x, y + i * gap], [x + w, y + i * gap]], { role: "face-3", w: 2.4 })); return s; };

export default {
  "polyline": () => {
    const s = board();
    ln(s, [[-13, 11], [-5, -9], [3, 6], [13, -12]]);
    for (const [x, y] of [[-13, 11], [-5, -9], [3, 6], [13, -12]]) node(s, x, y);
    return s;
  },
  "ray": () => {
    const s = board();
    ln(s, [[-12, 11], [12, -11]]);
    arrowHead(s, [13, -12], [1, -1], 4.5);
    return node(s, -12, 11, 3.2);
  },
  "donut": () => sc({ cy: 36 }).turned({ at: [0, 0, -5], r0: 15, r1: 15, h0: 0, h1: 6, hole: 7, kind: A }),
  "rev_cloud": () => {
    const s = board();
    const pts = [];
    for (let i = 0; i < 72; i++) { const t = (i / 72) * Math.PI * 2; pts.push([Math.cos(t) * (11 + 3 * Math.abs(Math.sin(t * 4))), Math.sin(t) * (9 + 3 * Math.abs(Math.sin(t * 4)))]); }
    return ln(s, pts, { closed: true });
  },
  "hatch": () => {
    const s = board();
    fillPoly(s, S.rect(-11, -11, 11, 11), "face-1", "ink");
    for (const c of [-14, -7, 0, 7, 14]) ln(s, [[Math.max(-11, -11 + c), Math.max(-11, -11 + c) - c], [Math.min(11, 11 + c), Math.min(11, 11 + c) - c]].map(([x, y]) => [x, y]), { w: 2.2 });
    return s;
  },
  "gradient": () => {
    const s = board();
    [["face-1", -11], ["accent-1", -5.5], ["accent-2", 0], ["accent-3", 5.5]].forEach(([r, x]) => s.plane([[x, -11, Z], [x + 5.5, -11, Z], [x + 5.5, 11, Z], [x, 11, Z]].map((p) => p), { role: r, stroke: null, layer: "over" }));
    return ln(s, [[-11, -11], [11, -11], [11, 11], [-11, 11]], { role: "ink", w: 0.1, closed: true });
  },
  "region": () => {
    const s = board();
    fillPoly(s, [[-12, -12], [8, -12], [8, -2], [-2, -2], [-2, 12], [-12, 12]]);
    return s;
  },
  "wipeout": () => {
    const s = board();
    ln(s, [[-15, -4], [15, 4]]);
    return s.box([-8, -9, Z], [9, 9, Z + 2.2]);
  },
  "m_text": () => {
    const s = board();
    ln(s, [[-12, 10], [-7, -10], [-2, 10]], { w: 3.2 });
    ln(s, [[-10, 3], [-4, 3]], { w: 2.4 });
    textLines(s, 3, -8, [9, 11, 8, 10], 5.5);
    return s;
  },
  "stretch": () => {
    const s = board();
    ln(s, [[-13, -9], [3, -9], [3, 9], [-13, 9]], { role: "face-3", closed: true });
    ln(s, [[3, -9], [9, -9], [9, 9], [3, 9]], { dash: "3 3", role: "accent-2", w: 2.4 });
    s.arrow([4, 0, Z], [17, 0, Z], { w: 3, head: 7 });
    return s;
  },
  "join": () => {
    const s = board();
    ln(s, [[-14, 8], [-3, -2]], { role: "face-3" });
    ln(s, [[-1, -3], [14, -9]], { role: "face-3" });
    ln(s, [[-3, -2], [-1, -3]]);
    return node(s, -2, -2.5, 3.6, "accent-3");
  },
  "lengthen": () => {
    const s = board();
    ln(s, [[-14, 8], [2, -2]], { role: "face-3" });
    ln(s, [[2, -2], [13, -9]], { dash: "3 3" });
    arrowHead(s, [14, -10], [1, -0.65], 4.5);
    return s;
  },
  "match_prop": () => {
    const s = board();
    s.box([-14, -14, Z], [-3, -3, Z + 3], { kind: A });
    s.box([3, 3, Z], [14, 14, Z + 3]);
    s.arrow([-4, -3, Z + 3.2], [4, 3, Z + 3.2], { w: 3, head: 6.5 });
    return s;
  },
  "dim_aligned": () => {
    const s = board();
    ln(s, [[-12, 10], [10, -12]], { role: "face-3" });
    return dim(s, [-12, 10], [10, -12], 7);
  },
  "dim_radius": () => {
    const s = board();
    ring(s, 0, 0, 12);
    ln(s, [[0, 0], [8.5, -8.5]]);
    arrowHead(s, [8.5, -8.5], [1, -1], 4.2);
    return node(s, 0, 0, 2.4, "accent-3");
  },
  "dim_diameter": () => {
    const s = board();
    ring(s, 0, 0, 12);
    ln(s, [[-8.5, 8.5], [8.5, -8.5]]);
    arrowHead(s, [8.5, -8.5], [1, -1], 4.2);
    arrowHead(s, [-8.5, 8.5], [-1, 1], 4.2);
    return s;
  },
  "dim_angle": () => {
    const s = board();
    ln(s, [[-12, 12], [-12, -12]], { role: "face-3" });
    ln(s, [[-12, 12], [12, 12]], { role: "face-3" });
    ln(s, S.arcPts(-12, 12, 17, -90, 0, 14), { w: 2.6 });
    arrowHead(s, [-12, -5], [-1, 0.25], 4);
    arrowHead(s, [5, 12], [1, 0.25], 4);
    return s;
  },
  "dim_arc": () => {
    const s = board();
    ln(s, S.arcPts(0, 0, 10, 200, 340, 14), { role: "face-3" });
    ln(s, S.arcPts(0, 0, 15, 205, 335, 14), { w: 2.6 });
    arrowHead(s, [15 * Math.cos((205 * Math.PI) / 180), 15 * Math.sin((205 * Math.PI) / 180)], [0.4, 1], 4);
    arrowHead(s, [15 * Math.cos((335 * Math.PI) / 180), 15 * Math.sin((335 * Math.PI) / 180)], [0.4, -1], 4);
    return s;
  },
  "dim_ordinate": () => {
    const s = board();
    ln(s, [[-14, 12], [-14, -12]], { role: "face-3" });
    ln(s, [[-14, 12], [14, 12]], { role: "face-3" });
    node(s, 2, -4, 2.6, "accent-3");
    ln(s, [[2, -4], [2, 4], [8, 4]]);
    ln(s, [[-14, -4], [2, -4]], { role: "face-3", dash: "2.5 3" });
    return textLines(s, 8, 4, [6], 0);
  },
  "dim_baseline": () => {
    const s = board();
    ln(s, [[-13, 12], [-13, -12]], { role: "face-3" });
    for (const [x, off] of [[0, 7], [11, 13]]) { ln(s, [[x, 12], [x, -12]], { role: "face-3", w: 1.6 }); }
    ln(s, [[-13, -7], [11, -7]], { w: 2.4 });
    ln(s, [[-13, 0], [0, 0]], { w: 2.4 });
    return s;
  },
  "dim_continue": () => {
    const s = board();
    for (const x of [-13, 0, 13]) ln(s, [[x, 10], [x, -12]], { role: "face-3", w: 1.6 });
    ln(s, [[-13, -7], [13, -7]], { w: 2.4 });
    for (const x of [-13, 0, 13]) node(s, x, -7, 2.3, "accent-3");
    return s;
  },
  "q_dim": () => {
    const s = board();
    for (const x of [-12, -4, 4, 12]) ln(s, [[x, 10], [x, -4]], { role: "face-3", w: 1.6 });
    ln(s, [[-12, -6], [12, -6]], { w: 2.4 });
    return badge(s, "plus");
  },
  "m_leader": () => leader(board()),
  "add_leader": () => badge(leader(board()), "plus"),
  "remove_leader": () => badge(leader(board()), "minus"),
  "align_leaders": () => {
    const s = board();
    for (const y of [-9, 0, 9]) { ln(s, [[-12, y + 4], [-2, y]]); ln(s, [[-2, y], [3, y]]); }
    ln(s, [[3, -12], [3, 12]], { role: "face-3", dash: "2.5 3" });
    textLines(s, 6, -9, [7, 7, 7], 9);
    return s;
  },
  "block_create": () => {
    const s = sc({ cy: 36 });
    s.box([-14, -14, -9], [14, 14, 5]);
    s.box([-6, -6, 5], [6, 6, 11], { kind: A });
    return badge(s, "plus");
  },
  "insert": () => {
    const s = board();
    s.box([-7, -7, Z], [7, 7, Z + 8], { kind: A });
    s.arrow([0, 0, 26], [0, 0, Z + 10], { w: 3, head: 7 });
    return s;
  },
  "block_edit": () => {
    const s = sc({ cy: 36 });
    s.box([-14, -14, -9], [14, 14, 5]);
    s.box([-6, -6, 5], [6, 6, 11], { kind: A });
    return badge(s, "refresh");
  },
  "blocks": () => {
    const s = sc({ cy: 36 });
    s.box([-16, -9, -10], [-3, 4, 3]);
    s.box([-2, -9, -10], [11, 4, 3], { kind: A });
    s.box([-9, -2, 3], [4, 11, 16]);
    return s;
  },
  "layers": () => {
    const s = sc({ cy: 38 });
    s.box([-15, -15, -13], [15, 15, -9]);
    s.box([-15, -15, -6], [15, 15, -2]);
    s.box([-15, -15, 1], [15, 15, 5], { kind: A });
    return s;
  },
  "properties": () => {
    const s = board();
    for (const [y, x] of [[-9, -5], [0, 5], [9, -2]]) { ln(s, [[-12, y], [12, y]], { role: "face-3", w: 2.4 }); node(s, x, y, 3.6, "accent-3"); }
    return s;
  },
  "area": () => {
    const s = board();
    fillPoly(s, [[-12, 8], [-9, -11], [10, -9], [12, 10]], "accent-1");
    textLines(s, -6, -2, [11, 8], 5);
    return s;
  },
  "distance": () => {
    const s = board();
    ln(s, [[-12, 10], [12, -10]], { w: 2.4 });
    node(s, -12, 10, 3.4, "accent-3");
    node(s, 12, -10, 3.4, "accent-3");
    return s;
  },
};
