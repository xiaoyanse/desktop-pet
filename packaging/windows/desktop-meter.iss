#ifndef ReleaseVersion
  #error ReleaseVersion is required
#endif
#ifndef PackageDir
  #error PackageDir is required
#endif
#ifndef OutputPath
  #error OutputPath is required
#endif

[Setup]
AppId={{57F879EB-06E4-44F2-8CC5-A990C358CD0E}
AppName=桌面工作助理
AppVersion={#ReleaseVersion}
AppPublisher=曜沧软件
AppPublisherURL=https://github.com/xiaoyanse/desktop-pet
AppSupportURL=https://gitee.com/xiaoyanse/desktop-pet
AppUpdatesURL=https://github.com/xiaoyanse/desktop-pet/releases
DefaultDirName={code:DefaultInstallDirectory}
DefaultGroupName=桌面工作助理
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputPath}
OutputBaseFilename=DesktopMeter-{#ReleaseVersion}-setup-win-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
DisableDirPage=no
UninstallFilesDir={app}\uninstall
UninstallDisplayIcon={app}\current\desktop-meter.exe
CloseApplications=no
RestartApplications=no
SetupLogging=yes

[Languages]
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; Flags: unchecked

[Files]
Source: "{#PackageDir}\desktop-meter.exe"; DestDir: "{app}\current"; Flags: ignoreversion
Source: "{#PackageDir}\README.md"; DestDir: "{app}\current"; Flags: ignoreversion
Source: "{#PackageDir}\启动设置.cmd"; DestDir: "{app}\current"; Flags: ignoreversion
Source: "{#PackageDir}\manifest.json"; DestDir: "{app}\current"; Flags: ignoreversion

[Icons]
Name: "{group}\桌面工作助理"; Filename: "{app}\current\desktop-meter.exe"
Name: "{group}\工作台设置"; Filename: "{app}\current\desktop-meter.exe"; Parameters: "--settings"
Name: "{autodesktop}\桌面工作助理"; Filename: "{app}\current\desktop-meter.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\current\desktop-meter.exe"; Description: "启动桌面工作助理"; Flags: postinstall nowait skipifsilent unchecked

[Code]
function DefaultInstallDirectory(Param: String): String;
begin
  if DirExists('E:\') then Result := 'E:\Apps\DesktopMeter'
  else if DirExists('D:\') then Result := 'D:\Apps\DesktopMeter'
  else Result := ExpandConstant('{localappdata}\Programs\DesktopMeter');
end;
