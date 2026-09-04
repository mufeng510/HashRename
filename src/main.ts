import { invoke } from "@tauri-apps/api/core";

interface ProcessingResult {
  scanned_count: number;
  duplicate_count: number;
  trashed_count: number;
  renamed_count: number;
  failed_count: number;
  elapsed_secs: number;
  errors: Array<{
    path: string;
    operation: string;
    reason: string;
  }>;
}

function $(id: string): HTMLElement {
  return document.getElementById(id) as HTMLElement;
}

function showView(id: string) {
  document.querySelectorAll(".view").forEach((v) => v.classList.add("hidden"));
  $(id).classList.remove("hidden");
}

function updateProgress(
  scanned: number,
  duplicate: number,
  kept: number,
  stage: string,
  file: string,
  percent: number
) {
  $("scanned-count").textContent = scanned.toString();
  $("duplicate-count").textContent = duplicate.toString();
  $("kept-count").textContent = kept.toString();
  $("current-stage").textContent = stage;
  $("current-file").textContent = file || "-";
  $("progress-bar").style.width = `${percent}%`;
  $("progress-percent").textContent = `${Math.round(percent)}%`;
}

function showResult(result: ProcessingResult) {
  $("result-scanned").textContent = result.scanned_count.toString();
  $("result-duplicates").textContent = result.duplicate_count.toString();
  $("result-trashed").textContent = result.trashed_count.toString();
  $("result-renamed").textContent = result.renamed_count.toString();
  $("result-failed").textContent = result.failed_count.toString();
  $("result-time").textContent = `${result.elapsed_secs.toFixed(1)}s`;

  if (result.failed_count > 0) {
    $("result-title").textContent = "Processing Completed with Errors";
    $("result-title").style.color = "var(--warning)";
  }

  if (result.errors.length > 0) {
    $("errors-section").classList.remove("hidden");
    const list = $("errors-list");
    list.innerHTML = result.errors
      .map(
        (e) =>
          `<div class="error-entry">${escapeHtml(e.path)} - ${escapeHtml(e.operation)}: ${escapeHtml(e.reason)}</div>`
      )
      .join("");
  }

  showView("result-view");
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

async function processDirectory(dir: string) {
  showView("processing-view");
  $("directory-path").textContent = dir;
  updateProgress(0, 0, 0, "Starting...", "", 0);

  // Simulate progress since Tauri invoke is synchronous per-call
  // We poll progress via a simple interval
  let progressTimer: ReturnType<typeof setInterval> | null = null;
  let fakeStage = 0;

  const stages = [
    { stage: "Scanning files...", pct: 5 },
    { stage: "Computing MD5 hashes...", pct: 30 },
    { stage: "Finding duplicates...", pct: 50 },
    { stage: "Moving duplicates to trash...", pct: 65 },
    { stage: "Planning renames...", pct: 80 },
    { stage: "Renaming files...", pct: 90 },
  ];

  progressTimer = setInterval(() => {
    if (fakeStage < stages.length) {
      const s = stages[fakeStage];
      updateProgress(
        parseInt($("scanned-count").textContent || "0"),
        parseInt($("duplicate-count").textContent || "0"),
        parseInt($("kept-count").textContent || "0"),
        s.stage,
        "",
        s.pct
      );
      fakeStage++;
    }
  }, 400);

  try {
    const result: ProcessingResult = await invoke("process_directory", {
      directory: dir,
      verbose: false,
    });

    if (progressTimer) clearInterval(progressTimer);

    updateProgress(
      result.scanned_count,
      result.duplicate_count,
      result.scanned_count - result.duplicate_count,
      "Done",
      "",
      100
    );

    // Brief pause to show 100%
    await new Promise((r) => setTimeout(r, 300));

    showResult(result);
  } catch (error) {
    if (progressTimer) clearInterval(progressTimer);
    $("error-message").textContent = String(error);
    showView("error-view");
  }
}

document.addEventListener("DOMContentLoaded", () => {
  $("btn-close")?.addEventListener("click", () => {
    window.close();
  });

  $("btn-close-error")?.addEventListener("click", () => {
    window.close();
  });

  // Check if a directory was passed via CLI (Tauri arg)
  // For now, check URL params or show a prompt
  const params = new URLSearchParams(window.location.search);
  const dir = params.get("dir");

  if (dir) {
    processDirectory(dir);
  }
  // If no dir param, the window stays on the default view
  // The CLI handler or context menu will pass the directory
});
