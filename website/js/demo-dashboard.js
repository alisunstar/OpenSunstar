(() => {
  const data = window.OpenSunstarDemoData;
  if (!data) return;

  const projectList = document.querySelector("[data-demo-project-list]");
  const scoreValue = document.querySelector("[data-demo-score]");
  const scoreBar = document.querySelector("[data-demo-score-bar]");
  const projectName = document.querySelector("[data-demo-project-name]");
  const projectPath = document.querySelector("[data-demo-project-path]");
  const projectRisk = document.querySelector("[data-demo-project-risk]");
  const projectSummary = document.querySelector("[data-demo-project-summary]");
  const nextList = document.querySelector("[data-demo-next]");
  const command = document.querySelector("[data-demo-command]");
  const matrix = document.querySelector("[data-demo-matrix]");
  const modeButtons = document.querySelectorAll("[data-demo-target]");
  const panels = document.querySelectorAll("[data-demo-panel]");

  let selectedProject = data.projects[0];

  function activatePanel(targetId) {
    modeButtons.forEach((button) => {
      button.classList.toggle("active", button.dataset.demoTarget === targetId);
    });
    panels.forEach((panel) => {
      panel.classList.toggle("active", panel.id === targetId);
    });
  }

  function statusLabel(status) {
    return data.statusLabels[status] ?? status;
  }

  function renderProjects() {
    if (!projectList) return;

    projectList.innerHTML = data.projects
      .map(
        (project) => `
          <button
            class="demo-project ${project.id === selectedProject.id ? "active" : ""}"
            type="button"
            data-project-id="${project.id}"
          >
            <span>
              <strong>${project.name}</strong>
              <small>${project.path}</small>
            </span>
            <b>${project.score}</b>
          </button>
        `,
      )
      .join("");
  }

  function renderMatrix() {
    if (!matrix) return;

    const header = `
      <div class="demo-matrix-cell demo-matrix-head">工具</div>
      ${data.assets
        .map(
          (asset) =>
            `<div class="demo-matrix-cell demo-matrix-head">${asset.label}</div>`,
        )
        .join("")}
    `;
    const rows = data.tools
      .map((tool) => {
        const cells = data.assets
          .map((asset) => {
            const status =
              selectedProject.matrix[tool]?.[asset.key] ?? "missing";
            return `
              <div class="demo-matrix-cell">
                <span class="demo-status demo-status-${status}">
                  ${statusLabel(status)}
                </span>
              </div>
            `;
          })
          .join("");
        return `<div class="demo-matrix-cell demo-tool-name">${tool}</div>${cells}`;
      })
      .join("");

    matrix.innerHTML = `${header}${rows}`;
  }

  function renderProjectDetail() {
    scoreValue.textContent = selectedProject.score;
    scoreBar.style.width = `${selectedProject.score}%`;
    projectName.textContent = selectedProject.name;
    projectPath.textContent = selectedProject.path;
    projectRisk.textContent = selectedProject.risk;
    projectSummary.textContent = selectedProject.summary;
    command.textContent = selectedProject.command;
    nextList.innerHTML = selectedProject.next
      .map((item) => `<li>${item}</li>`)
      .join("");
    renderMatrix();
  }

  function render() {
    renderProjects();
    renderProjectDetail();
  }

  modeButtons.forEach((button) => {
    button.addEventListener("click", () =>
      activatePanel(button.dataset.demoTarget),
    );
  });

  projectList?.addEventListener("click", (event) => {
    const button = event.target.closest("[data-project-id]");
    if (!button) return;

    selectedProject =
      data.projects.find(
        (project) => project.id === button.dataset.projectId,
      ) ?? selectedProject;
    render();
  });

  render();
})();
