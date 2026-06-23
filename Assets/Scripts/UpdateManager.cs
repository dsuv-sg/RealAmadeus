using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using UnityEngine;
using UnityEngine.Networking;
using Debug = UnityEngine.Debug;

/// <summary>
/// Automatic update manager (V1.3U port of QT V1.3Q UpdateManager).
///
/// 1. Checks GitHub Releases API for the latest version tag.
/// 2. Compares with <see cref="Application.version"/>.
/// 3. Raises <see cref="OnUpdateCheckCompleted"/> so the UI can prompt the user.
/// 4. On <see cref="StartUpdate"/>: writes a localized PowerShell bootstrap
///    script to disk and spawns it (Windows). The script waits for the
///    Unity EXE to exit, downloads the release ZIP, extracts it, copies the
///    files into the install directory, and relaunches the app.
///
/// Parity with QT V1.3Q updatemanager.cpp + updater.ps1.
/// </summary>
public class UpdateManager : MonoBehaviour
{
    public static UpdateManager Instance { get; private set; }

    private const string API_URL = "https://api.github.com/repos/dsuv-sg/RealAmadeus/releases/latest";
    private const string DEFAULT_RELEASE_PAGE = "https://github.com/dsuv-sg/RealAmadeus/releases/latest";
    private const string UPDATER_SCRIPT_RESOURCE = "updater_script"; // updater_script.txt under Resources/
    private const string UPDATER_SCRIPT_FILENAME = "updater.ps1";

    [SerializeField] private float timeoutSeconds = 10f;

    public bool IsChecking { get; private set; }
    public bool HasUpdate { get; private set; }
    public string CurrentVersion { get; private set; }
    public string LatestVersion { get; private set; }
    public string ReleasePageUrl { get; private set; } = DEFAULT_RELEASE_PAGE;
    public string DownloadUrl { get; private set; }
    public long AssetSize { get; private set; }
    public string ChangelogExcerpt { get; private set; }

    public event Action OnUpdateCheckFinished;
    public event Action<bool, bool> OnUpdateCheckCompleted;

    public static UpdateManager GetOrCreate()
    {
        if (Instance != null) return Instance;
        var go = new GameObject("[UpdateManager]");
        DontDestroyOnLoad(go);
        return go.AddComponent<UpdateManager>();
    }

    void Awake()
    {
        if (Instance != null && Instance != this) { Destroy(gameObject); return; }
        Instance = this;
        DontDestroyOnLoad(gameObject);
        CurrentVersion = string.IsNullOrEmpty(Application.version) ? "0.1.0" : Application.version;
    }

    void OnDestroy()
    {
        CancelCheck();
    }

    public void CancelCheck()
    {
        try
        {
            StopAllCoroutines();
            IsChecking = false;
        }
        catch { /* Ignore */ }
    }

    public void CheckForUpdate()
    {
        if (IsChecking) return;
        StartCoroutine(CheckForUpdateRoutine());
    }

    private System.Collections.IEnumerator CheckForUpdateRoutine()
    {
        IsChecking = true;
        Debug.Log("[UpdateManager] Checking for updates at " + API_URL);

        using (UnityWebRequest request = UnityWebRequest.Get(API_URL))
        {
            request.timeout = Mathf.CeilToInt(timeoutSeconds);
            request.SetRequestHeader("Accept", "application/vnd.github+json");
            request.SetRequestHeader("User-Agent", "RealAmadeusUnity-Updater/" + CurrentVersion);

            yield return request.SendWebRequest();
            IsChecking = false;

            if (request.result != UnityWebRequest.Result.Success)
            {
                Debug.LogWarning("[UpdateManager] GitHub API request failed: " + request.error);
                HasUpdate = false;
                OnUpdateCheckCompleted?.Invoke(false, false);
                OnUpdateCheckFinished?.Invoke();
                yield break;
            }

            try
            {
                ParseResponse(request.downloadHandler.text);
                HasUpdate = CompareVersions(CurrentVersion, LatestVersion);
                Debug.Log("[UpdateManager] Current=" + CurrentVersion + " Latest=" + LatestVersion + " HasUpdate=" + HasUpdate);
            }
            catch (Exception ex)
            {
                Debug.LogWarning("[UpdateManager] Parse error: " + ex.Message);
                HasUpdate = false;
                OnUpdateCheckCompleted?.Invoke(false, false);
                OnUpdateCheckFinished?.Invoke();
                yield break;
            }
        }

        OnUpdateCheckCompleted?.Invoke(true, HasUpdate);
        OnUpdateCheckFinished?.Invoke();
    }

