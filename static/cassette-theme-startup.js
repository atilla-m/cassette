try {
  if (window.localStorage.getItem("cassette:theme") === "light") {
    document.documentElement.dataset.theme = "light";
  }
} catch {
  // The app uses its default theme when storage is unavailable.
}
