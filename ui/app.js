/* stacklens UI - force graph + table over /api/graph */
const TYPES = ['repo', 'agent', 'service', 'framework', 'runtime', 'container', 'port', 'mcp'];
const TYPE_LABEL = { repo: 'Repos', agent: 'AI agents', service: 'Services', framework: 'Frameworks', runtime: 'Runtimes', container: 'Containers', port: 'Ports', mcp: 'MCP servers' };
const color = t => `var(--t-${t})`;
const radius = n => ({ repo: 14, agent: 12, service: 11, container: 10 }[n.type] || 8);

const $ = s => document.querySelector(s);
const state = { data: null, hidden: new Set(['port']), q: '', selected: null, view: 'graph' };
let sim, svg, g, linkSel, nodeSel, zoom;

async function load(rescan = false) {
  $('#rescan').disabled = true; $('#rescan').textContent = rescan ? 'Scanning…' : 'Loading…';
  const res = await fetch(rescan ? '/api/rescan' : '/api/graph', { method: rescan ? 'POST' : 'GET' });
  state.data = await res.json();
  $('#rescan').disabled = false; $('#rescan').textContent = 'Rescan';
  $('#generated').textContent = 'scanned ' + new Date(state.data.generatedAt).toLocaleString();
  renderStats(); renderLegend(); buildGraph(); renderTable(); applyFilters();
}

function visibleNodes() {
  const q = state.q.toLowerCase();
  return state.data.nodes.filter(n => !state.hidden.has(n.type) && (!q || n.label.toLowerCase().includes(q) || JSON.stringify(n.meta).toLowerCase().includes(q)));
}
function degree(id) { return state.data.edges.filter(e => e.from === id || e.to === id).length; }

function renderStats() {
  const counts = {}; for (const n of state.data.nodes) counts[n.type] = (counts[n.type] || 0) + 1;
  const sessions = state.data.nodes.filter(n => n.type === 'repo').reduce((a, n) => a + (n.meta.claudeSessions || 0), 0);
  const tiles = [
    ['Repos', counts.repo || 0], ['AI agents', counts.agent || 0], ['Services', counts.service || 0],
    ['Frameworks', counts.framework || 0], ['Runtimes', counts.runtime || 0], ['Connections', state.data.edges.length],
    ['Claude sessions', sessions],
  ];
  $('#stats').innerHTML = tiles.map(([l, n]) => `<div class="tile"><div class="n">${n}</div><div class="l">${l}</div></div>`).join('');
}

function renderLegend() {
  const counts = {}; for (const n of state.data.nodes) counts[n.type] = (counts[n.type] || 0) + 1;
  $('#legend').innerHTML = TYPES.map(t => `<button class="chip ${state.hidden.has(t) ? 'off' : ''}" data-type="${t}" style="--c:${color(t)}" aria-pressed="${!state.hidden.has(t)}"><span class="dot"></span>${TYPE_LABEL[t]}<span class="cnt">${counts[t] || 0}</span></button>`).join('');
  $('#legend').querySelectorAll('.chip').forEach(b => b.onclick = () => {
    const t = b.dataset.type; state.hidden.has(t) ? state.hidden.delete(t) : state.hidden.add(t);
    renderLegend(); applyFilters();
  });
}

function buildGraph() {
  const wrap = $('#graph-wrap'); const W = wrap.clientWidth, H = wrap.clientHeight;
  svg = d3.select('#graph').attr('viewBox', [0, 0, W, H]); svg.selectAll('*').remove();
  g = svg.append('g');
  zoom = d3.zoom().scaleExtent([0.2, 4]).on('zoom', e => g.attr('transform', e.transform));
  svg.call(zoom).on('click', () => select(null));

  const nodes = state.data.nodes.map(n => ({ ...n }));
  const byId = new Map(nodes.map(n => [n.id, n]));
  const links = state.data.edges.map(e => ({ ...e, source: byId.get(e.from), target: byId.get(e.to) })).filter(l => l.source && l.target);
  state.nodes = nodes; state.links = links;

  linkSel = g.append('g').selectAll('line').data(links).join('line').attr('class', 'link');
  nodeSel = g.append('g').selectAll('g').data(nodes).join('g').attr('class', 'node')
    .call(d3.drag().on('start', (e, d) => { if (!e.active) sim.alphaTarget(.3).restart(); d.fx = d.x; d.fy = d.y; })
      .on('drag', (e, d) => { d.fx = e.x; d.fy = e.y; })
      .on('end', (e, d) => { if (!e.active) sim.alphaTarget(0); d.fx = d.fy = null; }));
  nodeSel.append('circle').attr('r', radius).attr('fill', d => color(d.type))
    .on('click', (e, d) => { e.stopPropagation(); select(d.id); })
    .on('mousemove', (e, d) => showTip(e, d)).on('mouseleave', hideTip);
  nodeSel.append('text').attr('dy', d => radius(d) + 12).attr('text-anchor', 'middle').text(d => d.label);

  sim = d3.forceSimulation(nodes)
    .force('link', d3.forceLink(links).id(d => d.id).distance(l => l.kind === 'depends-on' ? 80 : 120).strength(.5))
    .force('charge', d3.forceManyBody().strength(-420))
    .force('collide', d3.forceCollide(d => radius(d) + 22))
    .force('center', d3.forceCenter(W / 2, H / 2))
    .on('tick', () => {
      linkSel.attr('x1', d => d.source.x).attr('y1', d => d.source.y).attr('x2', d => d.target.x).attr('y2', d => d.target.y);
      nodeSel.attr('transform', d => `translate(${d.x},${d.y})`);
    });
}

