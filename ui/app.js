const { invoke } = window.__TAURI__.core;

let currentModel = '';
let isSending = false;

function el(id) { return document.getElementById(id); }

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function formatMarkdown(text) {
  let html = escapeHtml(text);
  html = html.replace(/```([\w]*)\n?([\s\S]*?)```/g, '<pre><code>$2</code></pre>');
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  html = html.replace(/^(#{1,6})\s+(.+)$/gm, (_, hashes, content) => {
    const level = hashes.length;
    return `<h${level}>${content}</h${level}>`;
  });
  html = html.replace(/^[-*]\s+(.+)$/gm, '<li>$1</li>');
  html = html.replace(/(?:<li>.+<\/li>\n?)+/g, m => `<ul>${m}</ul>`);
  html = html.replace(/\n{2,}/g, '</p><p>');
  html = '<p>' + html + '</p>';
  html = html.replace(/<p><\/p>/g, '');
  html = html.replace(/<\/pre><\/p>/g, '</pre>').replace(/<p><pre>/g, '<pre>');
  html = html.replace(/<\/ul><\/p>/g, '</ul>').replace(/<p><ul>/g, '<ul>');
  return html;
}

async function loadModels() {
  try {
    const models = await invoke('get_models');
    const select = el('model-select');
    select.innerHTML = '';
    if (models.length === 0) {
      select.innerHTML = '<option>No models found</option>';
      el('model-status').textContent = 'Install models via Ollama CLI';
      el('model-status').className = 'status error';
      return;
    }
    models.forEach(m => {
      const opt = document.createElement('option');
      opt.value = m.name;
      opt.textContent = m.name + (m.parameter_size ? ' (' + m.parameter_size + ')' : '');
      select.appendChild(opt);
    });
    currentModel = models[0].name;
    el('model-status').textContent = models.length + ' model(s) available';
    el('model-status').className = 'status connected';
  } catch (e) {
    el('model-select').innerHTML = '<option>Ollama not running</option>';
    el('model-status').textContent = 'Start Ollama and refresh';
    el('model-status').className = 'status error';
  }
}

async function loadFolders() {
  try {
    const folders = await invoke('list_folders');
    const list = el('folder-list');
    list.innerHTML = '';
    folders.forEach(f => {
      const li = document.createElement('li');
      li.innerHTML = '<span class="folder-name">📁 ' + escapeHtml(f.name) + '</span>' +
        '<div><span class="folder-count">' + f.file_count + ' files</span>' +
        '<button class="remove-folder" data-path="' + escapeHtml(f.path) + '">&times;</button></div>';
      list.appendChild(li);
    });
    document.querySelectorAll('.remove-folder').forEach(btn => {
      btn.addEventListener('click', async () => {
        await invoke('remove_folder', { path: btn.dataset.path });
        loadFolders();
      });
    });
  } catch (e) {
    console.error('Failed to load folders:', e);
  }
}

async function loadTools() {
  try {
    const tools = await invoke('get_tool_status');
    const list = el('tool-list');
    list.innerHTML = '';
    tools.forEach(t => {
      const li = document.createElement('li');
      li.innerHTML = '<span class="tool-name">' + escapeHtml(t.display_name) + '</span>' +
        '<button class="tool-toggle ' + (t.enabled ? 'on' : '') + '" data-name="' + t.name + '" ' +
        (!t.available ? 'disabled ' : '') + 'title="' + escapeHtml(t.description) + '"></button>';
      list.appendChild(li);
    });
    document.querySelectorAll('.tool-toggle:not(:disabled)').forEach(btn => {
      btn.addEventListener('click', async () => {
        const enabled = !btn.classList.contains('on');
        await invoke('toggle_tool', { name: btn.dataset.name, enabled });
        btn.classList.toggle('on');
      });
    });
  } catch (e) {
    console.error('Failed to load tools:', e);
  }
}

function addMessage(role, content) {
  const container = el('chat-messages');
  const div = document.createElement('div');
  div.className = 'message ' + role;

  if (role === 'assistant') {
    div.innerHTML = formatMarkdown(content);
  } else {
    const p = document.createElement('p');
    p.textContent = content;
    div.appendChild(p);
  }

  const time = document.createElement('div');
  time.className = 'timestamp';
  time.textContent = new Date().toLocaleTimeString();
  div.appendChild(time);

  // Remove empty state if present
  const empty = container.querySelector('.empty-state');
  if (empty) empty.remove();

  container.appendChild(div);
  container.scrollTop = container.scrollHeight;
  return div;
}

function showTyping() {
  const div = document.createElement('div');
  div.className = 'message assistant typing-indicator';
  div.id = 'typing';
  div.innerHTML = '<span></span><span></span><span></span>';
  el('chat-messages').appendChild(div);
  el('chat-messages').scrollTop = el('chat-messages').scrollHeight;
}

function hideTyping() {
  const t = el('typing');
  if (t) t.remove();
}

function appendToLastMessage(text) {
  const msgs = el('chat-messages').querySelectorAll('.message.assistant');
  if (msgs.length === 0) return;
  const last = msgs[msgs.length - 1];
  const content = formatMarkdown(text);
  last.innerHTML = content;
  const time = document.createElement('div');
  time.className = 'timestamp';
  time.textContent = new Date().toLocaleTimeString();
  last.appendChild(time);
}

async function sendMessage() {
  const input = el('chat-input');
  const text = input.value.trim();
  if (!text || isSending) return;
  if (!currentModel) { alert('Please select a model'); return; }

  isSending = true;
  el('send-btn').disabled = true;
  input.value = '';
  input.style.height = 'auto';

  addMessage('user', text);
  showTyping();

  try {
    const response = await invoke('chat', { message: text, model: currentModel });
    hideTyping();
    addMessage('assistant', response);
  } catch (e) {
    hideTyping();
    addMessage('assistant', 'Error: ' + e);
  }

  isSending = false;
  el('send-btn').disabled = false;
  input.focus();
}

async function addFolder() {
  const path = prompt('Folder path to add:');
  if (!path) return;
  try {
    await invoke('add_folder', { path });
    loadFolders();
  } catch (e) {
    alert('Failed to add folder: ' + e);
  }
}

function setupInput() {
  const input = el('chat-input');
  input.addEventListener('input', () => {
    input.style.height = 'auto';
    input.style.height = Math.min(input.scrollHeight, 200) + 'px';
  });
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });
}

function setEmptyState() {
  el('chat-messages').innerHTML =
    '<div class="empty-state"><h2>Welcome to Juno</h2>' +
    '<p>Your local AI agent with workspace tools. Add folders to the sidebar, select a model, and start chatting.</p></div>';
}

document.addEventListener('DOMContentLoaded', () => {
  el('send-btn').addEventListener('click', sendMessage);
  el('add-folder-btn').addEventListener('click', addFolder);
  el('new-chat-btn').addEventListener('click', async () => {
    await invoke('clear_chat');
    el('chat-messages').innerHTML = '';
    setEmptyState();
    el('chat-title').textContent = 'New Chat';
  });
  el('model-select').addEventListener('change', e => { currentModel = e.target.value; });
  el('sidebar-toggle').addEventListener('click', () => {
    el('sidebar').classList.toggle('collapsed');
  });

  setupInput();
  loadModels();
  loadFolders();
  loadTools();
  setEmptyState();
});
