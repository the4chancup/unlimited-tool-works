// The roadmap page's timeline layout: the vertical positions `roadmap.py` leaves out of
// the SVG it writes, computed from the rows' order and line counts so a worklog row added
// moves only its own lines of the page. The page runs this on opening; `roadmap_test.py`
// runs it under node. Pure: no DOM, nothing else at top level.
function layout(input) {
  // One entry per slot past the last: a row's height is its lines plus the row padding,
  // the bare "now" line's empty slot one line. tops[0] is 0.
  const tops = [0];
  for (const slot of input.slots)
    tops.push(tops[tops.length - 1] +
      (slot.gap ? input.lineHeight : slot.lines * input.lineHeight + input.rowPad));

  // A row's y is the middle of its first line.
  const firstLine = Math.floor((input.rowPad + input.lineHeight) / 2);
  const rows = {};
  input.slots.forEach((slot, i) => {
    if (!slot.gap) rows[slot.path] = tops[i] + firstLine;
  });

  const tracks = {};
  for (const track of input.tracks)
    tracks[track.key] = [rows[track.first], rows[track.last]];

  const edges = input.edges.map(edge => {
    const y1 = rows[edge.from], y2 = rows[edge.to], ym = (y1 + y2) / 2;
    return `M ${edge.x1} ${y1} C ${edge.x1} ${ym}, ${edge.x2} ${ym}, ${edge.x2} ${y2}`;
  });

  let now;
  if (input.nowRows.length) {
    // The band covers the in-progress rows, from the first's top to the first slot
    // after the last of them.
    const indices = input.nowRows.map(path =>
      input.slots.findIndex(slot => slot.path === path));
    const top = tops[Math.min(...indices)];
    const bottom = tops[Math.max(...indices) + 1];
    now = {band: [top, bottom - top], labelY: (top + bottom) / 2};
  } else {
    // The bare line sits at the middle of the empty slot, its label above it.
    const gap = input.slots.findIndex(slot => slot.gap);
    const y = (tops[gap] + tops[gap + 1]) / 2;
    now = {line: y, labelY: y - 7};
  }
  return {height: tops[tops.length - 1], rows, tracks, edges, now};
}
