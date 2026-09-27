import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { parse } from "svelte/compiler";

// Check the actual Svelte markup so a wide clickable wrapper cannot quietly
// reintroduce lyrics navigation while the metadata callbacks still pass tests.
function template(relativePath) {
  const source = readFileSync(new URL(relativePath, import.meta.url), "utf8");
  const tree = parse(source, { modern: true });
  const elements = [];
  function visit(node, ancestors = []) {
    if (!node || typeof node !== "object") return;
    if (Array.isArray(node)) {
      node.forEach((child) => visit(child, ancestors));
      return;
    }
    if (node.type === "RegularElement" || node.type === "Component") {
      elements.push({ node, ancestors });
    }
    for (const [key, child] of Object.entries(node)) {
      if (["metadata", "loc", "name_loc", "attributes"].includes(key)) continue;
      if (child && typeof child === "object") visit(child, [...ancestors, node]);
    }
  }
  visit(tree.fragment);
  return { source, elements };
}

function attribute(node, name) {
  return node.attributes?.find((candidate) => candidate.type === "Attribute" && candidate.name === name);
}

function textAttribute(node, name) {
  const value = attribute(node, name)?.value;
  return Array.isArray(value) ? value.map((part) => part.type === "Text" ? part.data : "").join("") : undefined;
}

function hasClass(node, name) {
  return textAttribute(node, "class")?.split(/\s+/).includes(name);
}

function expression(source, node, name) {
  const value = attribute(node, name)?.value;
  const part = Array.isArray(value) ? value[0] : value;
  return part?.expression ? source.slice(part.expression.start, part.expression.end) : null;
}

test("browsing history has one persistent header outside the conditional sidebar and scrolling page", () => {
  const { elements } = template("../src/routes/+page.svelte");
  const headers = elements.filter(({ node }) => hasClass(node, "workspace-navigation"));
  assert.equal(headers.length, 1);
  const header = headers[0];
  assert.ok(!header.ancestors.some((node) => node.type === "IfBlock" || node.name === "main"));
  const controls = elements.filter(({ node }) => textAttribute(node, "aria-label") === "View history");
  assert.equal(controls.length, 1);
  assert.ok(controls[0].ancestors.includes(header.node));
  const arrows = elements.filter(({ node }) => ["Back to previous view", "Forward to next view"].includes(textAttribute(node, "aria-label")));
  assert.equal(arrows.length, 2);
  assert.ok(arrows.every(({ node, ancestors }) => node.name === "button" && ancestors.includes(header.node)));
});

test("the player opens lyrics only from the Lyrics button, never its empty space, cover or title", () => {
  const { source, elements } = template("../src/lib/components/NowPlayingBar.svelte");
  const lyricsClicks = elements.filter(({ node }) => expression(source, node, "onclick")?.includes("onOpenLyrics"));
  assert.equal(lyricsClicks.length, 1);
  assert.equal(lyricsClicks[0].node.name, "button");
  assert.equal(textAttribute(lyricsClicks[0].node, "aria-label"), "Open lyrics");
  const passive = elements.filter(({ node }) => node.name === "footer" || ["track", "track-copy", "track-meta", "cover", "track-title"].some((name) => hasClass(node, name)));
  assert.equal(passive.length, 6);
  assert.ok(passive.every(({ node }) => !attribute(node, "onclick")));
  assert.ok(!elements.some(({ node }) => hasClass(node, "track-open")));
  assert.ok(!source.includes("onOpenNowPlaying"));
});

test("player artist and album are separate keyboard buttons wired to existing grouping-aware navigation", () => {
  const bar = template("../src/lib/components/NowPlayingBar.svelte");
  const links = bar.elements.filter(({ node }) => hasClass(node, "track-link"));
  assert.equal(links.length, 2);
  assert.ok(links.every(({ node }) => node.name === "button" && textAttribute(node, "type") === "button"));
  assert.match(expression(bar.source, links[0].node, "onclick"), /onArtistSelect\?\.\(track\)/);
  assert.match(expression(bar.source, links[1].node, "onclick"), /onAlbumSelect\?\.\(track\)/);
  const page = template("../src/routes/+page.svelte");
  const player = page.elements.find(({ node }) => node.name === "NowPlayingBar").node;
  assert.equal(expression(page.source, player, "onArtistSelect"), "handleTrackArtistSelect");
  assert.equal(expression(page.source, player, "onAlbumSelect"), "handleTrackAlbumSelect");
  assert.equal(expression(page.source, player, "onOpenLyrics"), "handleLyricsSelect");
  assert.ok(!attribute(player, "onOpenNowPlaying"));
});

test("detail sort choices remain open for direction changes and close only on explicit dismissal", () => {
  const menu = template("../src/lib/components/TrackSortMenu.svelte");
  const radioButtons = menu.elements.filter(({ node }) => textAttribute(node, "role") === "menuitemradio");
  assert.equal(radioButtons.length, 1); // One Svelte each-block template for every option.
  assert.equal(expression(menu.source, radioButtons[0].node, "onclick"), "() => onSortChange(option.value)");
  const direction = menu.elements.find(({ node }) => textAttribute(node, "role") === "menuitemcheckbox")?.node;
  assert.ok(direction);
  assert.match(expression(menu.source, direction, "onclick"), /onDirectionChange/);
  assert.ok(!expression(menu.source, direction, "onclick").includes("closeMenu"));
  assert.match(menu.source, /function handleOutsidePointer\(event: PointerEvent\)/);
  assert.match(menu.source, /event\.key === "Escape"/);
  assert.ok(menu.elements.some(({ node }) => textAttribute(node, "aria-label") === "Close sort menu"));
});

test("artist and genre details render incremental sorted songs and Songs-page controls reset on re-entry", () => {
  const page = template("../src/routes/+page.svelte");
  const songsSections = page.elements.filter(({ node }) => node.name === "LibrarySection" && textAttribute(node, "title") === "Songs"
    && /selected(Artist|Genre)Tracks\.length/.test(expression(page.source, node, "viewAllLabel") ?? ""));
  assert.equal(songsSections.length, 2);
  const detailLists = page.elements.filter(({ node }) => node.name === "TrackList" &&
    ["selectedArtistDisplayTracks.slice(0, artistTrackVisibleLimit)", "selectedGenreDisplayTracks.slice(0, genreTrackVisibleLimit)"].includes(expression(page.source, node, "tracks")));
  assert.equal(detailLists.length, 2);
  assert.ok(detailLists.every(({ ancestors }) => songsSections.some(({ node }) => ancestors.includes(node))));
  assert.match(page.source, /if \(targetView === "Songs" && activeView !== "Songs"\) resetSongsBrowser\(\)/);
  assert.match(page.source, /if \(location\.view === "Songs" && activeView !== "Songs"\) resetSongsBrowser\(\)/);
  for (const reset of ['songSort = "title"', 'songSortDirection = "asc"', 'songFormatFilter = "All"', 'searchQuery = ""']) {
    assert.ok(page.source.includes(reset));
  }
});
