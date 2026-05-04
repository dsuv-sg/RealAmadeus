using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using UnityEngine;

/// <summary>
/// Manages bundled Python services (TTS/STT/Embedding) as child processes.
/// Provides auto-start/stop lifecycle so everything stays inside the application.
/// </summary>
public class EmbeddedServiceManager : MonoBehaviour
{
    public static EmbeddedServiceManager Instance { get; private set; }

    [Header("Python Executable")]
    [Tooltip("Name of python command on PATH, or relative path to bundled python.")]
    [SerializeField] private string pythonExecutableName = "python";

    [Header("Service Scripts (relative to repo root)")]
    [SerializeField] private string ttsScriptPath = "python/tts_server.py";
    [SerializeField] private string sttScriptPath = "python/stt_server.py";
    [SerializeField] private string embeddingScriptPath = "python/embedding_server.py";

    [Header("Ports")]
    [SerializeField] private int ttsPort = 5100;
    [SerializeField] private int sttPort = 5101;
    [SerializeField] private int embeddingPort = 5102;

    [Header("Auto Start on Launch")]
    [SerializeField] private bool autoStartTTS = true;
    [SerializeField] private bool autoStartSTT = true;
    [SerializeField] private bool autoStartEmbedding = true;

    // Internal state
    private Dictionary<string, Process> runningProcesses = new Dictionary<string, Process>();
    private string resolvedPythonPath;
    private string repoRoot;

    public bool IsPythonAvailable => !string.IsNullOrEmpty(resolvedPythonPath);

    void Awake()
    {
        if (Instance != null && Instance != this)
        {
            Destroy(gameObject);
            return;
        }
        Instance = this;
        DontDestroyOnLoad(gameObject);

        ResolveRepoRoot();
        ResolvePythonPath();
    }

    void Start()
    {
        if (!IsPythonAvailable)
        {
            UnityEngine.Debug.LogWarning("[EmbeddedServiceManager] Python not found. Services requiring local Python will not start.");
            return;
        }

        UnityEngine.Debug.Log("[EmbeddedServiceManager] Python available but server-based TTS/STT/RAG have been disabled. Native services only.");
    }

    void OnApplicationQuit()
    {
        StopAllServices();
    }

    void OnDestroy()
    {
        StopAllServices();
    }

    #region Path Resolution

    private void ResolveRepoRoot()
    {
        string dataPath = Application.dataPath;
        DirectoryInfo dir = new DirectoryInfo(dataPath);

        // Editor: .../RealAmadeusUnity/Assets
        // Standalone: .../RealAmadeusUnity_Data
        if (dir.Name.Equals("Assets", StringComparison.OrdinalIgnoreCase) || dir.Name.EndsWith("_Data", StringComparison.OrdinalIgnoreCase))
        {
            if (dir.Parent != null)
                repoRoot = dir.Parent.FullName;
        }
        else
        {
            repoRoot = dataPath;
        }

        // Validate by looking for python folder
        if (!Directory.Exists(Path.Combine(repoRoot, "python")))
        {
            string cwd = Directory.GetCurrentDirectory();
            if (Directory.Exists(Path.Combine(cwd, "python")))
                repoRoot = cwd;
        }

        UnityEngine.Debug.Log($"[EmbeddedServiceManager] Repo root resolved to: {repoRoot}");
    }

    private void ResolvePythonPath()
    {
        // 1. Bundled python (future-proofing)
        string bundled = Path.Combine(repoRoot, "python_embed", "python.exe");
        if (File.Exists(bundled))
        {
            resolvedPythonPath = bundled;
            UnityEngine.Debug.Log($"[EmbeddedServiceManager] Using bundled python: {resolvedPythonPath}");
            return;
        }

        // 2. python on PATH
        if (CheckCommand(pythonExecutableName, "--version"))
        {
            resolvedPythonPath = pythonExecutableName;
            UnityEngine.Debug.Log($"[EmbeddedServiceManager] Using system python: {resolvedPythonPath}");
            return;
        }

        // 3. py launcher on Windows
        if (Application.platform == RuntimePlatform.WindowsPlayer || Application.platform == RuntimePlatform.WindowsEditor)
        {
            if (CheckCommand("py", "--version"))
            {
                resolvedPythonPath = "py";
                UnityEngine.Debug.Log($"[EmbeddedServiceManager] Using py launcher: {resolvedPythonPath}");
                return;
            }
        }

        // 4. python3 on Unix-like
        if (CheckCommand("python3", "--version"))
        {
            resolvedPythonPath = "python3";
            UnityEngine.Debug.Log($"[EmbeddedServiceManager] Using python3: {resolvedPythonPath}");
            return;
        }

        UnityEngine.Debug.LogWarning("[EmbeddedServiceManager] Python executable not found. Please install Python or bundle it with the application.");
    }

