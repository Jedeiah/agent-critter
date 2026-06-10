/// Petdex-verbatim HTML/CSS/JS.
pub fn build_page(current_slug: &str, pets_json: &str, saved_scale: f64) -> String {
    let slug_json = serde_json::to_string(current_slug).unwrap_or_else(|_| "\"\"".into());

    format!(r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  html, body {{ margin: 0; padding: 0; background: transparent; overflow: hidden; width: 100%; height: 100%; font-family: -apple-system, system-ui, sans-serif; }}
  body input {{ -webkit-user-select: auto; user-select: auto; }}
  * {{ cursor: default !important; }}
  .stage {{ position: fixed; left: 0; top: 0; width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; }}
  .pet {{
    aspect-ratio: 192 / 208;
    width: 7rem;
    image-rendering: pixelated;
    background-image: url('/sprite');
    background-repeat: no-repeat;
    background-size: 800% 900%;
    background-position: 0% 0%;
    pointer-events: auto;
    cursor: grab;
  }}
  .pet.dragging {{ cursor: grabbing; }}
  ::-webkit-scrollbar {{ display: none; }}
</style>
</head>
<body>
<div class="stage"><div class="pet" id="pet" data-state="idle"></div></div>
<script>
var COLS = 8, ROWS = 9;
var STATES = {{
  idle:           {{ row: 0, frames: [{{c:0,d:280}},{{c:1,d:110}},{{c:2,d:110}},{{c:3,d:140}},{{c:4,d:140}},{{c:5,d:320}}], slow: 6 }},
  "running-right":{{ row: 1, count: 8, dur: 120, last: 220 }},
  "running-left": {{ row: 2, count: 8, dur: 120, last: 220 }},
  waving:         {{ row: 3, count: 4, dur: 140, last: 280 }},
  jumping:        {{ row: 4, count: 5, dur: 140, last: 280 }},
  failed:         {{ row: 5, count: 8, dur: 140, last: 240 }},
  waiting:        {{ row: 6, count: 6, dur: 150, last: 260 }},
  running:        {{ row: 7, count: 6, dur: 120, last: 220 }},
  review:         {{ row: 8, count: 6, dur: 150, last: 280 }},
}};
var CURRENT_SLUG = {slug_json};
window.__PETS = {pets_json};
function buildFrames(s) {{
  if (s.frames) {{ var slow = s.slow || 1; return s.frames.map(function(f) {{ return {{ c: f.c, r: s.row, d: f.d * slow }}; }}); }}
  return Array.from({{length: s.count}}, function(_,i) {{ return {{ c: i, r: s.row, d: i === s.count - 1 ? s.last : s.dur }}; }});
}}
function pos(c, r) {{ return c/(COLS-1)*100+'% '+r/(ROWS-1)*100+'%'; }}
var pet = document.getElementById('pet');
var stageEl = pet.parentElement;
if (stageEl) {{
  stageEl.style.position = 'fixed';
  stageEl.style.width = '100%';
  stageEl.style.height = '100%';
  stageEl.style.display = 'flex';
  stageEl.style.alignItems = 'center';
  stageEl.style.justifyContent = 'center';
}}
var currentState = 'idle', stateTimer = null;
function play(state) {{
  if (state === currentState) return;
  currentState = state;
  pet.dataset.state = state;
  if (stateTimer) {{ clearTimeout(stateTimer); stateTimer = null; }}
  var def = STATES[state] || STATES.idle;
  var frames = buildFrames(def);
  var i = 0;
  pet.style.backgroundPosition = pos(frames[0].c, frames[0].r);
  if (frames.length === 1) return;
  (function tick() {{
    stateTimer = setTimeout(function() {{
      i = (i + 1) % frames.length;
      pet.style.backgroundPosition = pos(frames[i].c, frames[i].r);
      tick();
    }}, frames[i].d);
  }})();
}}
play('idle');

// --- Bridge ---
var STATE_LABELS = {{idle:'空闲','running-right':'工作中','running-left':'工作中',running:'工作中',waving:'挥手中',waiting:'等待确认',failed:'崩溃了',review:'踩坑了',jumping:'跳跳'}};
window.__stateTimer = null;
window.setState = function(state, durationMs) {{
  play(state);
  if (window.__stateTimer) clearTimeout(window.__stateTimer);
  if (durationMs) {{
    window.__stateTimer = setTimeout(function() {{
      play(window.__realState || 'idle');
      window.__stateTimer = null;
    }}, durationMs);
  }}
}};
window.setHookState = function(state) {{
  window.__realState = state;
  window.__stateLabel = STATE_LABELS[state] || state;
}};
window.__realState = 'idle';
window.__stateLabel = '空闲';
window.__sessions = 0;
window.__savedScale = {saved_scale};

// Global scale function (used by menu and restore)
window.applyScale = function(s) {{
  window.__petScale = s;
  // 缩放整个页面（zoom 在 WKWebView 和 WebView2 均支持）
  document.body.style.zoom = s;
  var sizeLabel = document.getElementById('size-label');
  if (sizeLabel) sizeLabel.textContent = '🔍 大小 x'+s.toFixed(1);
  var pw = Math.ceil(112 * s);
  var ph = Math.ceil(pw / 192 * 208);
  window.ipc.postMessage(JSON.stringify({{type:'resize',w:pw+40, h:ph+60}}));
  window.ipc.postMessage(JSON.stringify({{type:'saveScale',scale:s}}));
}};

// Restore saved scale
if (window.__savedScale) window.applyScale(window.__savedScale);

// --- Bubble ---
var bubbleEl = null, bubbleTextEl = null;
function ensureBubble() {{
  if (bubbleEl) return bubbleEl;
  bubbleEl = document.createElement('div');
  bubbleEl.id = 'pet-bubble';
  bubbleEl.style.cssText = 'position:fixed;padding:4px 8px;border-radius:10px;background:#fff;color:#111;font:600 11px system-ui;line-height:1.2;box-shadow:0 2px 6px rgba(0,0,0,.3);text-align:left;white-space:normal;max-width:80vw;min-width:40px;display:flex;align-items:center;gap:6px;opacity:0;transition:opacity 180ms ease;pointer-events:none;z-index:5';
  bubbleTextEl = document.createElement('span');
  bubbleTextEl.style.cssText = 'display:block;min-width:0';
  bubbleEl.appendChild(bubbleTextEl);
  document.body.appendChild(bubbleEl);
  return bubbleEl;
}}
function positionBubble() {{
  if (!bubbleEl || !bubbleTextEl.textContent) return;
  bubbleEl.style.left = '50%';
  bubbleEl.style.transform = 'translateX(-50%)';
  var rect = pet.getBoundingClientRect();
  var bh = bubbleEl.offsetHeight || 22;
  bubbleEl.style.top = Math.max(2, rect.top - bh - 10)+'px';
}}
window.__persistentBubble = '';
window.setBubble = function(text, durationMs, persist) {{
  var el = ensureBubble();
  bubbleTextEl.textContent = text || '';
  clearTimeout(window.__bubbleTimer);
  if (persist) {{
    window.__persistentBubble = text || '';
  }}
  if (text) {{
    el.style.opacity = '1';
    positionBubble();
    if (!persist && durationMs) {{
      var saved = window.__persistentBubble;
      window.__bubbleTimer = setTimeout(function() {{
        if (saved) {{
          bubbleTextEl.textContent = saved;
          el.style.opacity = '1';
        }} else {{
          el.style.opacity = '0';
        }}
      }}, durationMs);
    }}
  }} else {{
    window.__persistentBubble = '';
    el.style.opacity = '0';
  }}
}};
// 用户交互触发的瞬态气泡：自动恢复底层的持久气泡
window.showTransientBubble = function(text, durationMs) {{
  var saved = window.__persistentBubble || '';
  var el = ensureBubble();
  bubbleTextEl.textContent = text;
  clearTimeout(window.__bubbleTimer);
  el.style.opacity = '1';
  positionBubble();
  window.__bubbleTimer = setTimeout(function() {{
    if (saved) {{
      bubbleTextEl.textContent = saved;
    }} else {{
      el.style.opacity = '0';
    }}
  }}, durationMs);
}};

// --- Drag: mousedown anywhere on window → move window ---
var dragging = false, wasDrag = false, startX = 0, startY = 0, lastMove = 0;
document.body.addEventListener('mousedown', function(e) {{
  if (e.button !== 0) return;
  if (navigator.platform.indexOf('Mac') !== -1 && (e.target === pet || pet.contains(e.target))) return;
  dragging = true; wasDrag = false;
  startX = e.screenX; startY = e.screenY; lastMove = 0;
}});
window.addEventListener('mousemove', function(e) {{
  if (!dragging) return;
  var now = Date.now();
  if (now - lastMove < 16) return; // throttle to ~60fps
  lastMove = now;
  var dx = e.screenX - startX, dy = e.screenY - startY;
  if (Math.abs(dx) > 2 || Math.abs(dy) > 2) wasDrag = true;
  window.ipc.postMessage(JSON.stringify({{type:'move',dx:dx,dy:dy}}));
  startX = e.screenX; startY = e.screenY;
}});
window.addEventListener('mouseup', function() {{
  if (!dragging) return;
  dragging = false;
  window.ipc.postMessage(JSON.stringify({{type:'savePos'}}));
  if (wasDrag) {{
    var dragMsg = (!window.__realState || window.__realState === 'idle') ? '哎呀，轻点拽~ 😵' : '别打扰我干活！';
    window.showTransientBubble(dragMsg, 2000);
    if (!window.__realState || window.__realState === 'idle') {{
      window.setState('waving', 2000);
    }}
  }}
}});

// --- Right-click pet menu ---
pet.addEventListener('contextmenu', function(e) {{
  e.preventDefault();
  window.ipc.postMessage(JSON.stringify({{act:'1'}}));
  // Activate the window so timers & blur work
  window.ipc.postMessage(JSON.stringify({{type:'focus'}}));
  var menu = document.getElementById('pet-menu');
  if (!menu) {{
    menu = document.createElement('div');
    menu.id = 'pet-menu';
    menu.style.cssText = 'position:fixed;background:rgba(20,20,22,0.96);border:1px solid rgba(255,255,255,0.08);border-radius:10px;padding:10px;z-index:999;left:4px;right:4px;top:4px;bottom:4px;overflow-y:auto;overflow-x:hidden;scrollbar-width:none;-ms-overflow-style:none;pointer-events:auto;display:none';
    document.body.appendChild(menu);
    // Dismiss on outside click or window blur
    document.addEventListener('click', function(ev) {{ if (menu && menu.style.display === 'block' && !menu.contains(ev.target) && ev.target !== pet) {{ menu.style.display = 'none'; document.body.style.pointerEvents = 'none'; }} }});
    window.addEventListener('blur', function() {{ if (menu) {{ menu.style.display = 'none'; document.body.style.pointerEvents = 'none'; }} }});
  }}
  menu.innerHTML = '<div style=\'padding:2px 8px 6px;color:rgba(255,255,255,0.4);font-size:9px;text-align:center\'>🐾 切换宠物</div>';
  var pets = window.__PETS || [];
  if (pets.length === 0) pets = [{{slug:'default',name:'Default'}}];
  pets.forEach(function(p) {{
    var row = document.createElement('div');
    row.style.cssText = 'display:flex;align-items:center;padding:2px 0';
    var item = document.createElement('span');
    item.textContent = p.name;
    item.style.cssText = 'flex:1;padding:2px 8px;border-radius:4px;color:#ccc;cursor:pointer;font-size:11px';
    if (p.slug === CURRENT_SLUG) item.style.color = '#00e676';
    item.addEventListener('mouseenter', function() {{ item.style.background = 'rgba(255,255,255,0.1)'; item.style.color = '#fff'; }});
    item.addEventListener('mouseleave', function() {{ item.style.background = ''; item.style.color = p.slug===CURRENT_SLUG?'#00e676':'#ccc'; }});
    item.addEventListener('click', function() {{
      window.ipc.postMessage(JSON.stringify({{theme:p.slug}}));
      menu.style.display = 'none';
      document.body.style.pointerEvents = 'none';
    }});
    row.appendChild(item);
    var del = document.createElement('span');
    del.textContent = '✕';
    del.style.cssText = 'padding:2px 6px;border-radius:3px;color:rgba(255,255,255,0.3);cursor:pointer;font-size:9px';
    del.addEventListener('mouseenter', function(){{ del.style.color='#f88'; }});
    del.addEventListener('mouseleave', function(){{ del.style.color='rgba(255,255,255,0.3)'; }});
    del.addEventListener('click', function(ev){{ ev.stopPropagation(); window.ipc.postMessage(JSON.stringify({{type:'deletePet',slug:p.slug}})); menu.style.display = 'none'; document.body.style.pointerEvents = 'none'; }});
    row.appendChild(del);
    menu.appendChild(row);
  }});
  menu.appendChild(document.createElement('hr'));
  var market = document.createElement('div');
  market.innerHTML = '🌐 浏览市场找名字';
  market.style.cssText = 'padding:5px 8px;border-radius:4px;color:#58a6ff;cursor:pointer;font-size:10px;text-align:center';
  market.addEventListener('mouseenter', function() {{ market.style.background = 'rgba(88,166,255,0.1)'; }});
  market.addEventListener('mouseleave', function() {{ market.style.background = ''; }});
  market.addEventListener('click', function() {{ window.ipc.postMessage(JSON.stringify({{url:'https://petdex.dev/collections'}})); menu.style.display = 'none'; }});
  menu.appendChild(market);
  // 输入框（占一行）
  var installInput = document.createElement('input');
  installInput.id = 'pet-install-input';
  installInput.placeholder = '输入宠物名字';
  installInput.style.cssText = 'display:block;width:calc(100% - 8px);margin:3px auto;padding:3px 4px;border:1px solid rgba(255,255,255,0.15);border-radius:3px;background:rgba(255,255,255,0.05);color:#ccc;font-size:10px;outline:none;box-sizing:border-box';
  installInput.addEventListener('keydown', function(ev) {{ if (ev.key === 'Enter') ev.stopPropagation(); }});
  menu.appendChild(installInput);
  // 按钮行
  var installBtns = document.createElement('div');
  installBtns.style.cssText = 'display:flex;gap:4px;justify-content:center;padding:2px 0;flex-wrap:nowrap';
  var installBtn = document.createElement('div');
  installBtn.textContent = '📥装';
  installBtn.style.cssText = 'padding:2px 8px;border-radius:4px;color:#4caf50;cursor:pointer;font-size:10px';
  installBtn.addEventListener('mouseenter', function(){{ installBtn.style.background='rgba(255,255,255,0.1)'; }});
  installBtn.addEventListener('mouseleave', function(){{ installBtn.style.background=''; }});
  installBtn.addEventListener('click', function() {{
    var name = installInput.value.trim().toLowerCase().replace(/\s+/g, '-');
    if (!name) {{ installInput.focus(); return; }}
    window.ipc.postMessage(JSON.stringify({{type:'installPet',name:name}}));
    menu.style.display = 'none';
    document.body.style.pointerEvents = 'none';
  }});
  installBtns.appendChild(installBtn);
  var randomBtn = document.createElement('div');
  randomBtn.textContent = '🎲随机';
  randomBtn.style.cssText = 'padding:2px 8px;border-radius:4px;color:#ff9800;cursor:pointer;font-size:10px';
  randomBtn.addEventListener('mouseenter', function(){{ randomBtn.style.background='rgba(255,255,255,0.1)'; }});
  randomBtn.addEventListener('mouseleave', function(){{ randomBtn.style.background=''; }});
  randomBtn.addEventListener('click', function() {{
    window.ipc.postMessage(JSON.stringify({{type:'installPet',name:'__random__'}}));
    menu.style.display = 'none';
    document.body.style.pointerEvents = 'none';
  }});
  installBtns.appendChild(randomBtn);
  menu.appendChild(installBtns);
  var hint = document.createElement('div');
  hint.textContent = '💡 浏览市场看宠物图和名字';
  hint.style.cssText = 'padding:0 8px 4px;color:rgba(255,255,255,0.35);font-size:8px;text-align:center';
  menu.appendChild(hint);
  menu.appendChild(document.createElement('hr'));

  // GitHub link
  // Size control
  window.__petScale = window.__petScale || 1.0;
  var sizeLabel = document.createElement('div');
  sizeLabel.id = 'size-label';
  sizeLabel.textContent = '🔍 大小 x' + window.__petScale.toFixed(1);
  sizeLabel.style.cssText = 'padding:3px 8px;color:rgba(255,255,255,0.4);font-size:9px;text-align:center';
  menu.appendChild(sizeLabel);
  var sizeRow = document.createElement('div');
  sizeRow.style.cssText = 'display:flex;gap:4px;justify-content:center;padding:2px 0';
  var minus = document.createElement('div');
  minus.textContent = '−'; minus.style.cssText = 'width:22px;text-align:center;color:#aaa;cursor:pointer;font-size:12px;border-radius:3px';
  minus.addEventListener('mouseenter', function(){{ minus.style.background='rgba(255,255,255,0.1)'; }});
  minus.addEventListener('mouseleave', function(){{ minus.style.background=''; }});
  var plus = document.createElement('div');
  plus.textContent = '+'; plus.style.cssText = 'width:22px;text-align:center;color:#aaa;cursor:pointer;font-size:12px;border-radius:3px';
  plus.addEventListener('mouseenter', function(){{ plus.style.background='rgba(255,255,255,0.1)'; }});
  plus.addEventListener('mouseleave', function(){{ plus.style.background=''; }});
  minus.addEventListener('click', function(e){{ e.stopPropagation(); window.applyScale(Math.max(0.5, window.__petScale - 0.1)); }});
  plus.addEventListener('click', function(e){{ e.stopPropagation(); window.applyScale(Math.min(1.5, window.__petScale + 0.1)); }});
  sizeRow.appendChild(minus); sizeRow.appendChild(plus);
  menu.appendChild(sizeRow);
  var hr2 = document.createElement('hr');
  hr2.style.cssText = 'margin:4px 0;border:none;border-top:1px solid rgba(255,255,255,0.06)';
  menu.appendChild(hr2);
  var github = document.createElement('div');
  github.innerHTML = '⭐ Star on GitHub';
  github.style.cssText = 'padding:5px 8px;border-radius:4px;color:#58a6ff;cursor:pointer;font-size:10px;text-align:center';
  github.addEventListener('mouseenter', function() {{ github.style.background = 'rgba(88,166,255,0.1)'; }});
  github.addEventListener('mouseleave', function() {{ github.style.background = ''; }});
  github.addEventListener('click', function() {{ window.ipc.postMessage(JSON.stringify({{url:'https://github.com/Jedeiah/agent-critter'}})); menu.style.display = 'none'; }});
  menu.appendChild(github);
  var quit = document.createElement('div');
  quit.textContent = '× 退出';
  quit.style.cssText = 'padding:4px 8px;border-radius:4px;color:#f88;cursor:pointer;font-size:11px;text-align:right';
  quit.addEventListener('mouseenter', function(){{ quit.style.background='rgba(255,255,255,0.05)'; }});
  quit.addEventListener('mouseleave', function(){{ quit.style.background=''; }});
  quit.addEventListener('click', function() {{ window.ipc.postMessage('quit'); menu.style.display = 'none'; }});
  menu.appendChild(quit);
  document.body.style.pointerEvents = 'auto';
  menu.style.display = 'block';
  menu.style.top = '4px';
  e.stopPropagation();
  // Auto-close after 3s (reset on hover)
  clearTimeout(window.__menuTimer);
  window.__menuTimer = setTimeout(function() {{ menu.style.display = 'none'; document.body.style.pointerEvents = 'none'; }}, 3000);
  menu.onmouseenter = function() {{ clearTimeout(window.__menuTimer); }};
  menu.onmouseleave = function() {{ window.__menuTimer = setTimeout(function() {{ menu.style.display = 'none'; document.body.style.pointerEvents = 'none'; }}, 1500); }};
}});

// --- Single-click: position-based interaction ---
var headTexts = ['嘿嘿，痒~', '摸头会长不高哦！', '（眯眼）舒服~', '再摸要收费了！', '（蹭蹭手）'];
var bodyTexts = [
  '戳我干嘛~','喵！','别闹，正忙着呢','有什么事吗主人？',
  '（打滚）','哼，不理你','再戳就咬你哦','嗯？叫我吗？',
  '（伸懒腰）今天天气不错~','（翻肚皮）摸摸~','盯——',
  '干嘛啦','再戳我要生气了','（蹭蹭）','呼噜呼噜...',
  '别摸了，代码要写不完了','（竖起耳朵）有Bug？','饿了...',
];
var clickActions = ['waving','jumping','waiting','review'];
pet.addEventListener('click', function(e) {{
  if (wasDrag) return;
  if (window.__realState && window.__realState !== 'idle') return;
  if (window.__clickBusy) return;
  window.__clickBusy = true;
  var rect = pet.getBoundingClientRect();
  var y = (e.clientY - rect.top) / rect.height;
  var t, a;
  if (y < 0.3) {{
    t = headTexts[Math.floor(Math.random() * headTexts.length)];
    a = 'waving';
  }} else {{
    t = bodyTexts[Math.floor(Math.random() * bodyTexts.length)];
    a = clickActions[Math.floor(Math.random() * clickActions.length)];
  }}
  window.showTransientBubble(t, 3000);
  window.setState(a, 2000);
  setTimeout(function() {{ window.__clickBusy = false; }}, 2000);
}});

// --- Double-click shows status with session count ---
pet.addEventListener('dblclick', function(e) {{
  var state = window.__realState || 'idle';
  var count = window.__sessions || 0;
  var msg;
  if (state === 'idle') {{
    if (count === 0) msg = '没人找我，好无聊啊...😴';
    else if (count === 1) msg = '1 个会话待命中 ✨';
    else msg = count + ' 个会话待命中 ✨';
  }} else if (state.indexOf('running') !== -1) {{
    msg = '工作中 · 共 ' + count + ' 个会话 🏃';
  }} else if (state === 'waiting') {{
    msg = '待确认 · 共 ' + count + ' 个会话 ⏳';
  }} else if (state === 'review') {{
    msg = '有异常 · 共 ' + count + ' 个会话 😅';
  }} else if (state === 'failed') {{
    msg = '出错了 · 共 ' + count + ' 个会话 😱';
  }} else {{
    msg = '会话: ' + count + ' | 状态: ' + (window.__stateLabel||'idle');
  }}
  window.showTransientBubble(msg, 3000);
}});


</script>
</body></html>"#, slug_json=slug_json, pets_json=pets_json, saved_scale=saved_scale)
}

pub fn build_empty_page(message: &str) -> String {
    let msg_json = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into());
    format!(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>
html,body{{margin:0;padding:0;background:transparent;width:100%;height:100%;font-family:-apple-system,sans-serif;-webkit-app-region:drag;display:flex;align-items:center;justify-content:center}}
p{{color:rgba(255,255,255,0.6);font-size:9px;text-align:center;white-space:pre-line;-webkit-app-region:no-drag}}
</style></head><body><p>{msg}</p></body></html>"#, msg=msg_json)
}

pub fn load_pet_bytes(slug: &str) -> Option<Vec<u8>> {
    if slug.contains("..") || slug.contains('/') || slug.contains('\\') {
        return None;
    }
    let home = crate::home_dir()?;
    let home = std::path::PathBuf::from(home);
    let bases = [home.join(".codex").join("pets"), home.join(".petdex").join("pets")];
    for base in &bases {
        for ext in &["webp", "png"] {
            let path = base.join(slug).join(format!("spritesheet.{}", ext));
            if let Ok(b) = std::fs::read(&path) { return Some(b); }
        }
    }
    None
}

pub fn find_first_pet() -> Option<(Vec<u8>, String)> {
    let home = crate::home_dir()?;
    let home = std::path::PathBuf::from(home);
    let bases = [home.join(".codex").join("pets"), home.join(".petdex").join("pets")];
    for base in &bases {
        let dir = std::fs::read_dir(base).ok()?;
        for entry in dir.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let slug = path.file_name()?.to_str()?.to_string();
            for ext in &["webp", "png"] {
                let sheet = path.join(format!("spritesheet.{}", ext));
                if let Ok(b) = std::fs::read(&sheet) { return Some((b, slug)); }
            }
        }
    }
    None
}