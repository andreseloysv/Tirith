// The flow canvas's zoom: fit to the canvas or a chosen scale, applied to the stage column and kept on resize.
import { useState, useEffect, useLayoutEffect, useRef } from '../../../vendor/preact-htm.js';

/** `[refs, setZoom, step]`: refs of the canvas, zoomer and stage; `step(k)` zooms by k from the scale shown. */
export function useZoom(id, sig) {
  const cv = useRef(null), zr = useRef(null), zs = useRef(null);
  const [zoom, setZoom] = useState({ fit: true, z: 1 });
  const apply = () => {
    const c = cv.current, s = zs.current, r = zr.current;
    if (!c || !s || !r) return;
    const w = s.offsetWidth, h = s.offsetHeight;
    const sc = zoom.fit ? Math.max(0.35, Math.min(1, (c.clientWidth - 48) / w, (c.clientHeight - 110) / h)) : zoom.z;
    if (c.dataset.sc === String(sc) && c.dataset.wh === `${w}x${h}`) return;
    s.style.transform = `scale(${sc})`;
    r.style.width = `${Math.round(w * sc)}px`; r.style.height = `${Math.round(h * sc)}px`;
    const zv = c.parentNode.querySelector('.zv');
    if (zv) zv.textContent = `${Math.round(sc * 100)}%`;
    c.dataset.sc = String(sc); c.dataset.wh = `${w}x${h}`;
  };
  const applyRef = useRef(apply);
  applyRef.current = apply;
  useLayoutEffect(apply, [zoom, id, sig]);
  useEffect(() => {
    let t = 0;
    const ro = new ResizeObserver(() => { cancelAnimationFrame(t); t = requestAnimationFrame(() => { if (cv.current) cv.current.dataset.sc = ''; applyRef.current(); }); });
    if (cv.current) ro.observe(cv.current);
    return () => { ro.disconnect(); cancelAnimationFrame(t); };
  }, []);
  const step = (k) => {
    const sc = Number(cv.current && cv.current.dataset.sc) || 1;
    setZoom({ fit: false, z: Math.max(0.35, Math.min(1.6, Math.round((sc + k) * 20) / 20)) });
  };
  return [{ cv, zr, zs }, setZoom, step];
}