    private bool CheckCommand(string fileName, string arguments)
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = fileName,
                Arguments = arguments,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using (var proc = Process.Start(psi))
            {
                if (proc == null) return false;
                proc.WaitForExit(5000);
                return proc.ExitCode == 0;
            }
        }
        catch
        {
            return false;
        }
    }

    #endregion

    #region Service Lifecycle

    public void StartService(string serviceName)
    {
        if (!IsPythonAvailable)
        {
            UnityEngine.Debug.LogWarning($"[EmbeddedServiceManager] Cannot start {serviceName}: Python not available.");
            return;
        }

        if (runningProcesses.ContainsKey(serviceName) && !runningProcesses[serviceName].HasExited)
        {
            UnityEngine.Debug.Log($"[EmbeddedServiceManager] {serviceName} is already running.");
            return;
        }

        string scriptRelative = GetScriptPath(serviceName);
        int port = GetPort(serviceName);
        if (string.IsNullOrEmpty(scriptRelative) || port <= 0)
        {
            UnityEngine.Debug.LogWarning($"[EmbeddedServiceManager] Unknown service: {serviceName}");
            return;
        }

        string fullScriptPath = Path.Combine(repoRoot, scriptRelative);
        if (!File.Exists(fullScriptPath))
        {
            UnityEngine.Debug.LogWarning($"[EmbeddedServiceManager] Script not found: {fullScriptPath}");
            return;
        }

        string args = $"\"{fullScriptPath}\" --host 127.0.0.1 --port {port}";
        var psi = new ProcessStartInfo
        {
            FileName = resolvedPythonPath,
            Arguments = args,
            WorkingDirectory = Path.GetDirectoryName(fullScriptPath),
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };

        // Environment overrides
        if (PlayerPrefs.GetInt("Config_LowSpecMode", 0) == 1)
        {
            psi.EnvironmentVariables["REALAMADEUS_TTS_DEVICE"] = "cpu";
        }

        try
        {
            var proc = Process.Start(psi);
            if (proc != null)
            {
                runningProcesses[serviceName] = proc;
                UnityEngine.Debug.Log($"[EmbeddedServiceManager] Started {serviceName} on port {port}");

                proc.OutputDataReceived += (sender, e) =>
                {
                    if (!string.IsNullOrEmpty(e.Data))
                        UnityEngine.Debug.Log($"[{serviceName}] {e.Data}");
                };
                proc.ErrorDataReceived += (sender, e) =>
                {
                    if (!string.IsNullOrEmpty(e.Data))
                        UnityEngine.Debug.LogWarning($"[{serviceName}] {e.Data}");
                };
                proc.BeginOutputReadLine();
                proc.BeginErrorReadLine();
            }
        }
        catch (Exception ex)
        {
            UnityEngine.Debug.LogError($"[EmbeddedServiceManager] Failed to start {serviceName}: {ex.Message}");
        }
    }

    public void StopService(string serviceName)
    {
        if (runningProcesses.TryGetValue(serviceName, out var proc))
        {
            try
            {
                if (!proc.HasExited)
                {
                    proc.Kill();
                    proc.WaitForExit(5000);
                }
            }
            catch (Exception ex)
            {
                UnityEngine.Debug.LogWarning($"[EmbeddedServiceManager] Error stopping {serviceName}: {ex.Message}");
            }
            runningProcesses.Remove(serviceName);
            UnityEngine.Debug.Log($"[EmbeddedServiceManager] Stopped {serviceName}");
        }
    }

    public void StopAllServices()
    {
        foreach (var kvp in new Dictionary<string, Process>(runningProcesses))
        {
            StopService(kvp.Key);
        }
    }

    public bool IsServiceRunning(string serviceName)
    {
        return runningProcesses.TryGetValue(serviceName, out var proc) && !proc.HasExited;
    }

    private string GetScriptPath(string serviceName)
    {
        switch (serviceName)
        {
            case "tts": return ttsScriptPath;
            case "stt": return sttScriptPath;
            case "embedding": return embeddingScriptPath;
            default: return null;
        }
    }

    private int GetPort(string serviceName)
    {
        switch (serviceName)
        {
            case "tts": return ttsPort;
            case "stt": return sttPort;
            case "embedding": return embeddingPort;
            default: return 0;
        }
    }

    #endregion
}
