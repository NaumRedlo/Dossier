from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

def over(base, extra):
    return page(W, H, f'<img class="frame" src="{base}">{extra}')

PANEL = '''
  <div class="panel">
    <div class="sec">
      <h3>Now</h3>
      <div class="row2"><span style="flex:1;">Rendering · NaumRedlo — Daisuke</span><span class="n">62 %</span></div>
      <div class="bar"><i style="width:62%;"></i></div>
      <div class="row2" style="margin-top:10px;"><span style="flex:1;">Fetching · xi — Blue Zenith</span><span class="n">14,2 / 21,8 MB</span></div>
      <div class="bar"><i style="width:65%;"></i></div>
    </div>
    <div class="sec">
      <h3>Worker</h3>
      <div class="row2"><span class="dot" style="display:block; width:8px; height:8px; border-radius:50%; background:#e24848; box-shadow:0 0 0 4px rgba(226,72,72,0.16);"></span><span>Ready to take work</span><span class="n" style="margin-left:auto;">2 in the queue · 14 today</span></div>
    </div>
    <div class="sec">
      <h3>Notices</h3>
      <div class="line bad" style="height:22px;"><span class="g">✕</span><span>Render did not finish</span><span class="d">· 14:02</span></div>
      <div class="line done" style="height:22px;"><span class="g">✓</span><span>Map fetched · Blue Zenith</span><span class="d">· 13:51</span></div>
      <div class="line done" style="height:22px;"><span class="g">✓</span><span>Rendered for @friend · sent</span><span class="d">· 12:20</span></div>
    </div>
    <div class="sec">
      <h3>Telegram</h3>
      <div class="who"><span class="ava"></span><div><div>@naumredlo</div><div class="n">linked · finished renders go to this chat</div></div></div>
    </div>
    <div class="sec">
      <h3>Build</h3>
      <div class="row2"><span>0.12.0 · same as the bot's</span></div>
    </div>
  </div>'''

DRAWER = '''
  <div class="drawer">
    <h2>Operations</h2>
    <div class="line now"><span class="g" style="color:#e24848;">●</span><b>Rendering</b><span class="d">· Daisuke · 62 %</span></div>
    <div class="line now"><span class="g" style="color:#e24848;">●</span><b>Fetching</b><span class="d">· Blue Zenith · 65 %</span></div>
    <div class="line done"><span class="g">✓</span><span>Map fetched</span><span class="d">· 13:51</span></div>
    <div class="line bad"><span class="g">✕</span><span>Render did not finish</span><span class="d">· 14:02</span></div>
    <div class="line done"><span class="g">✓</span><span>Rendered for @friend</span><span class="d">· 12:20</span></div>
    <div class="line done"><span class="g">✓</span><span>Worker took a job</span><span class="d">· 12:04</span></div>
    <div class="line todo"><span class="g">·</span><span>Build 0.12.0 · same as the bot's</span></div>
    <div style="position:absolute; left:28px; right:28px; bottom:26px;">
      <div class="who" style="display:flex; align-items:center; gap:10px;"><span class="ava" style="width:28px; height:28px; border-radius:50%; background:linear-gradient(135deg,#3a1015,#e24848);"></span><div><div>@naumredlo</div><div class="n">worker ready · 2 in the queue</div></div></div>
    </div>
  </div>'''

def boards():
    rest = frame("main-rest-en-US")
    out = {}
    out["CentreMark"] = over(rest, '''
  <div class="mark"></div>
  <div class="note" style="top:60px; left:auto; right:40px; width:300px; text-align:right;">the mark: an 8 px dot beside the words — faint when quiet, red when something asks</div>''')
    out["CentrePanel"] = over(rest, '<div class="mark hot"></div>' + PANEL)
    out["CentreDrawer"] = over(rest, '<div class="veil" style="background:rgba(7,3,4,0.35);"></div>' + DRAWER)
    out["CentreStripe"] = over(rest, '''
  <div class="stripe"><b>Rendering</b><span>Daisuke · 62 %</span><b>Fetching</b><span>Blue Zenith · 65 %</span><span style="margin-left:auto;"><b>Worker</b> ready · 2 in the queue</span></div>''')
    out["CentreTelegram"] = over(frame("main-rendered-en-US"), '''
  <div style="position:absolute; left:40px; top:522px; width:400px; height:44px; background:#0d0508;"></div>
  <div style="position:absolute; left:40px; top:526px; display:flex; align-items:center; gap:6px;">
    <span class="btn primary">Open</span><span class="btn quiet">In folder</span><span class="btn quiet">Send to Telegram</span>
  </div>
  <div class="toast" style="top:96px; left:auto; right:40px; width:340px;">
    <div class="h"><span class="x" style="color:#a9a29b;">·</span>Sent to @naumredlo</div>
    <div class="b">The bot delivered the video to your chat — 49 MB.</div>
  </div>''')
    return out

NOTES = {
    "CentreMark": ("A · The mark", "At rest the centre is one 8 px dot to the right of Settings, faint; it turns red when a notice waits, and breathes while the worker draws. Clicking opens B. Nothing else is added to the chrome — the three words stay three."),
    "CentrePanel": ("B · The panel", "A 380 px card drops from the mark: Now (every job with a bar), Worker (state, queue, today), Notices (the last few, ticks and crosses, times), Telegram (who is linked, where finished renders go), Build. Esc or a click outside closes it."),
    "CentreDrawer": ("C · The drawer", "The same content as one ledger, sliding from the right edge over the dimmed scene: jobs first, then what happened, newest first; the person at the bottom. More room, more solemn; a second screen in all but name."),
    "CentreStripe": ("D · The stripe", "No panel: a single line under the top row while anything is going on, and nothing when nothing is. Cheapest, calmest, but it cannot hold a history."),
    "CentreTelegram": ("What Telegram gives", "The bot knows the person. After a render, Send to Telegram beside Open hands the file to the bot, which puts it in the chat; the worker's finished jobs can be announced there too; the centre greets by name. All of it is one HTTP call per event on the bot's existing API."),
}

TO_BUILD = ("What the centre needs built, in order: (1) a notice queue in the application — one struct, kind · words · time · a link — fed by Render, Get the map, the scan and the worker, kept in ~/.dossier/notices.json, at most a hundred; (2) the mark and the panel, with the count of unseen notices; (3) the worker's own events from the bot (a job taken, drawn, delivered) over the existing hello endpoint polled every 30 s, or a long poll when the bot grows one; (4) Telegram: POST /render/send with the file, and the bot's answer as a notice; (5) the build line, from hello, with Download when the bot wants a newer one — the same fetch-and-unpack as ffmpeg's.")

RECOMMENDED = "Recommendation: A + B. The mark keeps the chrome as it is; the panel is a card like every other card; the drawer is for later if notices outgrow a card. D can live alongside as the in-progress line for people who never open the panel."

if __name__ == "__main__":
    out = boards()
    for name, html in out.items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote", len(out), "centre boards")
