"""Chat adapter for hosts that speak the Muse Session Protocol (MSP): JSON-RPC 2.0 over stdio, as served by
``muse serve`` (no sockets, no ports). The manager session (``msession``) speaks the stream-json dialect, so this
turns MSP notifications into those lines and the session's replies back into MSP requests. No I/O here: the
caller writes ``send`` lines to the host and feeds ``ui`` lines to its normal event handler.

Flow: ``initialize`` -> ``session/start`` -> one ``turn/start`` per user message. The host asks permission with
``approval/request`` (a server request answered with an empty receipt); the decision is ``approval/decide``.
Not supported: resuming a host session, interrupting a turn (``stop`` / ``restart`` end the process).
"""

from __future__ import annotations

import json

PROTOCOL = "msp"


def _frame(obj: dict) -> str:
    return json.dumps(obj)


def _text(text: str) -> str:
    return _frame({"type": "assistant", "message": {"content": [{"type": "text", "text": text}]}})


def _delta(text: str) -> str:
    return _frame({"type": "stream_event", "event": {"type": "content_block_delta",
                                                      "delta": {"type": "text_delta", "text": text}}})


def _tool(iid: str, name: str, args) -> str:
    """A tool step as the CLI shows it: a ``tool_use`` block (``args`` is JSON text, or plain text)."""
    try:
        data = json.loads(args) if isinstance(args, str) and args.strip() else args
    except ValueError:
        data = args
    data = data if isinstance(data, dict) else ({"args": _clip(data, 120)} if data else {})
    return _frame({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": iid, "name": name, "input": data}]}})


def _clip(s: str, n: int) -> str:
    one = " ".join(str(s).split())
    return one if len(one) <= n else one[:n] + "..."


class Out:
    def __init__(self) -> None:
        self.send: list[str] = []
        self.ui: list[str] = []


class Msp:
    def __init__(self, effort: str = "minimal", version: str = "0"):
        self.effort, self.version = effort, version
        self.next = 0
        self.init_id = self.start_id = 0
        self.session: str | None = None
        self.queue: list[str] = []
        self.pending: dict[str, dict] = {}
        self.streamed: set[str] = set()
        self.stepped: set[str] = set()
        self.said: list[str] = []  # the assistant text of the running turn

    def _id(self) -> int:
        self.next += 1
        return self.next

    def _request(self, method: str, params: dict) -> tuple[int, str]:
        rid = self._id()
        return rid, _frame({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})

    def hello(self) -> list[str]:
        self.init_id, line = self._request("initialize", {
            "clientInfo": {"name": "tirith", "version": self.version},
            "capabilities": {"userInputDialogs": False}})
        return [line]

    def _turn(self, session: str, text: str) -> str:
        command = f"tirith-{self._id()}"
        return self._request("turn/start", {"sessionId": session, "commandId": command,
                                            "reasoningEffort": self.effort,
                                            "input": [{"type": "text", "text": text}]})[1]

    def user(self, text: str) -> list[str]:
        """One user message; held until the session is open."""
        if self.session:
            return [self._turn(self.session, text)]
        self.queue.append(text)
        return []

    def answer(self, approval_id: str, allow: bool, feedback: str = "") -> str | None:
        """The ``approval/decide`` line for an open question, None when it is not open or has no such choice."""
        p = self.pending.pop(approval_id, None)
        if p is None or not self.session:
            return None
        choice = p["allow"] if allow else p["deny"]
        if not choice:
            return None
        command = f"tirith-{self._id()}"
        return self._request("approval/decide", {
            "sessionId": self.session, "approvalId": p["approval_id"], "choiceId": choice, "commandId": command,
            "requirementId": p["requirement"], "feedback": None if allow else (feedback or "Denied by Tirith.")})[1]

    def incoming(self, line: str) -> Out:
        out = Out()
        try:
            v = json.loads(line)
        except ValueError:
            return out
        if not isinstance(v, dict):
            return out
        method, rid = v.get("method"), v.get("id")
        if method is not None and rid is not None:
            self._on_request(method, rid, v.get("params") or {}, out)
        elif method is not None:
            self._on_notification(method, v.get("params") or {}, out)
        elif rid is not None:
            self._on_response(rid, v, out)
        return out

    def _on_response(self, rid, v: dict, out: Out) -> None:
        if isinstance(v.get("error"), dict):
            out.ui.append(_text(f"Muse error: {v['error'].get('message') or 'request failed'}"))
            return
        if rid == self.init_id and rid:
            self.start_id, line = self._request("session/start", {"commandId": "tirith-start",
                                                                  "approvalMode": "promptUnmatched"})
            out.send.append(line)
        elif rid == self.start_id and rid:
            sid = ((v.get("result") or {}).get("session") or {}).get("sessionId")
            if not isinstance(sid, str):
                out.ui.append(_text("Muse error: session/start returned no session id"))
                return
            self.session = sid
            queued, self.queue = self.queue, []
            out.send.extend(self._turn(sid, t) for t in queued)

    def _on_request(self, method: str, rid, p: dict, out: Out) -> None:
        out.send.append(_frame({"jsonrpc": "2.0", "id": rid, "result": {}}))  # a receipt; the decision is a command
        approval_id = p.get("approvalId")
        if method != "approval/request" or not isinstance(approval_id, str):
            return
        choices = [c for c in p.get("availableChoices") or [] if isinstance(c, dict)]

        def pick(decision: str) -> str | None:
            same = [c for c in choices if c.get("decision") == decision]
            best = next((c for c in same if c.get("scope") == "once"), same[0] if same else None)
            return best.get("choiceId") if best else None

        self.pending[approval_id] = {"approval_id": approval_id, "requirement": p.get("currentRequirementId"),
                                     "allow": pick("approved"), "deny": pick("denied") or pick("abort")}
        s = p.get("subject") if isinstance(p.get("subject"), dict) else {}
        tool = s.get("toolName") or s.get("kind") or "a tool"
        data = {k: s[k] for k in ("command", "path", "host", "access") if s.get(k) is not None}
        if tool == "shell" or s.get("kind") == "shell":
            tool = "Bash"  # the manager's rules (gates, "delegate edits") know the CLI's tool names
        elif data.get("path") and data.get("access") == "write":
            tool, data["file_path"] = "Write", data["path"]
        if not data and isinstance(p.get("rawArgs"), str):
            data["args"] = _clip(p["rawArgs"], 300)
        out.ui.append(_frame({"type": "control_request", "request_id": approval_id,
                              "request": {"subtype": "can_use_tool", "tool_name": tool, "input": data}}))

    def _on_notification(self, method: str, p: dict, out: Out) -> None:
        if method == "item/delta" and p.get("field") == "text":
            if isinstance(p.get("delta"), str):
                self.streamed.add(str(p.get("itemId") or ""))
                self.said.append(p["delta"])
                out.ui.append(_delta(p["delta"]))
        elif method in ("item/started", "item/completed"):
            item = p.get("item") if isinstance(p.get("item"), dict) else {}
            iid, kind = str(item.get("itemId") or ""), item.get("kind")
            if kind == "agentMessage" and method == "item/completed" and iid not in self.streamed:
                if item.get("text"):
                    self.said.append(str(item["text"]))
                    out.ui.append(_text(str(item["text"])))
            elif kind in ("toolCall", "userShell") and iid not in self.stepped:
                self.stepped.add(iid)
                name = str(item.get("tool") or "shell")
                args = item.get("args") if item.get("args") else (
                    {"command": item["commandText"]} if item.get("commandText") else "")
                out.ui.append(_tool(iid, name, args))
        elif method == "turn/completed":
            failed = p.get("terminal") != "completed"
            err = p.get("error")
            if isinstance(err, dict) and err.get("message"):
                out.ui.append(_text(f"Muse error: {err['message']}"))
            said, self.said = "".join(self.said), []
            out.ui.append(_frame({"type": "result", "subtype": "error" if failed else "success",
                                  "is_error": failed, "result": said}))
