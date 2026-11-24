; The OpenBot installer's own steps on Windows (tauri.conf.json, bundle.windows.nsis.installerHooks).
; The daemon, tirith.exe, outlives the app's window: while it runs, a new version cannot replace it,
; and the installer would stop on a "cannot write" dialog. It is ended first, out of sight (nsExec
; shows no window); the app starts the new one when it opens.

!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /F /T /IM tirith.exe'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /T /IM tirith.exe'
  Pop $0
!macroend
