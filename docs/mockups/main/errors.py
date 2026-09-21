from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

def over(base, extra):
    return page(W, H, f'<img class="frame" src="{base}">{extra}')

def boards():
    rest = frame("main-rest-en-US")
    rendered = frame("main-rendering-en-US")
    out = {}

    out["ErrorInline"] = over(rest, '''
  <div style="position:absolute; left:40px; top:526px; width:520px; background:#0d0508; padding:2px 0;">
    <span class="btn quiet" style="padding-left:0;">Once more</span>
    <div class="line bad" style="height:20px; margin-top:2px;"><span class="g">✕</span><span>ffmpeg ended with code 1</span><span class="d">· <a href="#" style="color:#e24848; text-decoration:none;">Details</a></span></div>
  </div>''')

    out["ErrorToast"] = over(rest, '''
  <div class="toast" style="bottom:150px;">
    <div class="h"><span class="x">✕</span>Render did not finish</div>
    <div class="b">ffmpeg ended with code 1 — the encoder rejected the audio track. <a href="#">Details</a> · <a href="#">Once more</a></div>
  </div>
  <div class="toast" style="bottom:236px; opacity:0.55;">
    <div class="h"><span class="x" style="color:#a9a29b;">·</span>Map fetched</div>
    <div class="b">xi — Blue Zenith, 21,8 MB from osu.direct. Gone in a moment.</div>
  </div>''')

    out["ErrorCentre"] = over(rest, '''
  <div class="mark hot"></div>
  <div class="note" style="top:96px; text-align:right;">a red dot beside the words: something asks for attention; the panel (next page) says what</div>''')

    out["ErrorFatal"] = page(W, H, f'''
  <img class="frame" src="{rest}">
  <div class="veil"></div>
  <div class="card" style="left:210px; top:180px; width:560px; padding:24px;">
    <div style="font-size:16px; font-weight:600;">Something went wrong</div>
    <div class="cap" style="margin-top:6px;">Dossier could not read its own settings — the file is not JSON.</div>
    <div class="line done" style="margin-top:14px;"><span class="g">·</span><span>~/.dossier/dossier.json</span></div>
    <div class="line done"><span class="g">·</span><span>expected value at line 1, column 1</span></div>
    <div style="display:flex; align-items:center; gap:8px; margin-top:18px;">
      <span class="btn quiet" style="padding-left:0;">Copy the log</span>
      <span style="flex:1;"></span>
      <span class="btn quiet">Quit</span>
      <span class="btn primary">Start over</span>
    </div>
  </div>''')

    out["ErrorButton"] = over(rendered, '''
  <div style="position:absolute; left:40px; top:526px; width:300px; height:40px; background:#0d0508;"></div>
  <div style="position:absolute; left:40px; top:530px;">
    <span class="btn soft" style="border-color: rgba(226,72,72,0.45);">Did not finish<i style="width:60%; background:#e24848; opacity:0.4;"></i></span>
    <span class="cap" style="margin-left:12px;">ffmpeg ended with code 1 · <a href="#" style="color:#e24848; text-decoration:none;">Details</a></span>
  </div>''')

    out["ErrorOffline"] = over(rest, '''
  <div class="stripe"><b>Offline</b><span>The bot cannot be reached · the worker rests · maps cannot be fetched</span><span style="margin-left:auto;">Trying again in 40 s</span></div>''')
    return out

NOTES = {
    "ErrorInline": ("A · Under the button", "The button says Once more; under it, one ledger line in the danger colour with the reason and a Details link that opens the log. Stays until the next choice. Quiet, in place, no new surface — the ledger's own grammar."),
    "ErrorButton": ("B · The button itself", "The progress button turns danger at its edge and keeps its bar where it stopped; the reason sits beside it in a caption. Nothing moves. Best for the two long jobs, Render and Get the map."),
    "ErrorToast": ("C · A toast, bottom-left", "A card slides up from the bottom-left, stays 8 s or until hovered, stacks at most three; a failure keeps its links. Good for things that happen while the person looks elsewhere — a worker job, a map arriving — but a second surface to learn."),
    "ErrorCentre": ("D · Into the centre", "No card at all: the operations centre's mark turns red, the notice waits inside. Calmest; also easiest to miss."),
    "ErrorFatal": ("E · Cannot go on", "When the application itself cannot continue — settings unreadable, no folder writable, a renderer that will not start — the scene dims to a fifth and one card says what, where, and the two ways out. The same card as the first run's, so it is not a new thing."),
    "ErrorOffline": ("F · A state, not an error", "Being offline is not a failure to shout about: a stripe under the top row names what rests and when it tries again, and goes away by itself."),
}

RECOMMENDED = "Recommendation: B for Render and Get the map (the button already is the story), A for everything that has a ledger, F for states, E for the fatal few; C and D only once the operations centre exists, and then D feeds C — the centre keeps every notice, a toast shows the fresh one."

if __name__ == "__main__":
    out = boards()
    for name, html in out.items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote", len(out), "error boards")
