(function () {
  "use strict";

  var root = document.documentElement;

  /* ---- Theme toggle + persistence ---- */
  var themeToggle = document.getElementById("theme-toggle");
  if (themeToggle) {
    themeToggle.addEventListener("click", function () {
      var next = root.getAttribute("data-theme") === "light" ? "dark" : "light";
      root.setAttribute("data-theme", next);
      try {
        localStorage.setItem("radianite-theme", next);
      } catch (e) {}
    });
  }

  /* ---- Mobile nav toggle ---- */
  var navToggle = document.getElementById("nav-toggle");
  var nav = document.getElementById("primary-nav");
  if (navToggle && nav) {
    var setNav = function (open) {
      nav.classList.toggle("open", open);
      navToggle.setAttribute("aria-expanded", String(open));
      navToggle.setAttribute("aria-label", open ? "Close menu" : "Open menu");
    };
    navToggle.addEventListener("click", function () {
      setNav(!nav.classList.contains("open"));
    });
    nav.addEventListener("click", function (e) {
      if (e.target.tagName === "A") setNav(false);
    });
  }

  /* ---- Resolve latest .exe from GitHub release ---- */
  var downloadLinks = document.querySelectorAll(".js-download");
  if (downloadLinks.length) {
    fetch("https://api.github.com/repos/radcolor-dev/radiante_val/releases/latest", {
      headers: { Accept: "application/vnd.github+json" }
    })
      .then(function (res) {
        if (!res.ok) throw new Error("GitHub API " + res.status);
        return res.json();
      })
      .then(function (release) {
        var assets = release.assets || [];
        var exe = assets.filter(function (a) {
          return /\.exe$/i.test(a.name);
        })[0];
        if (exe && exe.browser_download_url) {
          downloadLinks.forEach(function (link) {
            link.href = exe.browser_download_url;
            link.setAttribute("download", "");
          });
        }
      })
      .catch(function () {
        /* keep the /releases/latest fallback already in the markup */
      });
  }

  /* ---- Scroll reveal (progressive enhancement) ---- */
  var revealables = document.querySelectorAll(".reveal");
  if ("IntersectionObserver" in window && revealables.length) {
    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
    );
    revealables.forEach(function (el) {
      observer.observe(el);
    });
  } else {
    revealables.forEach(function (el) {
      el.classList.add("is-visible");
    });
  }
})();
