// Flow simulation (CFD): cases, meshes, physics, solvers, runs and results.
import { sc, badge } from "../motifs.mjs";
import * as S from "../shapes.mjs";

const A = "accent";

// The flow domain every case icon starts from: a long duct.
const duct = (s, { z0 = -8, z1 = 4, kind = "face" } = {}) => s.box([-17, -11, z0], [17, 11, z1], { kind });
// A stream line across the top of a body, as a gentle S curve.
const stream = (s, y, z, { x0 = -13, x1 = 13, amp = 2.4, role = "accent-3", width = 2.6 } = {}) => {
  const pts = [];
  for (let i = 0; i <= 24; i++) { const t = i / 24; pts.push([x0 + (x1 - x0) * t, y + amp * Math.sin(t * Math.PI * 2), z]); }
  return s.line(pts, { role, width });
};

export default {
  "cfd_case": () => {
    const s = duct(sc({ cy: 36 }));
    stream(s, -4, 4);
    return stream(s, 4, 4);
  },
  "cfd_mesh": () => {
    const s = sc({ cy: 36 });
    s.box([-14, -14, -8], [14, 14, 4]);
    for (const t of [-4.7, 4.7]) {
      s.line([[t, -14, 4], [t, 14, 4]], { role: "accent-3", width: 1.8 });
      s.line([[-14, t, 4], [14, t, 4]], { role: "accent-3", width: 1.8 });
      s.line([[14, t, 4], [14, t, -8]], { role: "face-3", width: 1.6 });
      s.line([[t, 14, 4], [t, 14, -8]], { role: "face-3", width: 1.6 });
    }
    return s;
  },
  "cfd_boundary": () => {
    const s = duct(sc({ cy: 36 }));
    s.plane([[17.1, -11, -8], [17.1, 11, -8], [17.1, 11, 4], [17.1, -11, 4]], { role: "accent-2", stroke: "ink", width: 2, layer: "over" });
    return s.arrow([28, 0, -2], [19, 0, -2], { w: 2.8, head: 6.5 });
  },
  "cfd_fluid": () => {
    const s = sc({ cy: 37 });
    s.cylinder([0, 0, -13], 13, 14, { kind: A });
    s.turned({ at: [0, 0, 1], r0: 13, r1: 13, h0: 0, h1: 11, hole: 10.5 });
    return s;
  },
  "cfd_turbulence": () => {
    const s = sc({ cy: 34 });
    s.box([-16, -16, -8], [16, 16, -5]);
    const pts = [];
    for (let i = 0; i <= 60; i++) { const t = i / 60, a = t * Math.PI * 4.2, r = 2.5 + 13 * t; pts.push([Math.cos(a) * r, Math.sin(a) * r, -5]); }
    s.line(pts, { role: "ink", width: 6.6 });
    s.line(pts, { role: "accent-3", width: 3.8 });
    return s.dot([0, 0, -4], { r: 2.8, role: "accent-1", ring: "ink" });
  },
  "cfd_viscosity": () => {
    const s = sc({ cy: 33, k: 1.25 });
    s.sphere([0, 0, -5], 11, { kind: A });
    s.turned({ at: [0, 0, 1], r0: 9.4, r1: 0, h0: 0, h1: 17, kind: A });
    return s;
  },
  "cfd_numerics": () => {
    const s = sc({ cy: 36 });
    s.box([-16, -16, -9], [16, 16, -6]);
    for (const [i, x] of [-14.5, -4, 6.5].entries()) for (const [j, y] of [-14.5, -4, 6.5].entries()) s.box([x, y, -6], [x + 8, y + 8, i === j ? 2 : -2], { kind: i === j ? A : "face" });
    return s;
  },
  "cfd_schemes": () => {
    const s = sc({ cy: 36 });
    s.box([-15, -5, -8], [-5, 5, 0]);
    s.box([5, -5, -8], [15, 5, 0]);
    s.box([-5, -15, -8], [5, -5, 0]);
    s.box([-5, 5, -8], [5, 15, 0]);
    return s.box([-5, -5, -8], [5, 5, 3], { kind: A });
  },
  "cfd_probe": () => {
    const s = sc({ cy: 36 });
    s.box([-16, -16, -12], [16, 16, -8]);
    s.turned({ at: [0, 0, -8], r0: 0, r1: 3.4, h0: 0, h1: 18, kind: A });
    s.cylinder([0, 0, 10], 3.4, 6);
    return s.dot([0, 0, -8], { r: 3, role: "accent-3", ring: "ink", layer: "under" });
  },
  "cfd_results": () => {
    const s = sc({ cy: 36 });
    s.box([-16, -16, -8], [16, 16, 0]);
    const band = (a, b, role) => s.plane([[-16 + a, -16, 0.05], [-16 + b, -16, 0.05], [-16 + b, 16, 0.05], [-16 + a, 16, 0.05]], { role, stroke: null, layer: "over" });
    band(0, 8, "accent-1"); band(8, 16, "accent-2"); band(16, 24, "accent-3"); band(24, 32, "face-3");
    return s;
  },
  "cfd_streamlines": () => {
    const s = sc({ cy: 36 });
    s.box([-16, -14, -9], [16, 14, -6]);
    for (const [y, amp] of [[-8, 3.4], [0, 4.4], [8, 3.4]]) stream(s, y, -6, { x0: -14, x1: 14, amp, role: "accent-3", width: 3 });
    return s;
  },
  "cfd_residuals": () => {
    const s = sc({ cy: 36, k: 1.3 });
    s.box([-17, -9, -12], [17, 9, -9]);
    for (const [i, h] of [16, 11, 7, 4].entries()) s.box([-15 + i * 8, -6, -9], [-9 + i * 8, 6, -9 + h], { kind: i === 3 ? A : "face" });
    return s;
  },
  "cfd_slice": () => {
    const s = sc({ cy: 36 });
    s.box([-14, -14, -9], [14, 14, 5]);
    s.plane([[0, -19, -13], [0, 19, -13], [0, 19, 9], [0, -19, 9]], { role: "accent-1", stroke: "ink", width: 2, layer: "over" });
    return s;
  },
  "pause": () => sc({ cy: 34 })
    .box([-12, -10, -4], [-2, 10, 4], { kind: A })
    .box([3, -10, -4], [13, 10, 4], { kind: A }),
  "stop": () => sc({ cy: 34 }).box([-12, -12, -6], [12, 12, 6], { kind: A }),
  "run": () => {
    const s = duct(sc({ cy: 36 }));
    return badge(s, "play");
  },
};
