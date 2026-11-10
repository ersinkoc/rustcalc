#ifndef AppVersion
  #error AppVersion must be supplied by package.ps1
#endif

[Setup]
AppId=RustCalc.Desktop
AppName=RustCalc
AppVersion={#AppVersion}
AppPublisher=Ersin Koc
AppPublisherURL=https://github.com/ersinkoc/rustcalc
AppUpdatesURL=https://github.com/ersinkoc/rustcalc/releases
AppSupportURL=https://github.com/ersinkoc/rustcalc/issues
DefaultDirName={localappdata}\Programs\RustCalc
DefaultGroupName=RustCalc
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
MinVersion=10.0
#if TargetArch == "aarch64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif
SetupIconFile=..\assets\rustcalc.ico
UninstallDisplayIcon={app}\bin\rustcalc.exe
OutputBaseFilename=rustcalc-v{#AppVersion}-windows-{#TargetArch}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern dynamic
VersionInfoVersion={#AppVersion}
CloseApplications=yes
RestartApplications=no
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[CustomMessages]
english.AddToPath=Add RustCalc to my PATH
turkish.AddToPath=RustCalc uygulamasını PATH değişkenime ekle
english.StartMenu=Create a Start menu shortcut
turkish.StartMenu=Başlat menüsü kısayolu oluştur
english.Launch=Open RustCalc
turkish.Launch=RustCalc uygulamasını aç
english.DirectoryConflict=Choose an empty folder or the folder of an existing RustCalc installation.
turkish.DirectoryConflict=Boş bir klasör veya mevcut bir RustCalc kurulumunun klasörünü seçin.
english.DirectoryChanged=Use the registered RustCalc folder, or uninstall it before choosing a different folder.
turkish.DirectoryChanged=Kayıtlı RustCalc klasörünü kullanın veya başka bir klasör seçmeden önce uygulamayı kaldırın.

[Tasks]
Name: "startmenu"; Description: "{cm:StartMenu}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked
Name: "userpath"; Description: "{cm:AddToPath}"; Flags: unchecked

[Files]
Source: "{#AppBinary}"; DestDir: "{app}\bin"; DestName: "rustcalc.exe"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "RustCalc-LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\RustCalc"; Filename: "{app}\bin\rustcalc.exe"; WorkingDir: "{app}"; Tasks: startmenu
Name: "{userdesktop}\RustCalc"; Filename: "{app}\bin\rustcalc.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\bin\rustcalc.exe"; Description: "{cm:Launch}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\bin\rustcalc.previous.exe"
Type: files; Name: "{app}\bin\rustcalc.exe.update-lock"
Type: dirifempty; Name: "{app}\bin"

[Code]
const
  OwnershipKey = 'Software\RustCalc\Desktop';
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\RustCalc.Desktop_is1';

var
  LegacyInstall: Boolean;

function IsLegacyDirectory(Dir: String): Boolean;
var
  Marker: AnsiString;
begin
  Result := FileExists(Dir + '\bin\rustcalc.exe') and
    FileExists(Dir + '\uninstall.ps1') and
    LoadStringFromFile(Dir + '\install.json', Marker) and
    (Pos('"RustCalc"', String(Marker)) > 0);
end;

function DirectoryHasContents(Dir: String): Boolean;
var
  Entry: TFindRec;
begin
  Result := False;
  if FindFirst(AddBackslash(Dir) + '*', Entry) then
    try
      repeat
        if (Entry.Name <> '.') and (Entry.Name <> '..') then begin
          Result := True;
          break;
        end;
      until not FindNext(Entry);
    finally
      FindClose(Entry);
    end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Dir, Registered: String;
begin
  Dir := ExpandConstant('{app}');
  Result := '';
  if RegQueryStringValue(HKCU, UninstallKey, 'InstallLocation', Registered) then begin
    if CompareText(RemoveBackslashUnlessRoot(Registered), RemoveBackslashUnlessRoot(Dir)) <> 0 then
      Result := CustomMessage('DirectoryChanged');
    exit;
  end;
  LegacyInstall := IsLegacyDirectory(Dir);
  if DirectoryHasContents(Dir) and not LegacyInstall then
    Result := CustomMessage('DirectoryConflict');
end;

function RemovePathEntry(Existing, Bin: String; var Found: Boolean): String;
var
  Part: String;
  Separator: Integer;
begin
  Result := '';
  Found := False;
  while Existing <> '' do begin
    Separator := Pos(';', Existing);
    if Separator = 0 then Separator := Length(Existing) + 1;
    Part := Copy(Existing, 1, Separator - 1);
    Delete(Existing, 1, Separator);
    if CompareText(RemoveBackslashUnlessRoot(Trim(Part)), Bin) = 0 then
      Found := True
    else if Part <> '' then begin
      if Result <> '' then Result := Result + ';';
      Result := Result + Part;
    end;
  end;
end;

procedure CurStepChanged(Step: TSetupStep);
var
  Existing, Bin, Clean, Owned: String;
  Found: Boolean;
begin
  if Step <> ssDone then exit;
  Bin := ExpandConstant('{app}\bin');
  RegQueryStringValue(HKCU, 'Environment', 'Path', Existing);
  Clean := RemovePathEntry(Existing, Bin, Found);
  if WizardIsTaskSelected('userpath') and not Found then begin
    if Clean <> '' then Clean := Clean + ';';
    if RegWriteExpandStringValue(HKCU, 'Environment', 'Path', Clean + Bin) then
      RegWriteStringValue(HKCU, OwnershipKey, 'OwnedPath', Bin);
  end else if LegacyInstall and Found then
    RegWriteStringValue(HKCU, OwnershipKey, 'OwnedPath', Bin)
  else if not WizardIsTaskSelected('userpath') and
      RegQueryStringValue(HKCU, OwnershipKey, 'OwnedPath', Owned) and
      (CompareText(Owned, Bin) = 0) then begin
    if not Found or RegWriteExpandStringValue(HKCU, 'Environment', 'Path', Clean) then
      RegDeleteValue(HKCU, OwnershipKey, 'OwnedPath');
  end;
  if LegacyInstall then begin
    DeleteFile(ExpandConstant('{app}\install.json'));
    DeleteFile(ExpandConstant('{app}\uninstall.ps1'));
  end;
end;

procedure CurUninstallStepChanged(Step: TUninstallStep);
var
  Bin, Owned, Existing, Clean: String;
  Found: Boolean;
begin
  if Step <> usPostUninstall then exit;
  Bin := ExpandConstant('{app}\bin');
  if RegQueryStringValue(HKCU, OwnershipKey, 'OwnedPath', Owned) and (CompareText(Owned, Bin) = 0) then begin
    RegQueryStringValue(HKCU, 'Environment', 'Path', Existing);
    Clean := RemovePathEntry(Existing, Bin, Found);
    if not Found or RegWriteExpandStringValue(HKCU, 'Environment', 'Path', Clean) then
      RegDeleteValue(HKCU, OwnershipKey, 'OwnedPath');
    RegDeleteKeyIfEmpty(HKCU, OwnershipKey);
  end;
end;
