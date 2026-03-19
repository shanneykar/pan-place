function timeAgo(ms) {
  const secs = Math.floor((Date.now() - ms) / 1000);
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

function truncateId(id, n = 8) {
  return id ? id.slice(0, n) + '…' : '—';
}

function humanType(t) {
  return {
    actor_registered: 'Actor Registered',
    pan_node_placed: 'Node Placed',
    presence_recorded: 'Presence',
    confirmation_recorded: 'Confirmation',
  }[t] || t;
}

// Build a confirmation count map: event_id -> count of confirmations referencing it
function buildConfirmCounts(events) {
  const counts = {};
  for (const e of events) {
    if (e.event_type === 'confirmation_recorded' && e.references_event) {
      counts[e.references_event] = (counts[e.references_event] || 0) + 1;
    }
  }
  return counts;
}

function renderEvent(e, confirmCounts) {
  const count = confirmCounts[e.event_id] || 0;
  const tags = (e.tags || []).map(t => `<span class="tag">${t}</span>`).join('');
  const confirmHtml = (e.event_type !== 'confirmation_recorded')
    ? `<span class="confirm-count"><strong>${count}</strong> confirm${count === 1 ? '' : 's'}</span>`
    : (e.references_event ? `<span class="confirm-count">re: ${truncateId(e.references_event)}</span>` : '');

  return `
    <div class="event">
      <div class="event-header">
        <span class="event-type">${humanType(e.event_type)}</span>
        <span class="event-time">${timeAgo(e.timestamp)}</span>
      </div>
      ${e.content ? `<div class="event-content">${e.content}</div>` : ''}
      <div class="event-meta">
        ${tags}
        ${confirmHtml}
      </div>
    </div>`;
}