    private void ParseResponse(string json)
    {
        if (string.IsNullOrEmpty(json)) return;

        var root = MiniJson.Deserialize(json) as System.Collections.Generic.Dictionary<string, object>;
        if (root == null) return;

        if (root.TryGetValue("tag_name", out object tagObj) && tagObj != null)
        {
            LatestVersion = CleanVersion(tagObj.ToString());
            if (LatestVersion.Length > 0 && !char.IsLetter(LatestVersion[0]))
            {
                LatestVersion = "V" + LatestVersion;
            }
        }

        if (root.TryGetValue("html_url", out object htmlObj) && htmlObj != null)
        {
            ReleasePageUrl = htmlObj.ToString();
            DownloadUrl = ReleasePageUrl;
        }

        if (root.TryGetValue("body", out object bodyObj) && bodyObj != null)
        {
            string body = bodyObj.ToString();
            ChangelogExcerpt = body.Length > 600 ? body.Substring(0, 600) + "..." : body;
        }

        if (root.TryGetValue("assets", out object assetsObj) && assetsObj is System.Collections.Generic.List<object> assets)
        {
            foreach (var item in assets)
            {
                if (item is System.Collections.Generic.Dictionary<string, object> asset)
                {
                    string name = asset.TryGetValue("name", out object nameObj) && nameObj != null ? nameObj.ToString() : "";
                    if (name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase) &&
                        asset.TryGetValue("browser_download_url", out object urlObj) && urlObj != null)
                    {
                        DownloadUrl = urlObj.ToString();
                        if (asset.TryGetValue("size", out object sizeObj) && sizeObj != null)
                        {
                            long parsed;
                            if (long.TryParse(sizeObj.ToString(), out parsed)) AssetSize = parsed;
                        }
                        break;
                    }
                }
            }
        }
    }

    public static string CleanVersion(string version)
    {
        if (string.IsNullOrEmpty(version)) return "0.0.0";
        int firstDigit = -1;
        for (int i = 0; i < version.Length; i++)
        {
            if (char.IsDigit(version[i])) { firstDigit = i; break; }
        }
        if (firstDigit < 0) return "0.0.0";

        int length = 0;
        for (int i = firstDigit; i < version.Length; i++)
        {
            if (char.IsDigit(version[i]) || version[i] == '.')
            {
                length++;
            }
            else
            {
                break;
            }
        }
        return version.Substring(firstDigit, length);
    }

    public static bool CompareVersions(string current, string latest)
    {
        if (string.IsNullOrEmpty(current) || string.IsNullOrEmpty(latest)) return false;
        Version cVer, lVer;
        if (!TryParseVersion(CleanVersion(current), out cVer)) return true;
        if (!TryParseVersion(CleanVersion(latest), out lVer)) return false;
        return lVer > cVer;
    }

    private static bool TryParseVersion(string s, out Version v)
    {
        s = CleanVersion(s);
        int dash = s.IndexOf('-');
        if (dash >= 0) s = s.Substring(0, dash);
        v = null;
        int dotCount = 0;
        for (int i = 0; i < s.Length; i++) if (s[i] == '.') dotCount++;
        while (dotCount < 3) { s += ".0"; dotCount++; }
        return Version.TryParse(s, out v);
    }

    /// <summary>
    /// QT-parity: write the PowerShell updater script to disk and launch it.
    /// Falls back to opening the release page if no ZIP asset URL is available
    /// or if PowerShell cannot be spawned.
    /// </summary>
    public bool StartUpdate(int langIndex = 0)
    {
        if (string.IsNullOrEmpty(DownloadUrl))
        {
            Debug.LogWarning("[UpdateManager] Update URL is empty");
            return false;
        }

        // If no ZIP asset URL was found, fall back to opening the release page.
        bool isZipUrl = DownloadUrl.EndsWith(".zip", StringComparison.OrdinalIgnoreCase)
                         || DownloadUrl.Contains("/releases/download/");
        if (!isZipUrl)
        {
            OpenReleasePage();
            return false;
        }

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        // Locate the install directory and EXE name.
        string appDir = LocateInstallDirectory();
        if (string.IsNullOrEmpty(appDir))
        {
            Debug.LogWarning("[UpdateManager] Could not determine install directory; falling back to browser.");
            OpenReleasePage();
            return false;
        }
        string exeName = LocateExecutableName(appDir);
        string scriptPath = Path.Combine(appDir, UPDATER_SCRIPT_FILENAME);

        // Load the PowerShell script from Resources.
        TextAsset scriptAsset = Resources.Load<TextAsset>(UPDATER_SCRIPT_RESOURCE);
        if (scriptAsset == null)
        {
            Debug.LogWarning("[UpdateManager] updater_script asset not found in Resources; falling back to browser.");
            OpenReleasePage();
            return false;
        }

        // Write the script with a UTF-8 BOM so Japanese characters survive in PowerShell.
        try
        {
            byte[] bom = new byte[] { 0xEF, 0xBB, 0xBF };
            byte[] body = Encoding.UTF8.GetBytes(scriptAsset.text);
            using (var fs = new FileStream(scriptPath, FileMode.Create, FileAccess.Write, FileShare.None))
            {
                fs.Write(bom, 0, bom.Length);
                fs.Write(body, 0, body.Length);
            }
            Debug.Log("[UpdateManager] Wrote updater script: " + scriptPath);
        }
        catch (Exception ex)
        {
            Debug.LogWarning("[UpdateManager] Failed to write updater script: " + ex.Message);
            OpenReleasePage();
            return false;
        }

        // Launch PowerShell with the updater arguments (mirrors QT QProcess::startDetached).
        int parentPid = Process.GetCurrentProcess().Id;
        var psi = new ProcessStartInfo
        {
            FileName = "powershell.exe",
            Arguments = string.Format(
                "-ExecutionPolicy Bypass -File \"{0}\" -ParentPid {1} -DownloadUrl \"{2}\" -AssetSize {3} -AppDir \"{4}\" -ExeName \"{5}\" -LangIndex {6}",
                scriptPath, parentPid, DownloadUrl, AssetSize, appDir, exeName, Mathf.Clamp(langIndex, 0, 10)),
            UseShellExecute = false,
            CreateNoWindow = true,
        };

        try
        {
            var proc = Process.Start(psi);
            if (proc == null)
            {
                Debug.LogWarning("[UpdateManager] PowerShell failed to start; falling back to browser.");
                OpenReleasePage();
                return false;
            }
            Debug.Log("[UpdateManager] PowerShell updater started, PID=" + proc.Id);
            // Hand off to the updater and exit. QT calls QCoreApplication::quit() here.
            Application.Quit();
            return true;
        }
        catch (Exception ex)
        {
            Debug.LogWarning("[UpdateManager] Failed to launch PowerShell: " + ex.Message);
            OpenReleasePage();
            return false;
        }
#elif UNITY_STANDALONE_OSX || UNITY_STANDALONE_LINUX || UNITY_EDITOR_OSX || UNITY_EDITOR_LINUX
        string appDir = LocateInstallDirectory();
        if (string.IsNullOrEmpty(appDir))
        {
            Debug.LogWarning("[UpdateManager] Could not determine install directory; falling back to browser.");
            OpenReleasePage();
            return false;
        }
        string exeName = LocateExecutableName(appDir);
        string scriptPath = "/tmp/realamadeus_updater.sh";

        string title = GetTitleTranslation(langIndex);
        string completed = GetCompletedTranslation(langIndex).Replace("\\n", "\n");

        try
        {
            string bashScript = "#!/bin/sh\n" + string.Format(@"
PARENT_PID=$1
DOWNLOAD_URL=$2
APP_DIR=$3
EXE_NAME=$4
OS_TYPE=$5

TITLE=""{0}""
COMPLETED=""{1}""

if [ ""$OS_TYPE"" = ""osx"" ]; then
    osascript -e 'display notification ""Downloading update..."" with title ""'$TITLE'""'
else
    if command -v notify-send >/dev/null 2>&1; then
        notify-send ""$TITLE"" ""Downloading update...""
    fi
fi

if [ -n ""$PARENT_PID"" ] && [ ""$PARENT_PID"" -gt 0 ]; then
    echo ""Waiting for parent process $PARENT_PID to exit...""
    while kill -0 $PARENT_PID 2>/dev/null; do
        sleep 1
    done
fi

TEMP_DIR=""/tmp/realamadeus_update""
rm -rf ""$TEMP_DIR""
mkdir -p ""$TEMP_DIR""
TEMP_ZIP=""$TEMP_DIR/update.zip""

echo ""Downloading update...""
if command -v curl >/dev/null 2>&1; then
    curl -L -o ""$TEMP_ZIP"" ""$DOWNLOAD_URL""
elif command -v wget >/dev/null 2>&1; then
    wget -O ""$TEMP_ZIP"" ""$DOWNLOAD_URL""
else
    echo ""Error: curl or wget is required.""
    exit 1
fi

echo ""Extracting...""
TEMP_EXTRACT=""$TEMP_DIR/extract""
mkdir -p ""$TEMP_EXTRACT""
unzip -o ""$TEMP_ZIP"" -d ""$TEMP_EXTRACT""

SOURCE_DIR=""$TEMP_EXTRACT""
ITEM_COUNT=$(ls -1 ""$TEMP_EXTRACT"" | wc -l)
if [ ""$ITEM_COUNT"" -eq 1 ] && [ -d ""$(ls -d ""$TEMP_EXTRACT""/*)"" ]; then
    SOURCE_DIR=""$(ls -d ""$TEMP_EXTRACT""/*)""
fi

if [ ""$OS_TYPE"" = ""osx"" ]; then
    if [ -d ""$SOURCE_DIR/RealAmadeus.app"" ]; then
        REAL_APP_PATH=$(cd ""$APP_DIR/../../.."" && pwd)
        REAL_APP_PARENT=$(dirname ""$REAL_APP_PATH"")
        echo ""Replacing app bundle at $REAL_APP_PATH...""
        rm -rf ""$REAL_APP_PATH""
        cp -R ""$SOURCE_DIR/RealAmadeus.app"" ""$REAL_APP_PARENT/""
        chmod +x ""$REAL_APP_PATH/Contents/MacOS/$EXE_NAME""
        
        osascript -e 'display dialog ""'$COMPLETED'"" with title ""'$TITLE'"" buttons {{""OK""}} default button ""OK"" with icon note'
        open -a ""$REAL_APP_PATH""
    else
        REAL_APP_PATH=$(cd ""$APP_DIR/../../.."" && pwd)
        cp -R ""$SOURCE_DIR/""* ""$REAL_APP_PATH/""
        chmod +x ""$APP_DIR/$EXE_NAME""
        
        osascript -e 'display dialog ""'$COMPLETED'"" with title ""'$TITLE'"" buttons {{""OK""}} default button ""OK"" with icon note'
        open -a ""$REAL_APP_PATH""
    fi
else
    echo ""Copying files to $APP_DIR...""
    cp -R ""$SOURCE_DIR/""* ""$APP_DIR/""
    chmod +x ""$APP_DIR/$EXE_NAME""
    
    if command -v zenity >/dev/null 2>&1; then
        zenity --info --title=""$TITLE"" --text=""$COMPLETED"" --width=350
    elif command -v notify-send >/dev/null 2>&1; then
        notify-send ""$TITLE"" ""$COMPLETED""
    fi
    ""$APP_DIR/$EXE_NAME"" &
fi

rm -rf ""$TEMP_DIR""
", title.Replace("\"", "\\\""), completed.Replace("\"", "\\\""));

            File.WriteAllText(scriptPath, bashScript, new UTF8Encoding(false));
            
            // Make the script executable
            var chmodPsi = new ProcessStartInfo
            {
                FileName = "chmod",
                Arguments = "+x \"" + scriptPath + "\"",
                UseShellExecute = false,
                CreateNoWindow = true
            };
            var procChmod = Process.Start(chmodPsi);
            procChmod?.WaitForExit();
            Debug.Log("[UpdateManager] Wrote bash updater script: " + scriptPath);
        }
        catch (Exception ex)
        {
            Debug.LogWarning("[UpdateManager] Failed to write updater script: " + ex.Message);
            OpenReleasePage();
            return false;
        }

        int parentPid = Process.GetCurrentProcess().Id;
        string osType = "linux";
#if UNITY_STANDALONE_OSX || UNITY_EDITOR_OSX
        osType = "osx";
#endif

        var psi = new ProcessStartInfo
        {
            FileName = "/bin/sh",
            Arguments = string.Format("\"{0}\" {1} \"{2}\" \"{3}\" \"{4}\" {5}",
                scriptPath, parentPid, DownloadUrl, appDir, exeName, osType),
            UseShellExecute = false,
            CreateNoWindow = true,
        };

        try
        {
            var proc = Process.Start(psi);
            if (proc == null)
            {
                Debug.LogWarning("[UpdateManager] Bash script failed to start; falling back to browser.");
                OpenReleasePage();
                return false;
            }
            Debug.Log("[UpdateManager] Bash updater started, PID=" + proc.Id);
            Application.Quit();
            return true;
        }
        catch (Exception ex)
        {
            Debug.LogWarning("[UpdateManager] Failed to launch bash script: " + ex.Message);
            OpenReleasePage();
            return false;
        }
#else
        // Non-desktop platforms: fall back to opening the release page.
        Debug.Log("[UpdateManager] Auto updater is desktop-only; opening release page.");
        OpenReleasePage();
        return false;
#endif
    }

    /// <summary>
    /// Locate the install directory of the running executable.
    /// In a built player on Windows this is the folder containing the EXE.
    /// </summary>
    private static string LocateInstallDirectory()
    {
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN || UNITY_STANDALONE_OSX || UNITY_STANDALONE_LINUX || UNITY_EDITOR_OSX || UNITY_EDITOR_LINUX
        try
        {
            string mainModule = Process.GetCurrentProcess().MainModule.FileName;
            if (!string.IsNullOrEmpty(mainModule))
            {
                return Path.GetDirectoryName(mainModule);
            }
        }
        catch { /* fall through */ }
#endif
        // Editor / fallback: use Application.dataPath (..) location.
        try
        {
            string dataPath = Application.dataPath;
            if (!string.IsNullOrEmpty(dataPath))
            {
                return Path.GetDirectoryName(dataPath);
            }
        }
        catch { /* ignore */ }
        return null;
    }

    /// <summary>
    /// Locate the executable name. Falls back to Application.productName if
    /// the running module name cannot be retrieved (e.g. editor).
    /// </summary>
    private static string LocateExecutableName(string appDir)
    {
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN || UNITY_STANDALONE_OSX || UNITY_STANDALONE_LINUX || UNITY_EDITOR_OSX || UNITY_EDITOR_LINUX
        try
        {
            string mainModule = Process.GetCurrentProcess().MainModule.FileName;
            if (!string.IsNullOrEmpty(mainModule))
            {
                return Path.GetFileName(mainModule);
            }
        }
        catch { /* fall through */ }
#endif
        if (!string.IsNullOrEmpty(Application.productName))
        {
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
            return Application.productName + ".exe";
#else
            return Application.productName;
#endif
        }
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        return "RealAmadeus.exe";
#else
        return "RealAmadeus";
#endif
    }

    public void OpenReleasePage()
    {
        string url = !string.IsNullOrEmpty(DownloadUrl) ? DownloadUrl : ReleasePageUrl;
        if (string.IsNullOrEmpty(url)) url = DEFAULT_RELEASE_PAGE;
        try { Application.OpenURL(url); }
        catch (Exception ex) { Debug.LogWarning("[UpdateManager] Failed to open release page: " + ex.Message); }
    }

    private static string GetTitleTranslation(int lang)
    {
        switch (lang)
        {
            case 0: return "RealAmadeus アップデーター";
            case 2: return "RealAmadeus 更新程序";
            case 3: return "RealAmadeus 업데이터";
            case 4: return "RealAmadeus Actualizador";
            case 5: return "Mise à jour RealAmadeus";
            case 6: return "RealAmadeus Updater";
            case 7: return "Обновление RealAmadeus";
            case 8: return "Оновлення RealAmadeus";
            case 9: return "Atualizador RealAmadeus";
            case 10: return "RealAmadeus Güncelleyici";
            default: return "RealAmadeus Updater";
        }
    }

    private static string GetCompletedTranslation(int lang)
    {
        switch (lang)
        {
            case 0: return "RealAmadeus の更新が完了しました。\\nOK を押すとアプリを再起動します。";
            case 2: return "RealAmadeus 已成功更新。\\n点击确定重启应用程序。";
            case 3: return "RealAmadeus 업데이트가 완료되었습니다.\\n확인을 누르면 앱이 다시 시작됩니다.";
            case 4: return "RealAmadeus se ha actualizado correctamente.\\nHaga clic en Aceptar para reiniciar la aplicación.";
            case 5: return "RealAmadeus a été mis à jour avec succès.\\nCliquez sur OK pour redémarrer l'application.";
            case 6: return "RealAmadeus wurde erfolgreich aktualisiert.\\nKlicken Sie auf OK, um die Anwendung neu zu starten.";
            case 7: return "RealAmadeus успешно обновлён.\\nНажмите OK, чтобы перезапустить приложение.";
            case 8: return "RealAmadeus успішно оновлено.\\nНатисніть OK, щоб перезапустити додаток.";
            case 9: return "O RealAmadeus foi atualizado com sucesso.\\nClique em OK para reiniciar o aplicativo.";
            case 10: return "RealAmadeus başarıyla güncellendi.\\nUygulamayı yeniden başlatmak için Tamam'a tıklayın.";
            default: return "RealAmadeus has been updated successfully.\\nClick OK to restart the application.";
        }
    }
}
