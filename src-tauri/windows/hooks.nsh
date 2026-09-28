; Prospector: NSIS installer hooks (tauri.conf.json → bundle.windows.nsis.installerHooks).
; The app writes the Explorer right-click menu itself (src-tauri/src/explorer.rs,
; same key names); uninstalling removes it. Not during an update: the
; updater runs the uninstaller with /UPDATE, and the new version keeps it.

; Lot 7.1: "Command line in the PATH" (Settings) is removed from this user's
; PATH before the files go, by the command line itself.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} ${FileExists} "$INSTDIR\prospector-cli.exe"
    ExecWait '"$INSTDIR\prospector-cli.exe" uninstall-cleanup'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegKey HKCU "Software\Classes\Directory\shell\Prospector"
    DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\Prospector"
    DeleteRegKey HKCU "Software\Classes\Drive\shell\Prospector"
  ${EndIf}
!macroend
