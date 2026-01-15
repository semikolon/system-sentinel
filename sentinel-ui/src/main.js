const { listen } = window.__TAURI__.event;

// UI Elements
const memValue = document.getElementById('mem-value');
const memBar = document.getElementById('mem-bar');
const memDetail = document.getElementById('mem-detail');

const swapValue = document.getElementById('swap-value');
const swapBar = document.getElementById('swap-bar');
const swapDetail = document.getElementById('swap-detail');

const growthValue = document.getElementById('growth-value');
const processItems = document.getElementById('process-items');
const statusDot = document.getElementById('status-dot');

// Helper to format bytes
function formatGB(bytes) {
  return (bytes / 1024 / 1024 / 1024).toFixed(1);
}

// Listen for metrics updates from the Rust backend
listen('metrics-update', (event) => {
  const metrics = event.payload;

  // Memory
  memValue.textContent = `${Math.round(metrics.memory_percent)}%`;
  memBar.style.width = `${metrics.memory_percent}%`;
  memDetail.textContent = `${formatGB(metrics.memory_used)} / ${formatGB(metrics.memory_total)} GB`;

  // Swap
  swapValue.textContent = `${Math.round(metrics.swap_percent)}%`;
  swapBar.style.width = `${metrics.swap_percent}%`;
  swapDetail.textContent = `${formatGB(metrics.swap_used)} / ${formatGB(metrics.swap_total)} GB`;

  // Growth
  if (metrics.memory_growth_rate !== null) {
    growthValue.textContent = `${metrics.memory_growth_rate.toFixed(2)} GB/h`;
    growthValue.style.color = metrics.memory_growth_rate > 1.0 ? '#fbbf24' : '#f8fafc';
  } else {
    growthValue.textContent = 'Calculating...';
  }

  // Processes
  renderProcesses(metrics.top_processes, metrics.aggregated_processes);

  // Status Dot
  updateStatus(metrics);
});

function renderProcesses(top, aggregated) {
  // Combine and sort by memory
  const all = [...top, ...aggregated]
    .filter(p => p.memory_mb > 100) // Only show significant processes
    .sort((a, b) => b.memory_mb - a.memory_mb)
    .slice(0, 6);

  processItems.innerHTML = all.map(p => {
    // Basic name resolution for the UI if needed, 
    // though the daemon already does it for us now!
    const name = p.name.replace(' (Group)', '');
    const gb = (p.memory_mb / 1024).toFixed(1);
    
    return `
      <div class="process-item">
        <div class="process-name">${name}</div>
        <div class="process-meta">
          <div class="process-memory">${gb} GB</div>
          <div class="process-cpu">${Math.round(p.cpu_usage)}%</div>
        </div>
      </div>
    `;
  }).join('');
}

function updateStatus(metrics) {
  let color = '#10b981'; // Green
  let glow = 'rgba(16, 185, 129, 0.5)';

  if (metrics.memory_percent > 90 || metrics.swap_percent > 40) {
    color = '#ef4444'; // Red
    glow = 'rgba(239, 68, 68, 0.5)';
  } else if (metrics.memory_percent > 80 || metrics.swap_percent > 10) {
    color = '#f59e0b'; // Yellow
    glow = 'rgba(245, 158, 11, 0.5)';
  }

  statusDot.style.background = color;
  statusDot.style.boxShadow = `0 0 10px ${glow}`;
}
