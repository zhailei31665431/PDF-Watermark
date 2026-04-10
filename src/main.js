import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

const state = {
  inputPath: "",
  outputPath: "",
  watermarkText: "CONFIDENTIAL",
  progress: 0,
  current: 0,
  total: 0,
  busy: false,
  status: "Please choose a PDF file.",
};

const app = document.querySelector("#app");

app.innerHTML = `
  <main class="shell">
    <section class="panel">
      <div class="field-group">
        <label class="field-label">PDF File</label>
        <div class="file-row">
          <button id="pick-file" class="button secondary">Choose PDF</button>
          <div id="file-path" class="path empty">No file selected</div>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label" for="watermark-input">Watermark Text</label>
        <input id="watermark-input" class="input" maxlength="64" value="${state.watermarkText}" placeholder="Enter watermark text, for example CONFIDENTIAL" />
      </div>

      <div class="field-group">
        <label class="field-label">Output File</label>
        <div class="file-row">
          <button id="pick-output" class="button secondary">Save As</button>
          <div id="output-path" class="path empty">No output path selected</div>
        </div>
      </div>

      <div class="actions">
        <button id="start" class="button primary">Add Watermark</button>
      </div>

      <section class="progress-card">
        <div class="progress-header">
          <span>Progress</span>
          <span id="progress-text">0%</span>
        </div>
        <div class="progress-track">
          <div id="progress-bar" class="progress-bar"></div>
        </div>
        <div id="page-text" class="page-text">0 / 0</div>
        <div id="status" class="status" aria-live="polite">${state.status}</div>
      </section>
    </section>
    <footer class="credit">
      By create&#xff1a;<a class="credit-link" href="mailto:31665431@qq.com">zhailei</a>
    </footer>
  </main>
`;

const pickFileButton = document.querySelector("#pick-file");
const pickOutputButton = document.querySelector("#pick-output");
const startButton = document.querySelector("#start");
const watermarkInput = document.querySelector("#watermark-input");
const filePathNode = document.querySelector("#file-path");
const outputPathNode = document.querySelector("#output-path");
const progressTextNode = document.querySelector("#progress-text");
const progressBarNode = document.querySelector("#progress-bar");
const pageTextNode = document.querySelector("#page-text");
const statusNode = document.querySelector("#status");

function defaultOutputPath(inputPath) {
  if (!inputPath.toLowerCase().endsWith(".pdf")) {
    return `${inputPath}_watermarked.pdf`;
  }

  return inputPath.replace(/\.pdf$/i, "_watermarked.pdf");
}

function setState(patch) {
  Object.assign(state, patch);
  render();
}

function render() {
  filePathNode.textContent = state.inputPath || "No file selected";
  filePathNode.classList.toggle("empty", !state.inputPath);

  outputPathNode.textContent = state.outputPath || "No output path selected";
  outputPathNode.classList.toggle("empty", !state.outputPath);

  progressTextNode.textContent = `${state.progress}%`;
  progressBarNode.style.width = `${state.progress}%`;
  pageTextNode.textContent = `${state.current} / ${state.total}`;
  statusNode.textContent = state.status;

  watermarkInput.disabled = state.busy;
  pickFileButton.disabled = state.busy;
  pickOutputButton.disabled = state.busy;
  startButton.disabled = state.busy;
  startButton.textContent = state.busy ? "Processing..." : "Add Watermark";
}

async function chooseOutputPath() {
  const filePath = await save({
    title: "Save Watermarked PDF",
    defaultPath: state.inputPath ? defaultOutputPath(state.inputPath) : undefined,
    filters: [{ name: "PDF Document", extensions: ["pdf"] }],
  });

  if (typeof filePath === "string") {
    setState({ outputPath: filePath });
  }
}

pickFileButton.addEventListener("click", async () => {
  const selected = await open({
    title: "Choose PDF File",
    multiple: false,
    directory: false,
    filters: [{ name: "PDF Document", extensions: ["pdf"] }],
  });

  if (typeof selected === "string") {
    setState({
      inputPath: selected,
      outputPath: state.outputPath || defaultOutputPath(selected),
      status: "File selected. Ready to process.",
    });
  }
});

pickOutputButton.addEventListener("click", async () => {
  await chooseOutputPath();
});

watermarkInput.addEventListener("input", (event) => {
  setState({ watermarkText: event.target.value });
});

startButton.addEventListener("click", async () => {
  const watermarkText = state.watermarkText.trim();

  if (!state.inputPath) {
    setState({ status: "Please choose a PDF file first." });
    return;
  }

  if (!watermarkText) {
    setState({ status: "Please enter watermark text." });
    return;
  }

  if (!state.outputPath) {
    await chooseOutputPath();
    if (!state.outputPath) {
      setState({ status: "An output path is required to continue." });
      return;
    }
  }

  setState({
    busy: true,
    progress: 0,
    current: 0,
    total: 0,
    status: "Generating watermarked PDF...",
  });

  try {
    await invoke("add_pdf_watermark", {
      inputPath: state.inputPath,
      outputPath: state.outputPath,
      watermarkText,
    });

    setState({
      busy: false,
      progress: 100,
      status: `Done. The output file was saved to ${state.outputPath}`,
    });
  } catch (error) {
    setState({
      busy: false,
      status: `Processing failed: ${String(error)}`,
    });
  }
});

listen("watermark://progress", (event) => {
  const payload = event.payload;

  if (!payload || typeof payload !== "object") {
    return;
  }

  const total = Number(payload.total ?? 0);
  const current = Number(payload.current ?? 0);
  const progress = total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0;

  setState({
    total,
    current,
    progress,
    status: total > 0 ? `Processing page ${current} of ${total}...` : state.status,
  });
}).catch((error) => {
  setState({ status: `Unable to listen for progress events: ${String(error)}` });
});

render();
