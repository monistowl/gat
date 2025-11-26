// Persona content data
const audienceContent = {
  students: {
    subtitle: "Open Source & Academic Community",
    slogan: "Your thesis deserves better than MATLAB.",
    description:
      "GAT is what happens when you rewrite half the power-systems curriculum in Rust and ship it as a single executable. Runs everywhere, solves fast, zero ceremony.",
    deepDive:
      "GAT is the toolkit every power-systems student always wished existed: reproducible, dependency-free, lightning-fast, and fully hackable. Run AC-OPF, N-1, SE, contingency analysis, and reliability metrics with a local binary smaller than most homework PDFs. Export to Arrow/Parquet, visualize via a built-in Ratatui dashboard, and tear apart the Rust code to see how everything works. Academic use is free — and contributions shape the future of open grid tooling.",
    vignette:
      "You sit down to reproduce a paper. The Python environment breaks. The MATLAB license expired. You remember GAT exists. One command later, you're running AC-OPF and generating Parquet files that drop straight into Polars. Suddenly your entire research pipeline works again.",
    features: ["No Conda, No Pain", "Single Binary", "Free for Academic Use"],
    image:
      "https://images.unsplash.com/photo-1555066931-4365d14bab8c?auto=format&fit=crop&q=80&w=1000",
    title: "Grad Students & Coders"
  },
  startups: {
    subtitle: "Early Customers",
    slogan: "Commercial-grade solvers without commercial-grade invoices.",
    description:
      "GAT hits the sweet spot: the performance of commercial engines, the simplicity of a single binary, and the cost of a small open-source license.",
    deepDive:
      "GAT gives small research teams and lean startups the computational power usually reserved for utilities with huge software budgets. Industrial AC-OPF, DC/AC PF, N-1 screening, piecewise generator costs, and reliability metrics come built-in. When the browser-based Studio launches, you'll get interactive notebooks and scenario builders with zero cloud lock-in. Lock in early licenses and upgrade later as your workflow scales.",
    vignette:
      "You're a two-person energy startup. You download GAT, run OPF on a 12k-bus model, and it solves in milliseconds. You realize you no longer need a $15k seat license to survive.",
    features: ["Industrial AC-OPF", "Parquet/Arrow Native", "Boutique Pricing"],
    image:
      "https://images.unsplash.com/photo-1551288049-bebda4e38f71?auto=format&fit=crop&q=80&w=1000",
    title: "Startups & Analysts"
  },
  enterprise: {
    subtitle: "Enterprise / Air-Gapped / Federated",
    slogan: "Fast enough for planning. Transparent enough for regulation.",
    description:
      "A single static binary means no patch sprawl, no hidden dependencies, and a smaller attack surface. Bring your own firewall.",
    deepDive:
      "GAT is engineered for operators and agencies that need high-speed, fully auditable analytics without ever exposing operational data. Deploy it behind your firewall, run it in an isolated enclave, or orchestrate it across federated regions. Production-grade AC-OPF, contingency analysis, LOLE/EUE, VVO, and deliverability scoring come built-in. A stable CLI, JSON schemas, and deterministic solvers ensure regulatory confidence.",
    vignette:
      "A regional operator needs N-1 scoring. Running it in Python takes hours. GAT finishes in under a minute and leaves an audit trail regulators actually understand.",
    features: ["Air-Gap Ready", "Auditable Outputs", "Deterministic Solvers"],
    image:
      "https://images.unsplash.com/photo-1473341304170-971dccb5ac1e?auto=format&fit=crop&q=80&w=1000",
    title: "Operators & Regulators"
  },
  ai: {
    subtitle: "LLM + Real Physics Backbone",
    slogan: "Give your agent a grid simulator, not a guess.",
    description:
      "LLMs hallucinate power flows. GAT computes them — deterministically, quickly, and with full provenance. The missing physics engine for Energy AI.",
    deepDive:
      "AI-native energy apps need real-time physics, not heuristics. GAT is the first open-core toolkit that gives LLM agents a deterministic, sub-50-ms AC-OPF engine they can call like any other tool. MCP endpoints, structured schemas, and reproducible CLI commands ensure perfect traceability. If you're building autonomous energy planning or 'Perplexity for the grid,' GAT is the foundation.",
    vignette:
      "One GAT tool call later, it returns flows, voltages, binding constraints, marginal prices, and a full audit log — all in JSON your orchestrator understands.",
    features: ["<50ms Response", "MCP Server Ready", "Structured JSON"],
    image:
      "https://images.unsplash.com/photo-1620712943543-bcc4688e7485?auto=format&fit=crop&q=80&w=1000",
    title: "AI & Agent Builders"
  }
};

