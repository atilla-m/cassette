import assert from "node:assert/strict";
import test from "node:test";
import { statsRangeForPeriod } from "../src/lib/utils/statsPeriod.ts";
import { resolveStatsPlayback, statsDayValueLabel } from "../src/lib/utils/listeningStats.ts";

process.env.TZ = "America/New_York";

test("uses local calendar midnights through spring and autumn DST", () => {
  const spring = statsRangeForPeriod("today", new Date(2026, 2, 8, 12));
  const autumn = statsRangeForPeriod("today", new Date(2026, 10, 1, 12));
  assert.equal(spring.endUtc - spring.startUtc, 23 * 3600);
  assert.equal(autumn.endUtc - autumn.startUtc, 25 * 3600);
  assert.deepEqual(spring.chartDates, ["2026-03-08"]);
  assert.equal(spring.dayBoundariesUtc[0], spring.startUtc);
  assert.equal(spring.dayBoundariesUtc.at(-1), spring.endUtc);
});

test("weeks start Monday and cross month/year boundaries correctly", () => {
  const range = statsRangeForPeriod("week", new Date(2026, 0, 4, 23));
  assert.equal(range.chartDates[0], "2025-12-29");
  assert.equal(range.chartDates.at(-1), "2026-01-04");
  const next = statsRangeForPeriod("week", new Date(2026, 0, 5));
  assert.equal(next.chartDates[0], "2026-01-05");
});

test("month/year selection preserves leap days and previous-year rollover", () => {
  const february = statsRangeForPeriod("month", new Date(2024, 1, 15));
  assert.equal(february.chartDates.length, 29);
  const previous = statsRangeForPeriod("previousMonth", new Date(2026, 0, 1));
  assert.equal(previous.chartDates[0], "2025-12-01");
  assert.equal(previous.chartDates.at(-1), "2025-12-31");
  const year = statsRangeForPeriod("previousYear", new Date(2025, 0, 1));
  assert.equal(year.chartDates.length, 366);
  const chosenMonth = statsRangeForPeriod("previousMonth", new Date(), "", "", "2024-02");
  assert.equal(chosenMonth.chartDates.length, 29);
  assert.equal(chosenMonth.chartDates[0], "2024-02-01");
  const chosenYear = statsRangeForPeriod("previousYear", new Date(), "", "", "", "2024");
  assert.equal(chosenYear.chartDates.length, 366);
  assert.equal(statsRangeForPeriod("previousMonth", new Date(), "", "", "2024-13"), null);
  assert.equal(statsRangeForPeriod("previousYear", new Date(), "", "", "", "abcd"), null);
});

test("custom dates are inclusive local dates, invalid dates fail, and all time is unbounded", () => {
  const range = statsRangeForPeriod("custom", new Date(), "2026-03-07", "2026-03-09");
  assert.deepEqual(range.chartDates, ["2026-03-07", "2026-03-08", "2026-03-09"]);
  assert.equal(range.endUtc - range.startUtc, 71 * 3600);
  assert.equal(statsRangeForPeriod("custom", new Date(), "2026-02-30", "2026-03-09"), null);
  assert.equal(statsRangeForPeriod("custom", new Date(), "2026-03-10", "2026-03-09"), null);
  const all = statsRangeForPeriod("all", new Date(2026, 2, 9));
  assert.equal(all.startUtc, null);
  assert.equal(all.endUtc, null);
  assert.equal(all.chartDates.length, 30);
});

test("same-day custom ranges include the whole local day across DST", () => {
  for (const [day, hours] of [["2026-03-08", 23], ["2026-11-01", 25], ["2026-09-30", 24]]) {
    const range = statsRangeForPeriod("custom", new Date(), day, day);
    assert.ok(range);
    assert.deepEqual(range.chartDates, [day]);
    assert.equal(range.endUtc - range.startUtc, hours * 3600);
    const events = [range.startUtc - 1, range.startUtc, range.endUtc - 1, range.endUtc];
    assert.deepEqual(events.filter((event) => event >= range.startUtc && event < range.endUtc), [range.startUtc, range.endUtc - 1]);
  }
});

test("period playback/context actions use authoritative tracks and omit unavailable history", () => {
  const actual = { id: "a", playCount: 99, filePath: "/synthetic/a.wav" };
  const another = { id: "b", playCount: 11, filePath: "/synthetic/b.wav" };
  const library = new Map([["a", actual], ["b", another]]);
  const rows = [{ ...actual, playCount: 2 }, { id: "pending:missing", playCount: 30 }, { ...another, playCount: 1 }];
  const selection = resolveStatsPlayback("a", rows, library);
  assert.strictEqual(selection.track, actual);
  assert.deepEqual(selection.queue, [actual, another]);
  assert.equal(rows[0].playCount, 2);
  assert.equal(resolveStatsPlayback("pending:missing", rows, library), null);
});

test("selected chart-day values refresh with the snapshot, including zero and changed periods", () => {
  const day = "2026-03-08";
  const dates = [day, "2026-03-09"];
  assert.equal(statsDayValueLabel(day, dates, [0, 2]), `${day}: 0 plays`);
  assert.equal(statsDayValueLabel(day, dates, [1, 2]), `${day}: 1 play`);
  assert.equal(statsDayValueLabel(day, ["2026-04-01"], [3]), null);
  assert.equal(statsDayValueLabel(null, dates, [1, 2]), null);
  assert.equal(statsDayValueLabel(day, dates, []), null);
});
