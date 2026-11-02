; Inno Setup Script for LosslessSwitcher For Apple Music (Win)
#define MyAppName "LosslessSwitcher For Apple Music"
#define MyAppShortName "AMWin-LosslessSwitcher"
#define MyAppVersion "0.3.0"
#define MyAppPublisher "Pratama Putra"
#define MyAppURL "https://github.com/pratama1246/AMWin-LosslessSwitcher"
#define MyAppExeName "lossless-switcher-am.exe"

[Setup]
AppId={{9F5B2F17-8D2A-4D3C-B974-91E3618D3C89}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppShortName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=AMWin-LosslessSwitcher-Setup-v{#MyAppVersion}
SetupIconFile=..\assets\app.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
UninstallDisplayIcon={app}\{#MyAppExeName}
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupicon"; Description: "Launch automatically on Windows startup (minimized to system tray)"; GroupDescription: "Preferences:"

[InstallDelete]
Type: filesandordirs; Name: "{userappdata}\LosslessSwitcher-AM"
Type: filesandordirs; Name: "{userappdata}\AMWin-LosslessSwitcher"
Type: filesandordirs; Name: "{localappdata}\LosslessSwitcher-AM"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\LosslessSwitcher-AM"
Type: filesandordirs; Name: "{userappdata}\AMWin-LosslessSwitcher"
Type: filesandordirs; Name: "{localappdata}\LosslessSwitcher-AM"
Type: filesandordirs; Name: "{localappdata}\AMWin-LosslessSwitcher"

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\app.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--minimized"; IconFilename: "{app}\app.ico"; Tasks: startupicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  // Terminate any running instance of the app before installing
  Exec('taskkill.exe', '/F /IM lossless-switcher-am.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
  begin
    // Wipe all old caches and settings during install to ensure zero residual conflict
    DelTree(ExpandConstant('{userappdata}\LosslessSwitcher-AM'), True, True, True);
    DelTree(ExpandConstant('{userappdata}\AMWin-LosslessSwitcher'), True, True, True);
    DelTree(ExpandConstant('{localappdata}\LosslessSwitcher-AM'), True, True, True);
  end;
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  // Terminate any running instance of the app before uninstalling
  Exec('taskkill.exe', '/F /IM lossless-switcher-am.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Clean up all app data on uninstall
    DelTree(ExpandConstant('{userappdata}\LosslessSwitcher-AM'), True, True, True);
    DelTree(ExpandConstant('{userappdata}\AMWin-LosslessSwitcher'), True, True, True);
    DelTree(ExpandConstant('{localappdata}\LosslessSwitcher-AM'), True, True, True);
    DelTree(ExpandConstant('{localappdata}\AMWin-LosslessSwitcher'), True, True, True);
  end;
end;
