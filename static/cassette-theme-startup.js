try {
  const savedTheme = window.localStorage.getItem("cassette:theme");
  if (["obsidian", "cassette-teal", "glacier", "light"].includes(savedTheme)) {
    document.documentElement.dataset.theme = savedTheme;
  }
} catch {
  // The app uses its default theme when storage is unavailable.
}
