# A check run by hand on a Mac: the fixed Accessibility script of `neobot safari click|type`
# (rust/tirith/assets/safari-ax-find.js + safari-ax.js) against a WebKit window of its own, never Safari, showing a
# page as heavy as a chat app (12,000 elements). Each command must do to the page what it says, and all of them
# together must stay fast: walking the page for every command took 55 s here, asking WebKit takes about 3 s.
# The terminal it runs from needs Accessibility (System Settings > Privacy & Security). No CI machine grants that.
#   python3 scripts/checks/safari-ax/run.py
import pathlib, subprocess, sys, tempfile, time

here = pathlib.Path(__file__).resolve().parent
assets = here.parents[2] / 'rust/tirith/assets'
find = (assets / 'safari-ax-find.js').read_text()
script = find + '\n' + (assets / 'safari-ax.js').read_text()
STATUS = find + """
function run(argv) {
  const web = pageOf($.AXUIElementCreateApplication(Number(argv[0])), '');
  const got = web ? search(web, ['AXStaticTextSearchKey'], 'status:', 5) : null;
  return got && got.length ? str(got[0].el, 'AXValue') : '?';
}"""
# (command, target, text, what the answer holds, what the page then says)
STEPS = [
    ('type', 'Type a message', 'Günaydın reis!', 'it now says «Günaydın reis!»', ''),
    ('click', 'Send', '', 'clicked AXButton «Send»', 'status: sent Günaydın reis!'),
    ('click', 'React to message 1499', '', 'clicked AXButton', 'status: reacted 1499'),
    ('type', 'Search', 'hola', 'it now says «hola»', 'status: searched hola'),
    ('click', 'Never', '', 'ERROR not clicked, it is disabled', 'status: searched hola'),
    ('click', 'zzqx nothing here 7731', '', 'ERROR not found', 'status: searched hola'),
]

def osa(src, *words):
    out = subprocess.run(['osascript', '-l', 'JavaScript', '-e', src, *words], capture_output=True, text=True)
    return (out.stdout or out.stderr).strip()

work = pathlib.Path(tempfile.mkdtemp(prefix='neobot-ax-'))
subprocess.run([sys.executable, str(here / 'page.py'), str(work / 'page.html')], check=True, stdout=subprocess.DEVNULL)
subprocess.run(['swiftc', '-O', '-o', str(work / 'host'), str(here / 'host.swift')], check=True)
host = subprocess.Popen([str(work / 'host'), str(work / 'page.html')], stdout=subprocess.PIPE, text=True)
try:
    pid = host.stdout.readline().split()[1]
    time.sleep(3)
    total = 0.0
    for cmd, target, text, answer, page in STEPS:
        t = time.time()
        said = osa(script, cmd, target, text, '', pid)
        total += time.time() - t
        saw = osa(STATUS, pid)
        print(f'{time.time() - t:5.1f}s  {cmd} "{target}" -> {said[:90]} | {saw}')
        assert answer in said, f'{cmd} {target}: {said}'
        assert not page or saw == page, f'after {cmd} {target} the page says: {saw}'
    assert total < 15, f'{total:.1f} s: the page is being walked again'
    print(f'{total:.1f} s for {len(STEPS)} commands: each did what it says')
finally:
    host.terminate()
