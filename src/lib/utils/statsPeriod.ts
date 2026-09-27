export type StatsPeriod = "all" | "today" | "week" | "month" | "previousMonth" | "year" | "previousYear" | "custom";

export type StatsRange = {
  startUtc: number | null;
  endUtc: number | null;
  chartDates: string[];
  dayBoundariesUtc: number[];
};

function localDateLabel(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function parseLocalDate(value: string) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(year, month - 1, day);
  return localDateLabel(date) === value ? date : null;
}

export function statsRangeForPeriod(
  period: StatsPeriod,
  now = new Date(),
  customFrom = "",
  customTo = "",
  selectedMonth = "",
  selectedYear = "",
): StatsRange | null {
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  let start: Date;
  let end: Date;

  switch (period) {
    case "all":
      // The all-time ranking includes undated legacy totals. The chart explicitly
      // shows just the most recent 30 local calendar days of dated events.
      start = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 29);
      end = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1);
      break;
    case "today":
      start = today;
      end = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1);
      break;
    case "week": {
      // ISO-style Monday start, independent of locale's display language.
      const mondayOffset = (today.getDay() + 6) % 7;
      start = new Date(today.getFullYear(), today.getMonth(), today.getDate() - mondayOffset);
      end = new Date(start.getFullYear(), start.getMonth(), start.getDate() + 7);
      break;
    }
    case "month":
      start = new Date(today.getFullYear(), today.getMonth(), 1);
      end = new Date(today.getFullYear(), today.getMonth() + 1, 1);
      break;
    case "previousMonth":
      if (selectedMonth) {
        const chosen = parseLocalDate(`${selectedMonth}-01`);
        if (!chosen) return null;
        start = chosen;
        end = new Date(chosen.getFullYear(), chosen.getMonth() + 1, 1);
      } else {
        start = new Date(today.getFullYear(), today.getMonth() - 1, 1);
        end = new Date(today.getFullYear(), today.getMonth(), 1);
      }
      break;
    case "year":
      start = new Date(today.getFullYear(), 0, 1);
      end = new Date(today.getFullYear() + 1, 0, 1);
      break;
    case "previousYear":
      if (selectedYear && (!/^\d{4}$/.test(selectedYear) || Number(selectedYear) < 1970)) return null;
      start = new Date(selectedYear ? Number(selectedYear) : today.getFullYear() - 1, 0, 1);
      end = new Date(start.getFullYear() + 1, 0, 1);
      break;
    case "custom": {
      const from = parseLocalDate(customFrom);
      const through = parseLocalDate(customTo);
      if (!from || !through || from > through) return null;
      start = from;
      end = new Date(through.getFullYear(), through.getMonth(), through.getDate() + 1);
      break;
    }
  }

  const chartDates: string[] = [];
  const dayBoundariesUtc: number[] = [];
  for (let day = new Date(start); day < end; day = new Date(day.getFullYear(), day.getMonth(), day.getDate() + 1)) {
    if (chartDates.length >= 36625) return null;
    chartDates.push(localDateLabel(day));
    dayBoundariesUtc.push(Math.floor(day.getTime() / 1000));
  }
  dayBoundariesUtc.push(Math.floor(end.getTime() / 1000));
  return {
    startUtc: period === "all" ? null : Math.floor(start.getTime() / 1000),
    endUtc: period === "all" ? null : Math.floor(end.getTime() / 1000),
    chartDates,
    dayBoundariesUtc,
  };
}
