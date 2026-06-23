(() => {
  const navLinks = document.querySelectorAll(".nav-links a[data-section]");
  const sections = document.querySelectorAll("section[id]");

  const setActiveNav = () => {
    let current = "";
    sections.forEach((section) => {
      const top = section.offsetTop - 100;
      if (window.scrollY >= top) current = section.id;
    });
    navLinks.forEach((link) => {
      link.classList.toggle("active", link.dataset.section === current);
    });
  };

  window.addEventListener("scroll", setActiveNav, { passive: true });
  setActiveNav();

  document.querySelectorAll(".platform-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      const target = tab.dataset.platform;
      document
        .querySelectorAll(".platform-tab")
        .forEach((t) => t.classList.remove("active"));
      document
        .querySelectorAll(".platform-panel")
        .forEach((p) => p.classList.remove("active"));
      tab.classList.add("active");
      document.getElementById(`platform-${target}`)?.classList.add("active");
    });
  });

  document.querySelectorAll(".copy-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const code = btn.closest(".code-block")?.querySelector("code");
      if (!code) return;
      const text = code.innerText;
      try {
        await navigator.clipboard.writeText(text);
        const original = btn.textContent;
        btn.textContent = "已复制";
        setTimeout(() => {
          btn.textContent = original;
        }, 1500);
      } catch {
        btn.textContent = "复制失败";
      }
    });
  });
})();
