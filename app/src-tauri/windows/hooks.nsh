; The NeoBot installer's own steps on Windows (tauri.conf.json, bundle.windows.nsis.installerHooks).
; The daemon (neobot.exe; tirith.exe in rc.1-rc.3, and the alias the app links beside it) outlives the
; app's window: while it runs, a new version cannot replace it, and the installer would stop on a
; "cannot write" dialog. It is ended first, out of sight (nsExec shows no window); the app starts the
; new one when it opens. Uninstalling also removes the alias, which the installer never wrote.

!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /F /T /IM neobot.exe'
  Pop $0
  nsExec::Exec 'taskkill /F /T /IM tirith.exe'
  Pop $0
!macroend

; NeoBot was called OpenBot in its first Windows test builds (0.2.2-rc.1 to rc.3): that install goes,
; silently (its own uninstaller, which keeps the app's data when silent), so one NeoBot remains and a
; bare `tirith` no longer finds the old binary first on PATH.
!macro NSIS_HOOK_POSTINSTALL
  ${If} ${FileExists} "$LOCALAPPDATA\OpenBot\uninstall.exe"
    ExecWait '"$LOCALAPPDATA\OpenBot\uninstall.exe" /S _?=$LOCALAPPDATA\OpenBot'
    RMDir /r "$LOCALAPPDATA\OpenBot"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /T /IM neobot.exe'
  Pop $0
  nsExec::Exec 'taskkill /F /T /IM tirith.exe'
  Pop $0
  Delete "$INSTDIR\tirith.exe"
!macroend
