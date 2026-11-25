# A check run by hand on a Mac: the fixed Accessibility script of `neobot safari click|type|find|wait`
# (rust/tirith/assets/safari-ax-find.js + safari-ax-look.js + safari-ax.js) against a WebKit window of its own,
# never Safari, showing a page as heavy as a chat app (12,000 elements). Each command must do to the page what it
# says and answer what steps.py expects, and one that waits for nothing must stay fast: walking the page took 4 to
# 16 s a command here (55 s for the first six), asking WebKit takes well under 4.
# The terminal it runs from needs Accessibility (System Settings > Privacy & Security). No CI machine grants that.
#   python3 scripts/checks/safari-ax/run.py
import pathlib, subprocess, sys, tempfile, time

here = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(here))
sys.dont_write_bytecode = True  # no __pycache__ left in the repository
from steps import STEPS

assets = here.parents[2] / 'rust/tirith/assets'
find = (assets / 'safari-ax-find.js').read_text()
script = '\n'.join([find, (assets / 'safari-ax-look.js').read_text(), (assets / 'safari-ax.js').read_text()])
STATUS = find + """
function run(argv) {
  const web = pageOf($.AXUIElementCreateApplication(Number(argv[0])), '');
  const got = web ? search(web, ['AXStaticTextSearchKey'], 'status:', 5) : null;
  return got && got.length ? str(got[0].el, 'AXValue') : '?';
}"""

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
    quick = 0.0
    for cmd, target, text, answers, page, waits in STEPS:
        t = time.time()
        said = osa(script, cmd, target, text, '', pid)
        took = time.time() - t
        quick += 0 if waits else took
        saw = osa(STATUS, pid)
        print(f'{took:5.1f}s  {cmd} "{target}" -> {said[:110]!r} | {saw}')
        for answer in answers:
            assert answer in said, f'{cmd} {target} should say «{answer}»: {said}'
        assert not page or saw == page, f'after {cmd} {target} the page says: {saw}'
        assert waits or took < 4, f'{cmd} {target} took {took:.1f} s: the page is being walked again'
    print(f'{len(STEPS)} commands did what they say; {quick:.1f} s for those that wait for nothing')
finally:
    host.terminate()
