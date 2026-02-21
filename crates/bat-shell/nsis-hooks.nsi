; Add install dir to user PATH on install
!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCmp $0 "" 0 +3
    WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR"
    Goto done_path
  WriteRegExpandStr HKCU "Environment" "Path" "$0;$INSTDIR"
  done_path:
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=500
!macroend
