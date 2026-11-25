# A page as heavy as a chat app: thousands of nested elements, hundreds of buttons and links, a message box and
# a Send button at the very end (where a chat keeps them), and a list. What is done to it shows in its line that
# starts with «status:». scripts/checks/safari-ax/run.py makes it and reads that line.
import sys

rows = []
for i in range(1500):
    rows.append(
        f'<div class="row" role="listitem" onclick="say(\'opened Person {i}\')"><span class="who">Person {i}</span>'
        f'<div class="msg"><p>Message number {i} with <b>some</b> <i>words</i> in it</p>'
        f'<span class="time">09:{i % 60:02d}</span></div>'
        f'<button type="button" aria-label="React to message {i}" onclick="event.stopPropagation();say(\'reacted {i}\')">+</button>'
        f'<a href="#m{i}">Link {i}</a></div>'
    )
html = f"""<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Heavy page</title>
<style>body{{font:13px sans-serif}} .row{{display:flex;gap:6px}}</style></head><body>
<h1>Heavy page</h1>
<p id="status">status: nothing yet</p>
<script>function say(what){{document.getElementById('status').textContent='status: '+what}}</script>
<nav>{''.join(f'<button type="button">Chat {i}</button>' for i in range(120))}</nav>
<main>{''.join(rows)}</main>
<label>Colour <select onchange="say('picked '+this.value)"><option>Red</option><option>Green</option><option>Blue</option></select></label>
<div role="textbox" contenteditable="true" aria-label="Type a message" data-placeholder="Type a message"
     oninput="document.getElementById('draft').textContent=this.textContent"></div>
<input placeholder="Search" oninput="say('searched '+this.value)">
<p id="draft"></p>
<button type="button" onclick="say('sent '+document.getElementById('draft').textContent)">Send</button>
<button type="button" disabled>Never</button>
<label><input type="checkbox" checked onchange="say('teams '+this.checked)"> Teams meeting</label>
<button type="button" onclick="later('Late thing', 900)">Start soon</button>
<button type="button" onclick="later('Much later thing', 4000)">Start late</button>
<script>function later(words, ms){{setTimeout(function(){{var b=document.createElement('button');b.textContent=words;b.onclick=function(){{say('pressed '+words)}};document.body.appendChild(b)}}, ms)}}</script>
<iframe title="A frame" src="data:text/html,<button onclick=&quot;this.textContent='Framed done'&quot;>Inside frame</button><p>Words in a frame</p>"></iframe>
</body></html>"""
open(sys.argv[1], 'w').write(html)
print(len(html), 'bytes')