function setActivePersona(key) {
  const data = audienceContent[key];
  if (!data) return;

  const subtitleEl = document.getElementById("personaSubtitle");
  const sloganEl = document.getElementById("personaSlogan");
  const descEl = document.getElementById("personaDescription");
  const vignetteEl = document.getElementById("personaVignette");
  const deepDiveEl = document.getElementById("personaDeepDive");
  const featuresEl = document.getElementById("personaFeatures");
  const imgMain = document.getElementById("personaMainImage");
  const imgBg = document.getElementById("personaBgImage");

  subtitleEl.textContent = data.subtitle;
  sloganEl.textContent = data.slogan;
  descEl.textContent = data.description;
  vignetteEl.textContent = `"${data.vignette}"`;
  deepDiveEl.textContent = data.deepDive;

  // Features badges
  featuresEl.innerHTML = "";
  data.features.forEach(function (feature) {
    const div = document.createElement("div");
    div.className =
      "flex items-center gap-2 text-slate-300 text-xs font-medium bg-slate-900 px-3 py-1 rounded-full border border-slate-800";
    const icon = document.createElement("span");
    icon.className = "text-orange-500 text-xs";
    icon.textContent = "✓";
    const text = document.createElement("span");
    text.textContent = feature;
    div.appendChild(icon);
    div.appendChild(text);
    featuresEl.appendChild(div);
  });

  // Update images
  imgMain.src = data.image;
  imgMain.alt = data.title;
  imgMain.classList.remove("animate-fadeIn");
  void imgMain.offsetWidth; // trigger reflow for animation restart
  imgMain.classList.add("animate-fadeIn");

  imgBg.src = data.image;
  imgBg.alt = data.title;

  // Tab button active state
  const tabs = document.querySelectorAll(".persona-tab");
  tabs.forEach(function (btn) {
    const isActive = btn.getAttribute("data-tab") === key;
    btn.className =
      "w-full text-left px-6 py-4 rounded-lg transition-all border persona-tab " +
      (isActive
        ? "bg-slate-800 border-orange-500/50 text-white shadow-lg shadow-orange-900/10"
        : "border-transparent text-slate-500 hover:bg-slate-800/50 hover:text-slate-300");
  });
}

