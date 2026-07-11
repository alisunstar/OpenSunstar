(() => {
  const data = window.OpenSunstarDemoData;
  if (!data) return;

  const backendButtons = document.querySelectorAll("[data-sync-backend]");
  const encryptionToggle = document.querySelector("[data-sync-encryption]");
  const syncButton = document.querySelector("[data-sync-run]");
  const syncBackend = document.querySelector("[data-sync-backend-label]");
  const syncEndpoint = document.querySelector("[data-sync-endpoint]");
  const syncLatency = document.querySelector("[data-sync-latency]");
  const syncManifest = document.querySelector("[data-sync-manifest]");
  const syncLog = document.querySelector("[data-sync-log]");
  const securityButtons = document.querySelectorAll("[data-security-state]");
  const securityOutput = document.querySelector("[data-security-output]");
  const securityTitle = document.querySelector("[data-security-title]");

  let selectedBackend = data.syncBackends[0];
  let selectedSecurityState = "manifest";

  function manifest() {
    return {
      manifestVersion: 2,
      backend: selectedBackend.id,
      endpoint: selectedBackend.endpoint,
      encrypted: Boolean(encryptionToggle.checked),
      algorithm: encryptionToggle.checked ? "AES-256-GCM" : "plain-json",
      kdfSalt: encryptionToggle.checked
        ? "7df1c24e8a0b4d19b65c9e332b1a0f5e"
        : null,
      assets: ["mcp", "skills", "prompts", "commands", "permissions"],
      snapshotDigest: "sha256:8f3d9e4c0c5b7e2c9f01f6a4d2b8a2d9",
      retention: 10,
    };
  }

  function syncSteps() {
    const mode = encryptionToggle.checked ? "加密快照" : "明文快照";
    return [
      `读取本地 ~/.OpenSunstar/OpenSunstar.db`,
      `生成 ${mode} 与 manifest`,
      `上传至 ${selectedBackend.label}`,
      `校验远端 SHA256 摘要`,
      `保留最近 10 份快照`,
    ];
  }

  function renderSync() {
    syncBackend.textContent = selectedBackend.label;
    syncEndpoint.textContent = selectedBackend.endpoint;
    syncLatency.textContent = selectedBackend.latency;
    syncManifest.textContent = JSON.stringify(manifest(), null, 2);
    syncLog.innerHTML = syncSteps()
      .map((step, index) => `<li><span>${index + 1}</span>${step}</li>`)
      .join("");
  }

  function renderSecurity() {
    const state = data.securityStates[selectedSecurityState];
    securityTitle.textContent = state.label;
    securityOutput.textContent = JSON.stringify(state.output, null, 2);

    securityButtons.forEach((button) => {
      button.classList.toggle(
        "active",
        button.dataset.securityState === selectedSecurityState,
      );
    });
  }

  backendButtons.forEach((button) => {
    button.addEventListener("click", () => {
      selectedBackend =
        data.syncBackends.find(
          (backend) => backend.id === button.dataset.syncBackend,
        ) ?? selectedBackend;
      backendButtons.forEach((item) => item.classList.remove("active"));
      button.classList.add("active");
      renderSync();
    });
  });

  encryptionToggle?.addEventListener("change", renderSync);
  syncButton?.addEventListener("click", () => {
    renderSync();
    syncButton.textContent = "已生成快照";
    setTimeout(() => {
      syncButton.textContent = "模拟同步";
    }, 1400);
  });

  securityButtons.forEach((button) => {
    button.addEventListener("click", () => {
      selectedSecurityState = button.dataset.securityState;
      renderSecurity();
    });
  });

  renderSync();
  renderSecurity();
})();
