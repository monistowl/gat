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
});
