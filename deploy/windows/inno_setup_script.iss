[Setup]
AppName=EIDOLON
AppVersion=1.1.0
AppPublisher=TUFTA
AppCopyright=Copyright (C) 2026 TUFTA

DefaultDirName={localappdata}\Programs\EIDOLON
DefaultGroupName=EIDOLON
PrivilegesRequired=lowest

SetupIconFile=C:\EIDOLON_Release\icon.ico
UninstallDisplayIcon={app}\EIDOLON.exe
WizardStyle=modern
DisableWelcomePage=no
LicenseFile=C:\EIDOLON_Release\manifesto.txt
WizardImageFile=C:\EIDOLON_Release\side_image.bmp
WizardSmallImageFile=C:\EIDOLON_Release\top_logo.bmp

OutputDir=C:\EIDOLON_Release
OutputBaseFilename=EIDOLON_Setup_v1.1
Compression=lzma2/ultra64
SolidCompression=yes

[Files]
Source: "C:\EIDOLON_Release\EIDOLON.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\EIDOLON_Release\ffmpeg.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\EIDOLON_Release\yt-dlp.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\EIDOLON"; Filename: "{app}\EIDOLON.exe"
Name: "{group}\Uninstall EIDOLON"; Filename: "{uninstallexe}"
Name: "{autodesktop}\EIDOLON"; Filename: "{app}\EIDOLON.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a Desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\EIDOLON.exe"; Description: "Launch EIDOLON System"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{userappdata}\eidolon_config.txt"
Type: filesandordirs; Name: "{%TEMP}\eidolon_audio"
Type: files; Name: "{%TEMP}\eidolon_ytdlp.mp4"