function applyFilters() {
  const vis = new Set(visibleNodes().map(n => n.id));
  const sel = state.selected;
  const neigh = new Set();
  if (sel) for (const l of state.links) { if (l.source.id === sel) neigh.add(l.target.id); if (l.target.id === sel) neigh.add(l.source.id); }
  nodeSel.style('display', d => vis.has(d.id) ? null : 'none').classed('dim', d => sel && d.id !== sel && !neigh.has(d.id));
  linkSel.style('display', d => vis.has(d.source.id) && vis.has(d.target.id) ? null : 'none')
    .classed('hi', d => sel && (d.source.id === sel || d.target.id === sel))
    .classed('dim', d => sel && d.source.id !== sel && d.target.id !== sel);
  renderTable();
}

function select(id) { state.selected = id; applyFilters(); renderPanel(); }

function renderPanel() {
  const p = $('#panel');
  const n = state.data.nodes.find(x => x.id === state.selected);
  if (!n) { p.innerHTML = '<p class="muted">Select a node to see what it connects to.</p>'; return; }
  const meta = Object.entries(n.meta).filter(([, v]) => v != null && v !== '' && !(Array.isArray(v) && !v.length));
  const out = state.data.edges.filter(e => e.from === n.id).map(e => [e.kind, state.data.nodes.find(x => x.id === e.to)]);
  const inn = state.data.edges.filter(e => e.to === n.id).map(e => [e.kind, state.data.nodes.find(x => x.id === e.from)]);
  const li = ([k, m]) => m ? `<li data-id="${m.id}"><span class="dot" style="--c:${color(m.type)}"></span>${m.label}<span class="kind">${k}</span></li>` : '';
  p.innerHTML = `<div class="type">${TYPE_LABEL[n.type]} · via ${n.source}</div><h2><span class="dot" style="--c:${color(n.type)}"></span>${n.label}</h2>
    <dl>${meta.map(([k, v]) => `<dt>${k}</dt><dd>${Array.isArray(v) ? v.join(', ') : String(v)}</dd>`).join('')}</dl>
    ${out.length ? `<h3>Uses (${out.length})</h3><ul>${out.map(li).join('')}</ul>` : ''}
    ${inn.length ? `<h3>Used by (${inn.length})</h3><ul>${inn.map(li).join('')}</ul>` : ''}`;
  p.querySelectorAll('li[data-id]').forEach(el => el.onclick = () => select(el.dataset.id));
}

function renderTable() {
  const rows = visibleNodes().sort((a, b) => TYPES.indexOf(a.type) - TYPES.indexOf(b.type) || a.label.localeCompare(b.label));
  $('#table tbody').innerHTML = rows.map(n => `<tr data-id="${n.id}"><td><span class="dot" style="--c:${color(n.type)}"></span> ${TYPE_LABEL[n.type]}</td><td>${n.label}</td><td>${degree(n.id)}</td><td class="details">${Object.entries(n.meta).filter(([, v]) => v != null && v !== '').slice(0, 4).map(([k, v]) => `${k}: ${Array.isArray(v) ? v.join(', ') : v}`).join(' · ')}</td></tr>`).join('');
  $('#table tbody').querySelectorAll('tr').forEach(tr => tr.onclick = () => select(tr.dataset.id));
}

function showTip(e, d) {
  const t = $('#tooltip'); t.hidden = false;
  const r = $('#graph-wrap').getBoundingClientRect();
  t.innerHTML = `<b>${d.label}</b>${TYPE_LABEL[d.type]} · ${degree(d.id)} links`;
  t.style.left = (e.clientX - r.left + 14) + 'px'; t.style.top = (e.clientY - r.top + 14) + 'px';
}
function hideTip() { $('#tooltip').hidden = true; }

function setView(v) {
  state.view = v;
  $('#graph-wrap').hidden = v !== 'graph'; $('#table-wrap').hidden = v !== 'table';
  $('#view-graph').classList.toggle('on', v === 'graph'); $('#view-table').classList.toggle('on', v === 'table');
  $('#view-graph').setAttribute('aria-pressed', v === 'graph'); $('#view-table').setAttribute('aria-pressed', v === 'table');
}

$('#search').oninput = e => { state.q = e.target.value; applyFilters(); };
$('#rescan').onclick = () => load(true);
$('#view-graph').onclick = () => setView('graph');
$('#view-table').onclick = () => setView('table');
window.addEventListener('resize', () => { if (state.data) { buildGraph(); applyFilters(); } });
load();
