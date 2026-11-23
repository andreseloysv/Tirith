//! `tirith safari click <target> [<url>]` and `type <target> <text> [<url>]`: the owner lets the chat roles
//! click and type in their Safari without asking (their choice of 2026-11-23). The agent sends only words — the
//! target (a CSS selector, or the words on the element or its label), a text and a URL — never code: the daemon's
//! fixed JavaScript finds the element and acts on it, run by Safari's `do JavaScript` (which needs Safari's
//! Develop › Allow JavaScript from Apple Events). It never reads cookies, storage or passwords.

/// How the commands that act are written.
pub const USAGE: &str =
    "tirith safari click <target> [<url>] | tirith safari type <target> <text> [<url>]";

/// The visible element a selector or some words name: an exact label first, else the smallest one that says them.
const FIND: &str = concat!(
    "const norm=s=>(s||'').replace(/\\s+/g,' ').trim().toLowerCase();",
    "const seen=e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);",
    "return r.width>0&&r.height>0&&s.visibility!=='hidden'&&s.display!=='none'};",
    "const own=e=>norm(e.innerText||e.value||e.getAttribute('aria-label')||e.placeholder||e.title);",
    "const said=e=>norm([e.innerText,e.getAttribute('aria-label'),e.title,e.placeholder,e.name,e.id,",
    "e.labels&&[...e.labels].map(l=>l.innerText).join(' ')].filter(Boolean).join(' '));",
    "const find=(want,kinds)=>{let hit=[];try{hit=[...document.querySelectorAll(want)].filter(seen)}catch(e){}",
    "if(hit.length)return hit[0];const w=norm(want),all=[...document.querySelectorAll(kinds)].filter(seen);",
    "const exact=all.find(e=>own(e)===w);if(exact)return exact;",
    "const part=all.filter(e=>said(e).includes(w)).sort((a,b)=>said(a).length-said(b).length);",
    "return part[0]||null};",
    "const named=e=>e.tagName.toLowerCase()+' «'+(e.innerText||e.value||e.getAttribute('aria-label')",
    "||e.placeholder||e.name||'').replace(/\\s+/g,' ').trim().slice(0,80)+'» on '+location.href;",
);

/// A click as a person makes it: the element in view, pressed and released, then clicked.
const CLICK: &str = concat!(
    "const el=find(want,'button,a,input[type=submit],input[type=button],input[type=checkbox],input[type=radio],",
    "select,summary,label,option,[role=button],[role=link],[role=menuitem],[role=option],[role=tab],",
    "[role=checkbox],[role=radio],[role=switch],[onclick]')||find(want,'body *');",
    "if(!el)return 'not found: '+want;el.scrollIntoView({block:'center'});",
    "for(const t of ['pointerdown','mousedown','pointerup','mouseup'])",
    "el.dispatchEvent(new MouseEvent(t,{bubbles:true,cancelable:true,view:window}));",
    "el.click();return 'clicked '+named(el);",
);

/// Typing as a page's own scripts see it: the value set through the field's own setter, then input and change.
const TYPE: &str = concat!(
    "const el=find(want,'input:not([type=hidden]),textarea,select,[contenteditable=\"\"],",
    "[contenteditable=true],[role=textbox],[role=combobox]');",
    "if(!el)return 'not found: '+want;el.scrollIntoView({block:'center'});el.focus();",
    "if(el.tagName==='SELECT'){const o=[...el.options].find(o=>norm(o.text)===norm(value)||o.value===value)",
    "||[...el.options].find(o=>norm(o.text).includes(norm(value)));",
    "if(!o)return 'no option «'+value+'» in '+named(el);el.value=o.value}",
    "else if(el.isContentEditable){el.textContent=value}",
    "else{const p=el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;",
    "Object.getOwnPropertyDescriptor(p,'value').set.call(el,value)}",
    "el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));",
    "return 'typed into '+named(el);",
);

/// The AppleScript that runs the fixed JavaScript of `cmd` on the current tab, or on the first tab whose URL starts
/// with `url`; the agent's words ride in only as JSON strings, escaped once more for AppleScript.
pub fn act_script(cmd: &str, target: &str, text: &str, url: &str) -> Result<String, String> {
    let control = |s: &str| s.chars().any(char::is_control);
    let fits = !target.trim().is_empty() && !control(target) && !control(url);
    let text_fits = text.chars().all(|c| c == '\n' || !c.is_control());
    let js = match (cmd, fits, text_fits) {
        ("click", true, _) => format!("(function(want){{{FIND}{CLICK}}})({})", quoted(target)),
        ("type", true, true) => format!(
            "(function(want,value){{{FIND}{TYPE}}})({},{})",
            quoted(target),
            quoted(text)
        ),
        _ => return Err(USAGE.into()),
    };
    let (js, url) = (applescript(&js), applescript(url));
    Ok(match url.is_empty() {
        true => format!(
            "tell application \"Safari\" to do JavaScript \"{js}\" in current tab of front window"
        ),
        false => format!("tell application \"Safari\"\nrepeat with w in windows\nrepeat with t in tabs of w\nif (URL of t) starts with \"{url}\" then return do JavaScript \"{js}\" in t\nend repeat\nend repeat\nerror \"no tab starts with that URL\"\nend tell"),
    })
}

/// `s` as a JSON string, which is also a JavaScript string.
fn quoted(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// `s` escaped to sit inside an AppleScript string.
fn applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