document.addEventListener("DOMContentLoaded", function () {
  // Copy button
  const copyButton = document.getElementById("copyButton");
  const copyLabel = document.getElementById("copyButtonLabel");
  if (copyButton && copyLabel) {
    copyButton.addEventListener("click", function () {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText("cargo install gat-cli").then(
          function () {
            copyLabel.textContent = "Copied!";
            setTimeout(function () {
              copyLabel.textContent = "Copy";
            }, 2000);
          },
          function () {
            copyLabel.textContent = "Error";
            setTimeout(function () {
              copyLabel.textContent = "Copy";
            }, 2000);
          }
        );
      } else {
        copyLabel.textContent = "Not supported";
        setTimeout(function () {
          copyLabel.textContent = "Copy";
        }, 2000);
      }
    });
  }

  // Persona tabs
  const tabs = document.querySelectorAll(".persona-tab");
  tabs.forEach(function (btn) {
    btn.addEventListener("click", function () {
      const key = btn.getAttribute("data-tab");
      setActivePersona(key);
    });
  });

  // Initialize features for default persona
  setActivePersona("students");

  // Notebook WASM preview interactions
  const runButton = document.getElementById("notebookRunButton");
  const statusBadge = document.getElementById("notebookStatusBadge");
  const outputEl = document.getElementById("notebookOutput");
  const timelineEl = document.getElementById("notebookTimeline");
  const drawer = document.getElementById("notebookDrawer");
  const drawerToggle = document.getElementById("notebookDrawerToggle");
  const drawerClose = document.getElementById("notebookDrawerClose");
  const drawerList = document.getElementById("notebookList");
  const selectedTitle = document.getElementById("notebookSelectedTitle");
  const selectedDesc = document.getElementById("notebookSelectedDesc");
  const selectedFiles = document.getElementById("notebookSelectedFiles");
  const selectedCommand = document.getElementById("notebookSelectedCommand");
  const wasmEmbedStatus = document.getElementById("wasmEmbedStatus");
  const wasmEmbedLog = document.getElementById("wasmEmbedLog");
  const wasmSupportStatus = document.getElementById("wasmSupportStatus");
  const wasmEmbedButton = document.getElementById("wasmEmbedButton");

  const wasmBundlePaths = {
    wasm: "/wasm/gat-notebook/notebook_bg.wasm",
    js: "/wasm/gat-notebook/notebook.js",
    readme: "/wasm/gat-notebook/README.md"
  };

  const wasmBridge = {
    ready: false,
    module: null,
    exports: null,
    runExport: null
  };

  const badgeBase = "px-3 py-1 text-xs rounded-full border";

  const setStatusBadge = (text, palette) => {
    if (!statusBadge) return;
    statusBadge.textContent = text;
    statusBadge.className = `${badgeBase} ${palette}`;
  };

  const markWasmRuntimeReady = () => {
    if (!statusBadge) return;
    setStatusBadge(
      "WASM runtime idle • ready for notebook cells",
      "bg-emerald-500/15 text-emerald-300 border-emerald-400/40"
    );
    if (runButton && activeNotebook) {
      runButton.textContent = `Run ${activeNotebook.title} in WASM`;
    }
  };

  const updateWasmStatus = (text, variant = "muted") => {
    if (!wasmEmbedStatus) return;
    const base = "text-xs font-mono rounded-lg px-3 py-2 border";
    const palettes = {
      muted: `${base} bg-slate-950 border-slate-800 text-slate-300`,
      loading: `${base} bg-amber-500/10 border-amber-400/40 text-amber-100`,
      success: `${base} bg-emerald-500/10 border-emerald-400/40 text-emerald-200`,
      error: `${base} bg-rose-500/10 border-rose-400/40 text-rose-200`
    };

    wasmEmbedStatus.className = palettes[variant] || palettes.muted;
    wasmEmbedStatus.textContent = text;
  };

  const appendWasmLog = (text, tone = "info") => {
    if (!wasmEmbedLog) return;
    const row = document.createElement("div");
    const tones = {
      info: "text-slate-200",
      warn: "text-amber-200",
      success: "text-emerald-200",
      error: "text-rose-200"
    };
    row.className = tones[tone] || tones.info;
    row.textContent = text;
    wasmEmbedLog.appendChild(row);
  };

  const detectWasmSupport = () => {
    if (!wasmEmbedStatus || !wasmSupportStatus) return;
    if (typeof WebAssembly === "undefined") {
      updateWasmStatus("WebAssembly is not available in this browser.", "error");
      wasmSupportStatus.textContent = "Please try a modern browser (Chrome, Edge, Firefox, Safari) with WASM enabled.";
      appendWasmLog("WASM not supported by this runtime.", "error");
      return false;
    }

    if (typeof WebAssembly.instantiateStreaming !== "function") {
      appendWasmLog("Streaming instantiation not available; falling back to ArrayBuffer path.", "warn");
    }

    appendWasmLog("WASM supported. Looking for gat-notebook artifacts…", "success");
    return true;
  };

  const loadWasmBundle = async () => {
    if (wasmEmbedLog) {
      wasmEmbedLog.innerHTML = "";
    }

    if (!detectWasmSupport()) return;

    updateWasmStatus("Fetching wasm bundle…", "loading");
    appendWasmLog(`GET ${wasmBundlePaths.wasm}`, "info");

    try {
      const response = await fetch(wasmBundlePaths.wasm, { cache: "no-store" });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status} for ${wasmBundlePaths.wasm}`);
      }

      const bytes = await response.arrayBuffer();
      const module = await WebAssembly.instantiate(bytes, {});
      const exported = Object.keys(module.instance?.exports || {});
      const runHook = ["run_demo", "run_cell", "main", "start"].find(
        (name) => typeof module.instance?.exports?.[name] === "function"
      );

      wasmBridge.ready = true;
      wasmBridge.module = module;
      wasmBridge.exports = module.instance?.exports || {};
      wasmBridge.runExport = runHook || null;

      updateWasmStatus("Loaded wasm binary — exports hydrated.", "success");
      appendWasmLog(
        `Exports: ${exported.length ? exported.join(", ") : "(none)"}`,
        "success"
      );
      appendWasmLog(
        runHook
          ? `Notebook bridge found export \'${runHook}\'. Wire run button to this function.`
          : "No obvious run export found. Wire a JS shim to call into the notebook runtime.",
        runHook ? "success" : "warn"
      );

      markWasmRuntimeReady();

      try {
        appendWasmLog(`Loading JS shim ${wasmBundlePaths.js}…`, "info");
        await import(`${wasmBundlePaths.js}?t=${Date.now()}`);
        appendWasmLog("JS shim imported (stub). Wire real bindings to run cells.", "success");
      } catch (err) {
        appendWasmLog(`JS shim missing: ${err?.message || err}`, "warn");
      }
    } catch (error) {
      updateWasmStatus("Failed to load wasm bundle.", "error");
      appendWasmLog(
        typeof error === "string" ? error : error?.message || "Unknown wasm load error",
        "error"
      );
      appendWasmLog(
        "Drop wasm-pack outputs into website/static/wasm/gat-notebook to replace this stub.",
        "warn"
      );
      appendWasmLog("Offline build note: missing wasm32 std (see README).", "warn");
    }
  };

  if (wasmEmbedButton) {
    wasmEmbedButton.addEventListener("click", loadWasmBundle);
  }

  detectWasmSupport();

  const renderOutput = (lines) => {
    if (!outputEl) return;
    outputEl.innerHTML = "";
    lines.forEach(function (line) {
      const div = document.createElement("div");
      div.textContent = line;
      outputEl.appendChild(div);
    });
  };

  const notebookSamples = [
    {
      slug: "ac-opf",
      title: "AC-OPF quickstart",
      desc: "IEEE 14-bus • CLI parity",
      files: "case14.arrow · limits.csv",
      command: "gat opf ac case14.arrow --limits limits.csv --out opf.parquet",
      outputs: ["flows.parquet → 14 branches", "binding constraints: 2", "objective: 13,284.22"]
    },
    {
      slug: "ac-pf",
      title: "AC power flow scan",
      desc: "IEEE 118-bus • voltage band",
      files: "ieee118.arrow",
      command: "gat pf ac ieee118.arrow --out flows.parquet",
      outputs: ["voltages: 0.95–1.05 p.u.", "branches: 186", "converged in 21 iters"]
    },
    {
      slug: "nminus1",
      title: "N-1 thermal screening",
      desc: "200 contingencies • DC PF",
      files: "case300.arrow · contingencies.yaml",
      command: "gat nminus1 dc case300.arrow --spec contingencies.yaml",
      outputs: ["200 outages enumerated", "violations: 3", "worst: branch 121"]
    }
  ];

  let activeNotebook = notebookSamples[0];

  const setActiveNotebook = (sample) => {
    activeNotebook = sample;
    if (selectedTitle && selectedDesc && selectedFiles && selectedCommand) {
      selectedTitle.textContent = sample.title;
      selectedDesc.textContent = sample.desc;
      selectedFiles.textContent = sample.files;
      selectedCommand.textContent = sample.command;
    }

    if (runButton) {
      runButton.textContent = wasmBridge.ready
        ? `Run ${sample.title} in WASM`
        : `Run ${sample.title} in browser`;
    }

    renderOutput(sample.outputs);
  };

  const renderDrawerList = () => {
    if (!drawerList) return;
    drawerList.innerHTML = "";
    notebookSamples.forEach(function (sample) {
      const row = document.createElement("button");
      row.type = "button";
      row.className =
        "w-full text-left px-4 py-3 hover:bg-slate-900/60 transition-colors" +
        (activeNotebook.slug === sample.slug ? " bg-slate-900/70" : "");
      const title = document.createElement("div");
      title.className = "text-sm text-white font-semibold";
      title.textContent = sample.title;
      const desc = document.createElement("div");
      desc.className = "text-[11px] text-slate-400";
      desc.textContent = sample.desc;
      const meta = document.createElement("div");
      meta.className = "text-[11px] text-orange-300 font-mono mt-1";
      meta.textContent = sample.command;
      row.appendChild(title);
      row.appendChild(desc);
      row.appendChild(meta);
      row.addEventListener("click", function () {
        setActiveNotebook(sample);
        renderDrawerList();
        drawer?.classList.add("hidden");
      });
      drawerList.appendChild(row);
    });
  };

  const toggleDrawer = (open) => {
    if (!drawer) return;
    const shouldOpen = open !== undefined ? open : drawer.classList.contains("hidden");
    if (shouldOpen) {
      drawer.classList.remove("hidden");
    } else {
      drawer.classList.add("hidden");
    }
  };

  if (drawerToggle && drawer) {
    drawerToggle.addEventListener("click", function () {
      toggleDrawer();
    });
  }

  if (drawerClose && drawer) {
    drawerClose.addEventListener("click", function () {
      toggleDrawer(false);
    });
  }

  if (drawerList) {
    renderDrawerList();
  }

  setActiveNotebook(activeNotebook);

  if (runButton && statusBadge && outputEl && timelineEl) {
    const steps = [
      {
        status: "Compiling WASM bundle…",
        badge: "bg-amber-500/15 text-amber-200 border-amber-400/40",
        output: [
          "wasm-pack build --target web",
          "optimizing: -O3 • tree-shaking",
          "emscripten: shared runtime ready"
        ],
        timeline: "Compile wasm build"
      },
      {
        status: "Hydrating notebook shell…",
        badge: "bg-blue-500/15 text-blue-200 border-blue-400/40",
        output: [
          "registering service worker",
          "mounting file dropzones",
          "connecting Arrow viewer"
        ],
        timeline: "Hydrate notebook shell"
      },
      {
        status: "AC-OPF solving (browser)…",
        badge: "bg-orange-500/15 text-orange-200 border-orange-400/40",
        output: [
          "ac_opf(case14) → 48ms",
          "binding constraints: 2",
          "dual residual: 9.6e-7"
        ],
        timeline: "Solve ac_opf"
      },
      {
        status: "Streaming Parquet + charts",
        badge: "bg-emerald-500/15 text-emerald-200 border-emerald-400/40",
        output: [
          "flows.parquet (14 rows)",
          "voltages.parquet (14 rows)",
          "objective: 13,284.22"
        ],
        timeline: "Stream outputs"
      }
    ];

    const renderTimeline = (activeIndex = -1) => {
      timelineEl.innerHTML = "";
      steps.forEach(function (step, idx) {
        const row = document.createElement("div");
        row.className = "flex items-center gap-2";
        const dot = document.createElement("span");
        dot.className =
          "w-2 h-2 rounded-full " +
          (idx < activeIndex
            ? "bg-emerald-400"
            : idx === activeIndex
            ? "bg-orange-400 animate-pulse"
            : "bg-slate-700");
        const label = document.createElement("span");
        label.className = "font-mono " + (idx <= activeIndex ? "text-slate-100" : "text-slate-400");
        label.textContent = step.timeline;
        row.appendChild(dot);
        row.appendChild(label);
        timelineEl.appendChild(row);
      });
    };

    renderTimeline();

    runButton.addEventListener("click", function () {
      runButton.disabled = true;
      runButton.textContent = "Running…";

      if (wasmBridge.ready) {
        appendWasmLog(
          `Routing demo '${activeNotebook.slug}' through wasm runtime${
            wasmBridge.runExport ? ` via ${wasmBridge.runExport}()` : " (shim pending)"
          }.`,
          wasmBridge.runExport ? "success" : "warn"
        );
      } else {
        appendWasmLog(
          "WASM runtime not loaded yet. Falling back to front-end mock run.",
          "warn"
        );
      }

      let stepIndex = 0;

      const advance = () => {
        const step = steps[stepIndex];
        setStatusBadge(step.status, step.badge);
        renderOutput(step.output);
        renderTimeline(stepIndex);

        stepIndex += 1;

        if (stepIndex < steps.length) {
          setTimeout(advance, 900);
        } else {
          setTimeout(function () {
            setStatusBadge(
              wasmBridge.ready
                ? "WASM runtime idle • ready for next cell"
                : "Complete • ready for next run",
              "bg-emerald-500/15 text-emerald-300 border-emerald-400/40"
            );
            runButton.disabled = false;
            runButton.textContent = wasmBridge.ready
              ? `Run ${activeNotebook.title} in WASM`
              : `Run ${activeNotebook.title} in browser`;
            renderTimeline(steps.length);
          }, 750);
        }
      };

      advance();
    });
  }
});
