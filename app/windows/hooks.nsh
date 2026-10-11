; Repo Launcher's additions to Tauri's NSIS uninstaller (DECISIONS D-041). Updates run the old
; uninstaller with /UPDATE; nothing here happens then.
;
; Before the files go: `repo-launcher.exe --uninstall` removes the shortcuts Repo Launcher made,
; its keychain entries and its start-at-login entry (it reads the list from its own database,
; which "Delete the application data" removes next).
; After the files go, if that box was ticked: a copy of the program deletes whatever was left
; of the data (Go marks its module cache read-only) and the WebView's folders.

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --uninstall'
    ${If} $DeleteAppDataCheckboxState = 1
      InitPluginsDir
      CopyFiles /SILENT "$INSTDIR\${MAINBINARYNAME}.exe" "$PLUGINSDIR\repo-launcher-cleanup.exe"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} $DeleteAppDataCheckboxState = 1
    ExecWait '"$PLUGINSDIR\repo-launcher-cleanup.exe" --uninstall --delete-data'
  ${EndIf}
!macroend
