// Runs before first paint (CSP forbids inline scripts, hence a file).
(function () {
  try {
    var t = localStorage.getItem("downpipe:theme");
    if (t === "dark" || t === "light") document.documentElement.setAttribute("data-theme", t);
  } catch {
    // Storage blocked: fall back to the system theme.
  }
})();
