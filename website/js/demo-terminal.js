(() => {
  const data = window.OpenSunstarDemoData;
  if (!data) return;

  const commandList = document.querySelector("[data-demo-command-list]");
  const terminal = document.querySelector("[data-demo-terminal-output]");
  const title = document.querySelector("[data-demo-terminal-title]");

  let selectedCommand = data.terminalCommands[0];

  function formatCommand(command) {
    return [
      `$ ${command.label}`,
      JSON.stringify(command.output, null, 2),
      "",
      command.output.exitCode
        ? `exit code: ${command.output.exitCode}`
        : "exit code: 0",
    ].join("\n");
  }

  function renderCommandList() {
    commandList.innerHTML = data.terminalCommands
      .map(
        (command) => `
          <button
            class="demo-command ${command.id === selectedCommand.id ? "active" : ""}"
            type="button"
            data-command-id="${command.id}"
          >
            ${command.label}
          </button>
        `,
      )
      .join("");
  }

  function renderTerminal() {
    title.textContent = selectedCommand.label;
    terminal.textContent = formatCommand(selectedCommand);
  }

  commandList?.addEventListener("click", (event) => {
    const button = event.target.closest("[data-command-id]");
    if (!button) return;

    selectedCommand =
      data.terminalCommands.find(
        (command) => command.id === button.dataset.commandId,
      ) ?? selectedCommand;
    renderCommandList();
    renderTerminal();
  });

  renderCommandList();
  renderTerminal();
})();
