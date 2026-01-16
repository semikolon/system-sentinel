const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

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

// ========== CHAT FUNCTIONALITY ==========

const chatMessages = document.getElementById('chat-messages');
const chatInput = document.getElementById('chat-input');
const chatSend = document.getElementById('chat-send');
const chatStatus = document.getElementById('chat-status');
const actionCard = document.getElementById('action-card');
const actionRisk = document.getElementById('action-risk');
const actionDescription = document.getElementById('action-description');
const actionCancel = document.getElementById('action-cancel');
const actionConfirm = document.getElementById('action-confirm');

let currentMetrics = null;
let pendingAction = null;

// Store latest metrics for context
listen('metrics-update', (event) => {
  currentMetrics = event.payload;
});

// Listen for Claude streaming responses
listen('claude-stream', (event) => {
  const lastMessage = chatMessages.querySelector('.chat-message.assistant:last-child .message-content');
  if (lastMessage && lastMessage.dataset.streaming === 'true') {
    lastMessage.textContent += event.payload;
    chatMessages.scrollTop = chatMessages.scrollHeight;
  }
});

listen('claude-done', (event) => {
  chatStatus.textContent = '';
  chatStatus.classList.remove('thinking');
  chatSend.disabled = false;

  const lastMessage = chatMessages.querySelector('.chat-message.assistant:last-child .message-content');
  if (lastMessage) {
    lastMessage.dataset.streaming = 'false';
  }

  // Check for action suggestions in the response
  if (event.payload && event.payload.action) {
    showActionCard(event.payload.action);
  }
});

listen('claude-error', (event) => {
  chatStatus.textContent = 'Error';
  chatStatus.classList.remove('thinking');
  chatSend.disabled = false;
  addMessage('assistant', `Error: ${event.payload}`);
});

function addMessage(role, content) {
  const div = document.createElement('div');
  div.className = `chat-message ${role}`;
  div.innerHTML = `<div class="message-content" ${role === 'assistant' ? 'data-streaming="true"' : ''}>${content}</div>`;
  chatMessages.appendChild(div);
  chatMessages.scrollTop = chatMessages.scrollHeight;
}

async function sendMessage() {
  const text = chatInput.value.trim();
  if (!text) return;

  addMessage('user', text);
  chatInput.value = '';
  chatSend.disabled = true;
  chatStatus.textContent = 'Thinking...';
  chatStatus.classList.add('thinking');

  // Add empty assistant message for streaming
  addMessage('assistant', '');

  try {
    await invoke('submit_query', {
      prompt: text,
      metricsJson: currentMetrics ? JSON.stringify(currentMetrics) : null
    });
  } catch (e) {
    chatStatus.textContent = 'Error';
    chatStatus.classList.remove('thinking');
    chatSend.disabled = false;
    addMessage('assistant', `Failed to send: ${e}`);
  }
}

function showActionCard(action) {
  pendingAction = action;
  actionDescription.textContent = action.description;

  if (action.risk === 'high') {
    actionCard.classList.add('high-risk');
    actionRisk.textContent = '🔴 High Risk';
  } else {
    actionCard.classList.remove('high-risk');
    actionRisk.textContent = '⚠️ Moderate Risk';
  }

  actionCard.classList.remove('hidden');
}

function hideActionCard() {
  actionCard.classList.add('hidden');
  pendingAction = null;
}

async function confirmAction() {
  if (!pendingAction) return;

  try {
    await invoke('execute_action', { action: pendingAction });
    addMessage('assistant', `✓ Action executed: ${pendingAction.description}`);
  } catch (e) {
    addMessage('assistant', `✗ Action failed: ${e}`);
  }

  hideActionCard();
}

// Event listeners
chatSend.addEventListener('click', sendMessage);
chatInput.addEventListener('keypress', (e) => {
  if (e.key === 'Enter') sendMessage();
});
actionCancel.addEventListener('click', hideActionCard);
actionConfirm.addEventListener('click', confirmAction);
