// Strata Mundi wiki: theme switch, remembered language, search on the page's index.
(function () {
  "use strict";
  function stored(key) { try { return localStorage.getItem(key); } catch (e) { return null; } }
  function store(key, value) { try { localStorage.setItem(key, value); } catch (e) {} }

  var theme = stored("strata-wiki-theme");
  if (theme) { document.documentElement.setAttribute("data-theme", theme); }

  function fold(text) {
    return text.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "");
  }

  function search(input, list, root) {
    var words = fold(input.value).split(/\s+/).filter(Boolean);
    list.textContent = "";
    if (!words.length || !window.SEARCH_INDEX) { list.hidden = true; return; }
    var inTitle = function (page) {
      var title = fold(page.t);
      return words.every(function (w) { return title.indexOf(w) >= 0; });
    };
    var found = window.SEARCH_INDEX.filter(function (page) {
      var hay = fold(page.t + " " + page.s);
      return words.every(function (w) { return hay.indexOf(w) >= 0; });
    });
    // Pages named after the search first, then those that only mention it.
    found = found.filter(inTitle).concat(found.filter(function (p) { return !inTitle(p); }))
      .slice(0, 20);
    found.forEach(function (page) {
      var item = document.createElement("li");
      var link = document.createElement("a");
      link.href = root + page.u;
      link.textContent = page.t;
      item.appendChild(link);
      list.appendChild(item);
    });
    list.hidden = found.length === 0;
  }

  document.addEventListener("DOMContentLoaded", function () {
    var button = document.getElementById("theme");
    if (button) {
      button.addEventListener("click", function () {
        var dark = document.documentElement.getAttribute("data-theme") === "dark" ||
          (!document.documentElement.getAttribute("data-theme") &&
            window.matchMedia("(prefers-color-scheme: dark)").matches);
        var next = dark ? "light" : "dark";
        document.documentElement.setAttribute("data-theme", next);
        store("strata-wiki-theme", next);
      });
    }
    store("strata-wiki-lang", document.body.getAttribute("data-lang"));
    var input = document.getElementById("search");
    var list = document.getElementById("results");
    var root = document.body.getAttribute("data-root") || "";
    if (input && list) {
      input.addEventListener("input", function () { search(input, list, root); });
      input.addEventListener("keydown", function (event) {
        var first = list.querySelector("a");
        if (event.key === "Enter" && first) { location.href = first.href; }
        if (event.key === "Escape") { input.value = ""; list.hidden = true; }
      });
    }
  });
})();
