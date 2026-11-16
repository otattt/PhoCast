; PhoCast All-in-One Installer Script
; Bundles PhoCast Client and the Native OBS Studio Plugin into a single setup.exe

#define MyAppName "PhoCast"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "PhoCast Team"
#define MyAppURL "https://github.com/otattt/PhoCast"
#define MyAppExeName "PhoCast.exe"

[Setup]
AppId={{D37E6924-4B2E-4D61-A79B-7C0094B1B3C0}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=PhoCast-Setup-v1.0.0
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
SetupIconFile=icon.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
; Main Executable
Source: "..\client-pc\rust\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Native OBS Studio Plugin DLL (installed to app folder)
Source: "..\obs-plugin\target\release\phocast_obs.dll"; DestDir: "{app}\obs-plugin"; Flags: ignoreversion
; Native OBS Studio Plugin DLL (installed directly into OBS Studio 64-bit plugins directory if OBS exists)
Source: "..\obs-plugin\target\release\phocast_obs.dll"; DestDir: "{autopf}\obs-studio\obs-plugins\64bit"; Flags: ignoreversion uninsremovereadonly; Check: DirExists(ExpandConstant('{autopf}\obs-studio\obs-plugins\64bit'))
; Driver Cleaner Tool
Source: "..\clean_driver.bat"; DestDir: "{app}"; Flags: ignoreversion
; Documentation
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; Clean up old DirectShow driver DLLs from previous versions
Type: files; Name: "{app}\driver\*"
Type: dirifempty; Name: "{app}\driver"

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; Completely remove any leftover DirectShow Virtual Camera registry keys that caused crashes/green screens
Filename: "reg.exe"; Parameters: "delete ""HKLM\Software\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{5C2CD55C-92AD-4999-8666-912BD3E70010}"" /f"; Flags: runhidden
Filename: "reg.exe"; Parameters: "delete ""HKLM\Software\Classes\CLSID\{5C2CD55C-92AD-4999-8666-912BD3E70010}"" /f"; Flags: runhidden
Filename: "reg.exe"; Parameters: "delete ""HKLM\Software\Classes\WOW6432Node\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{5C2CD55C-92AD-4999-8666-912BD3E70010}"" /f"; Flags: runhidden
Filename: "reg.exe"; Parameters: "delete ""HKLM\Software\Classes\WOW6432Node\CLSID\{5C2CD55C-92AD-4999-8666-912BD3E70010}"" /f"; Flags: runhidden
; Launch app after install
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Remove plugin from OBS Studio on uninstall if installed
Filename: "cmd.exe"; Parameters: "/c del /f /q ""{autopf}\obs-studio\obs-plugins\64bit\phocast_obs.dll"""; Flags: runhidden
