// Script injected into the isolated "smv" content world of every viewer page
// (STANDARD §4.2). Document content cannot run JavaScript; the app drives the
// page exclusively through the functions exported on `smv`.
"use strict";

(() => {
  const CONTENT_ID = "smv-content";
  const USER_STYLE_ID = "smv-user";
  const MAX_MATCHES = 10000;
  // Text in different blocks is never joined into one match (STANDARD §6).
  const BLOCK_TAGS = new Set([
    "ADDRESS", "ARTICLE", "ASIDE", "BLOCKQUOTE", "BR", "CAPTION", "DD", "DETAILS",
    "DIV", "DL", "DT", "FIGCAPTION", "FIGURE", "FOOTER", "H1", "H2", "H3", "H4",
    "H5", "H6", "HEADER", "HR", "LI", "MAIN", "NAV", "OL", "P", "PRE", "SECTION",
    "SUMMARY", "TABLE", "TBODY", "TD", "TFOOT", "TH", "THEAD", "TR", "UL",
  ]);
  const SKIPPED_TAGS = new Set(["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"]);
  const HEADING_TAGS = new Set(["H1", "H2", "H3", "H4", "H5", "H6"]);
  const BLOCK_SEPARATOR = "\u0000";

  // `kind` is "keyword", "regex" or "ai" (STANDARD §6.2). `blocks` holds the
  // ranges of the blocks collected for the last AI search.
  const search = {
    kind: "keyword",
    query: "",
    caseSensitive: false,
    invalid: false,
    ranges: [],
    index: -1,
    blocks: [],
  };

  const content = () => document.getElementById(CONTENT_ID);

  // Replaces the rendered document while keeping unchanged top-level nodes,
  // so images do not reload and the scroll position stays put (STANDARD §4.4).
  function update(html) {
    const container = content();
    const template = document.createElement("template");
    template.innerHTML = html;
    const next = Array.from(template.content.childNodes);
    const current = Array.from(container.childNodes);

    let prefix = 0;
    while (prefix < next.length && prefix < current.length &&
           current[prefix].isEqualNode(next[prefix])) {
      prefix++;
    }
    let suffix = 0;
    while (suffix < next.length - prefix && suffix < current.length - prefix &&
           current[current.length - 1 - suffix].isEqualNode(next[next.length - 1 - suffix])) {
      suffix++;
    }

    const anchor = current[current.length - suffix] ?? null;
    for (const node of current.slice(prefix, current.length - suffix)) {
      node.remove();
    }
    for (const node of next.slice(prefix, next.length - suffix)) {
      container.insertBefore(node, anchor);
    }

    if (search.kind === "ai") {
      // The blocks of the old content are gone; the app runs the AI search
      // again (STANDARD §6.1).
      search.blocks = [];
      search.ranges = [];
      search.index = -1;
      paint();
    } else if (search.query) {
      const previous = search.index;
      computeMatches();
      search.index = search.ranges.length
        ? Math.min(Math.max(previous, 0), search.ranges.length - 1)
        : -1;
      paint();
    }
    return state();
  }

  function setStyle(css) {
    document.getElementById(USER_STYLE_ID).textContent = css;
  }

  // --- Search -------------------------------------------------------------

  function blockOf(node) {
    let element = node.parentElement;
    while (element && element.id !== CONTENT_ID && !BLOCK_TAGS.has(element.tagName)) {
      element = element.parentElement;
    }
    return element;
  }

  function isSkipped(node) {
    for (let element = node.parentElement; element && element.id !== CONTENT_ID;
         element = element.parentElement) {
      if (SKIPPED_TAGS.has(element.tagName)) {
        return true;
      }
    }
    return false;
  }

  // Concatenates the document text. Each text node is remembered with its
  // offset; a separator is inserted wherever the enclosing block changes.
  function collectText() {
    const walker = document.createTreeWalker(content(), NodeFilter.SHOW_TEXT);
    const pieces = [];
    let text = "";
    let block;
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (!node.data || isSkipped(node)) {
        continue;
      }
      const nodeBlock = blockOf(node);
      if (nodeBlock !== block) {
        text += BLOCK_SEPARATOR;
        block = nodeBlock;
      }
      pieces.push({ node, start: text.length });
      text += node.data;
    }
    return { text, pieces };
  }

  // Lower-cases code point by code point, keeping the string length so that
  // offsets still match the original text.
  function fold(text) {
    let folded = "";
    for (const ch of text) {
      const lower = ch.toLowerCase();
      folded += lower.length === ch.length ? lower : ch;
    }
    return folded;
  }

  // Every white space character matches a space (same length mapping).
  const normalizeSpace = (text) => text.replace(/\s/g, " ");

  // Maps a text offset to a DOM position. An end offset that falls on a node
  // boundary maps to the end of the preceding node.
  function position(pieces, offset, isEnd) {
    let low = 0;
    let high = pieces.length - 1;
    while (low < high) {
      const mid = (low + high + 1) >> 1;
      const start = pieces[mid].start;
      if (start < offset || (!isEnd && start === offset)) {
        low = mid;
      } else {
        high = mid - 1;
      }
    }
    const piece = pieces[low];
    return [piece.node, offset - piece.start];
  }

  function addRange(pieces, start, end) {
    const range = document.createRange();
    range.setStart(...position(pieces, start, false));
    range.setEnd(...position(pieces, end, true));
    search.ranges.push(range);
  }

  function computeMatches() {
    search.ranges = [];
    search.invalid = false;
    if (search.kind === "regex") {
      computeRegexMatches();
      return;
    }
    const fit = (text) => normalizeSpace(search.caseSensitive ? text : fold(text));
    const query = fit(search.query);
    if (!query) {
      return;
    }
    const { text, pieces } = collectText();
    const haystack = fit(text);
    for (let at = haystack.indexOf(query);
         at !== -1 && search.ranges.length < MAX_MATCHES;
         at = haystack.indexOf(query, at + query.length)) {
      addRange(pieces, at, at + query.length);
    }
  }

  // Regular expression search (STANDARD §6.5). Each block is matched on its
  // own, so a match never crosses blocks and `^` and `$` match at the edges of
  // a block. White space counts as a space, as in keyword search.
  function computeRegexMatches() {
    if (!search.query) {
      return;
    }
    let pattern;
    try {
      pattern = new RegExp(search.query, search.caseSensitive ? "gu" : "giu");
    } catch {
      search.invalid = true;
      return;
    }
    const { text, pieces } = collectText();
    const haystack = normalizeSpace(text);
    for (let start = 0; start <= haystack.length && search.ranges.length < MAX_MATCHES;) {
      let end = haystack.indexOf(BLOCK_SEPARATOR, start);
      if (end === -1) {
        end = haystack.length;
      }
      for (const match of haystack.slice(start, end).matchAll(pattern)) {
        if (!match[0].length) {
          continue;
        }
        addRange(pieces, start + match.index, start + match.index + match[0].length);
        if (search.ranges.length >= MAX_MATCHES) {
          break;
        }
      }
      start = end + 1;
    }
  }

  // The blocks of the document for AI search (STANDARD §6.6): each run of
  // text inside one block, with the titles of the headings it belongs to.
  // Runs without text are left out.
  function aiBlocks(query) {
    clear();
    search.kind = "ai";
    search.query = String(query ?? "");
    const walker = document.createTreeWalker(content(), NodeFilter.SHOW_TEXT);
    const titles = [];
    const runs = [];
    let block;
    let run = null;
    let heading = null;
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (!node.data || isSkipped(node)) {
        continue;
      }
      const nodeBlock = blockOf(node);
      if (!run || nodeBlock !== block) {
        block = nodeBlock;
        const level = block && HEADING_TAGS.has(block.tagName) ? Number(block.tagName[1]) : 0;
        if (level && block !== heading) {
          heading = block;
          titles.length = level - 1;
          titles[level - 1] = block.textContent.replace(/\s+/g, " ").trim();
        }
        run = {
          first: node,
          last: node,
          text: "",
          heading: level > 0,
          headings: titles.slice(0, level ? level - 1 : titles.length).filter(Boolean),
        };
        runs.push(run);
      }
      run.last = node;
      run.text += node.data;
    }
    const kept = runs.filter((r) => r.text.trim());
    search.blocks = kept.map((r) => {
      const range = document.createRange();
      range.setStart(r.first, 0);
      range.setEnd(r.last, r.last.data.length);
      return range;
    });
    return kept.map((r) => ({ text: r.text, headings: r.headings, heading: r.heading }));
  }

  // Highlights the blocks the model chose, in document order. The current
  // result stays current while more results arrive.
  function showBlocks(indices) {
    const current = search.ranges[search.index];
    search.ranges = Array.from(indices ?? [], Number)
      .sort((a, b) => a - b)
      .map((i) => search.blocks[i])
      .filter(Boolean);
    search.index = current ? search.ranges.indexOf(current) : -1;
    const fresh = search.index < 0 && search.ranges.length > 0;
    if (fresh) {
      search.index = firstVisibleIndex();
    }
    paint();
    if (fresh) {
      reveal(search.ranges[search.index]);
    }
    return state();
  }

  function paint() {
    const registry = globalThis.CSS?.highlights;
    if (!registry) {
      return;
    }
    const all = new Highlight();
    for (const range of search.ranges) {
      all.add(range);
    }
    registry.set("smv-find", all);
    const current = search.ranges[search.index];
    if (current) {
      registry.set("smv-find-current", new Highlight(current));
    } else {
      registry.delete("smv-find-current");
    }
  }

  // Index of the first match that is not above the visible area.
  function firstVisibleIndex() {
    const ranges = search.ranges;
    let low = 0;
    let high = ranges.length;
    while (low < high) {
      const mid = (low + high) >> 1;
      if (ranges[mid].getBoundingClientRect().bottom < 0) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    return low < ranges.length ? low : 0;
  }

  // Opens collapsed <details> around the match and scrolls it into view.
  function reveal(range) {
    for (let element = range.startContainer.parentElement; element;
         element = element.parentElement) {
      if (element.tagName === "DETAILS" && !element.open) {
        element.open = true;
      }
    }
    const rect = range.getBoundingClientRect();
    for (let element = range.startContainer.parentElement;
         element && element.id !== CONTENT_ID; element = element.parentElement) {
      if (element.scrollWidth > element.clientWidth) {
        const box = element.getBoundingClientRect();
        if (rect.left < box.left || rect.right > box.right) {
          element.scrollLeft += rect.left - box.left - element.clientWidth / 3;
        }
        break;
      }
    }
    const visible = range.getBoundingClientRect();
    const margin = window.innerHeight * 0.15;
    if (visible.top < margin || visible.bottom > window.innerHeight - margin) {
      window.scrollBy(0, visible.top - window.innerHeight / 3);
    }
  }

  function state() {
    return {
      count: search.ranges.length,
      index: search.index,
      capped: search.ranges.length >= MAX_MATCHES,
      invalid: search.invalid,
    };
  }

  // Keyword search, or regular expression search when `regex` is set.
  function find(query, caseSensitive, regex) {
    search.kind = regex ? "regex" : "keyword";
    search.query = String(query ?? "");
    search.caseSensitive = Boolean(caseSensitive);
    search.blocks = [];
    computeMatches();
    search.index = search.ranges.length ? firstVisibleIndex() : -1;
    paint();
    if (search.index >= 0) {
      reveal(search.ranges[search.index]);
    }
    return state();
  }

  function step(backwards) {
    const count = search.ranges.length;
    if (count) {
      search.index = (search.index + (backwards ? count - 1 : 1)) % count;
      paint();
      reveal(search.ranges[search.index]);
    }
    return state();
  }

  function clear() {
    search.kind = "keyword";
    search.query = "";
    search.invalid = false;
    search.ranges = [];
    search.index = -1;
    search.blocks = [];
    globalThis.CSS?.highlights?.delete("smv-find");
    globalThis.CSS?.highlights?.delete("smv-find-current");
    return state();
  }

  // Ends a search; the current match becomes the selection (STANDARD §6).
  function finish() {
    const current = search.ranges[search.index];
    clear();
    if (current) {
      const selection = window.getSelection();
      selection.removeAllRanges();
      selection.addRange(current);
    }
    return state();
  }

  // --- Misc ---------------------------------------------------------------

  const selectionText = () => String(window.getSelection() ?? "");
  const scrollPosition = () => window.scrollY;
  const restoreScroll = (y) => window.scrollTo(0, Number(y) || 0);

  function scrollToFragment(fragment) {
    let id = String(fragment ?? "");
    try {
      id = decodeURIComponent(id);
    } catch {
      // keep the raw fragment
    }
    const target = document.getElementById(id) ?? document.getElementsByName(id)[0];
    target?.scrollIntoView();
    return Boolean(target);
  }

  globalThis.smv = Object.freeze({
    update, setStyle, find, step, clear, finish, state, aiBlocks, showBlocks,
    selectionText, scrollPosition, restoreScroll, scrollToFragment,
  });
})();
