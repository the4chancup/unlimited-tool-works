// The maintainer's pages. Each `section.entry` is one item, keyed by its id; its radio groups
// (one per `.options`, named by the group's `data-q`) and its `textarea.own` are remembered in
// this browser, per page. "Copy my picks" puts the answered items in the clipboard as text for
// the chat: a static page cannot write to the repository, and the lead applies the answers.
(() => {
  const page = document.body.dataset.page;
  const key = "utw_maintainer_" + page;
  const saved = JSON.parse(localStorage.getItem(key) || "{}");
  const entries = [...document.querySelectorAll("section.entry")];
  const title = (entry) => entry.querySelector("h3").textContent.trim();
  const store = () => localStorage.setItem(key, JSON.stringify(saved));
  const answered = (id) => {
    const s = saved[id];
    return !!s && (Object.keys(s.picks || {}).length > 0 || !!(s.note || "").trim());
  };

  for (const entry of entries) {
    const id = entry.id;
    for (const group of entry.querySelectorAll(".options")) {
      const q = group.dataset.q || "";
      for (const radio of group.querySelectorAll("input[type=radio]")) {
        radio.name = id + ":" + q;
        radio.checked = saved[id]?.picks?.[q] === radio.value;
        radio.addEventListener("change", () => {
          saved[id] ??= {};
          (saved[id].picks ??= {})[q] = radio.value;
          store();
          refresh();
        });
      }
    }
    const own = entry.querySelector("textarea.own");
    if (own) {
      own.value = saved[id]?.note || "";
      own.addEventListener("input", () => {
        (saved[id] ??= {}).note = own.value;
        store();
        refresh();
      });
    }
  }

  // The contents: each group's heading, then its items, ticked once answered.
  const toc = document.getElementById("toc");
  for (const el of document.querySelectorAll("h2, section.entry")) {
    const li = document.createElement("li");
    const a = document.createElement("a");
    if (el.tagName === "H2") {
      if (!el.id) el.id = "g-" + el.textContent.trim().toLowerCase().replace(/\W+/g, "-");
      a.textContent = el.textContent.trim();
      li.style.listStyle = "none";
      li.style.fontWeight = "600";
      li.style.marginTop = "6px";
    } else {
      a.textContent = title(el);
      li.dataset.entry = el.id;
    }
    a.href = "#" + el.id;
    li.append(a);
    toc.append(li);
  }

  function refresh() {
    let done = 0;
    for (const entry of entries) {
      const on = answered(entry.id);
      done += on ? 1 : 0;
      entry.classList.toggle("done", on);
      toc.querySelector(`li[data-entry="${entry.id}"]`)?.classList.toggle("done", on);
    }
    document.getElementById("count").textContent = `${done} of ${entries.length} answered`;
  }

  document.getElementById("copy").addEventListener("click", () => {
    const lines = [];
    for (const entry of entries) {
      if (!answered(entry.id)) continue;
      const s = saved[entry.id];
      const picks = Object.entries(s.picks || {}).map(([q, v]) => (q ? q + ": " : "") + v);
      const note = (s.note || "").trim();
      lines.push(`[${entry.id}] ${title(entry)}: ` +
        [picks.join("; "), note && "note: " + note].filter(Boolean).join(" — "));
    }
    const out = document.getElementById("out");
    out.value = lines.length ? `${page}.html, my answers\n` + lines.join("\n") : "(nothing answered yet)";
    out.select();
    // The text stays selected in the box, so Ctrl+C still copies it where the browser refuses.
    navigator.clipboard?.writeText(out.value).catch(() => {});
  });

  document.getElementById("clear").addEventListener("click", () => {
    if (!confirm("Clear every pick and note on this page?")) return;
    localStorage.removeItem(key);
    location.reload();
  });

  refresh();
})();
