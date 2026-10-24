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
  const BLOCK_SEPARATOR = "\u0000";

  const search = { query: "", caseSensitive: false, ranges: [], index: -1 };

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

    if (search.query) {
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

  function computeMatches() {
    search.ranges = [];
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
      const range = document.createRange();
      range.setStart(...position(pieces, at, false));
      range.setEnd(...position(pieces, at + query.length, true));
      search.ranges.push(range);
    }
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
    };
  }

  function find(query, caseSensitive) {
    search.query = String(query ?? "");
    search.caseSensitive = Boolean(caseSensitive);
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
    search.query = "";
    search.ranges = [];
    search.index = -1;
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
    update, setStyle, find, step, clear, finish, state,
    selectionText, scrollPosition, restoreScroll, scrollToFragment,
  });
})();
