using System;
using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;
using Debug = UnityEngine.Debug;

public static class DesktopNotificationService
{
    private static object notifyIcon;
    private static Type notifyIconType;
    private static bool setupAttempted;

    // ─── Win32 Focus Detection ───
    [DllImport("user32.dll", SetLastError = true)] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", SetLastError = true)] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("kernel32.dll")] static extern uint GetCurrentProcessId();
    [DllImport("user32.dll")] static extern IntPtr GetActiveWindow();

    // ─── Win32 Tray Notification ───
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] static extern bool Shell_NotifyIcon(uint dwMessage, ref NOTIFYICONDATA lpData);
    [DllImport("user32.dll")] static extern IntPtr LoadIcon(IntPtr hInstance, IntPtr lpIconName);
    [DllImport("user32.dll")] static extern bool DestroyIcon(IntPtr hIcon);

    const uint NIM_ADD = 0x00000000;
    const uint NIM_MODIFY = 0x00000001;
    const uint NIM_DELETE = 0x00000002;
    const uint NIF_ICON = 0x00000002;
    const uint NIF_MESSAGE = 0x00000001;
    const uint NIF_TIP = 0x00000004;
    const uint NIF_INFO = 0x00000010;
    static readonly IntPtr IDI_INFORMATION = new IntPtr(0x7F04);
    const uint NIIF_INFO = 0x00000001;
    const uint NIIF_NOSOUND = 0x00000010;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct NOTIFYICONDATA
    {
        public uint cbSize;
        public IntPtr hWnd;
        public uint uID;
        public uint uFlags;
        public uint uCallbackMessage;
        public IntPtr hIcon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szTip;
        public uint dwState;
        public uint dwStateMask;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string szInfo;
        public uint uVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string szInfoTitle;
        public uint dwInfoFlags;
        public Guid guidItem;
        public IntPtr hBalloonIcon;
    }

    /// <summary>
    /// Returns true if any window from this process is currently in the foreground.
    /// Uses Win32 API for reliable detection even with borderless windows.
    /// </summary>
    public static bool IsForegroundWindow()
    {
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        try
        {
            IntPtr fgWindow = GetForegroundWindow();
            if (fgWindow == IntPtr.Zero) return false;
            GetWindowThreadProcessId(fgWindow, out uint fgProcessId);
            bool isFg = fgProcessId == GetCurrentProcessId();
            return isFg;
        }
        catch
        {
            return Application.isFocused;
        }
#else
        return Application.isFocused;
#endif
    }

    public static void Show(string title, string message)
    {
        if (PlayerPrefs.GetInt("Config_DesktopNotifications", 1) != 1)
        {
            return;
        }

        // Only notify when window is not active (same behavior as QT version)
        if (IsForegroundWindow())
        {
            return;
        }

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        try
        {
            string safeTitle = Trim(title, 63);
            string safeMessage = Trim(message, 255);

            // Try multiple notification methods in order of reliability
            bool shown = ShowViaPowerShell(safeTitle, safeMessage);
            if (!shown) shown = ShowViaSystemWindowsForms(safeTitle, safeMessage);
            if (!shown) ShowViaWin32(safeTitle, safeMessage);
        }
        catch (Exception ex)
        {
            UnityEngine.Debug.LogWarning($"[DesktopNotification] Failed to show toast: {ex.Message}");
        }
#endif
    }

    // ─── System.Windows.Forms path (original) ───
    private static bool ShowViaSystemWindowsForms(string title, string message)
    {
        try
        {
            if (!EnsureNotifyIcon()) return false;

            SetProperty("Visible", true);
            SetProperty("BalloonTipTitle", title);
            SetProperty("BalloonTipText", message);

            MethodInfo showMethod = notifyIconType.GetMethod("ShowBalloonTip", new[] { typeof(int) });
            showMethod?.Invoke(notifyIcon, new object[] { 3000 });
            return true;
        }
        catch (Exception ex)
        {
            UnityEngine.Debug.LogWarning($"[DesktopNotification] System.Windows.Forms path failed: {ex.Message}");
            return false;
        }
    }

    // ─── PowerShell path (most reliable on Win10/11) ───
    private static bool ShowViaPowerShell(string title, string message)
    {
        try
        {
            // Escape single quotes for PowerShell
            string psTitle = title.Replace("'", "''");
            string psMessage = message.Replace("'", "''").Replace("\r", " ").Replace("\n", " ");

            string command = $@"Add-Type -AssemblyName System.Windows.Forms; $n = New-Object System.Windows.Forms.NotifyIcon; $n.Icon = [System.Drawing.SystemIcons]::Information; $n.BalloonTipTitle = '{psTitle}'; $n.BalloonTipText = '{psMessage}'; $n.Visible = $True; $n.ShowBalloonTip(3000); Start-Sleep -Milliseconds 4000; $n.Dispose()";

            var psi = new ProcessStartInfo
            {
                FileName = "powershell.exe",
                Arguments = $"-WindowStyle Hidden -Command \"{command}\"",
                CreateNoWindow = true,
                UseShellExecute = false,
                WindowStyle = ProcessWindowStyle.Hidden
            };

            using (Process proc = Process.Start(psi))
            {
                // Fire-and-forget; don't wait for exit
            }
            return true;
        }
        catch (Exception ex)
        {
            UnityEngine.Debug.LogWarning($"[DesktopNotification] PowerShell path failed: {ex.Message}");
            return false;
        }
    }

    // ─── Win32 Shell_NotifyIcon path (fallback) ───
    private static void ShowViaWin32(string title, string message)
    {
        try
        {
            IntPtr hwnd = GetActiveWindow();
            if (hwnd == IntPtr.Zero) hwnd = Process.GetCurrentProcess().MainWindowHandle;
            if (hwnd == IntPtr.Zero)
            {
                UnityEngine.Debug.LogWarning("[DesktopNotification] No window handle available for Win32 notification");
                return;
            }

            IntPtr hIcon = LoadIcon(IntPtr.Zero, IDI_INFORMATION);
            if (hIcon == IntPtr.Zero)
            {
                UnityEngine.Debug.LogWarning("[DesktopNotification] Failed to load info icon");
            }

            uint id = (uint)title.GetHashCode();

            NOTIFYICONDATA nid = new NOTIFYICONDATA();
            nid.cbSize = (uint)Marshal.SizeOf(nid);
            nid.hWnd = hwnd;
            nid.uID = id;
            nid.uFlags = NIF_INFO | NIF_ICON | NIF_TIP;
            nid.hIcon = hIcon;
            nid.szTip = "Real Amadeus";
            nid.szInfo = message;
            nid.szInfoTitle = title;
            nid.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND;
            nid.uVersion = 4; // Vista+

            // Add icon
            if (!Shell_NotifyIcon(NIM_ADD, ref nid))
            {
                UnityEngine.Debug.LogWarning("[DesktopNotification] Shell_NotifyIcon ADD failed");
            }

            // Show balloon (modify)
            if (!Shell_NotifyIcon(NIM_MODIFY, ref nid))
            {
                UnityEngine.Debug.LogWarning("[DesktopNotification] Shell_NotifyIcon MODIFY (balloon) failed");
            }

            // Cleanup icon after short delay via coroutine is tricky from static class,
            // so we just delete immediately. The balloon usually still shows because
            // Windows buffers the notification.
            Shell_NotifyIcon(NIM_DELETE, ref nid);

            if (hIcon != IntPtr.Zero) DestroyIcon(hIcon);
        }
        catch (Exception ex)
        {
            UnityEngine.Debug.LogWarning($"[DesktopNotification] Win32 path failed: {ex.Message}");
        }
    }

    private static bool EnsureNotifyIcon()
    {
        if (notifyIcon != null && notifyIconType != null) return true;
        if (setupAttempted) return false;
        setupAttempted = true;

        try
        {
            Assembly formsAssembly = Assembly.Load("System.Windows.Forms");
            if (formsAssembly == null) return false;

            notifyIconType = formsAssembly.GetType("System.Windows.Forms.NotifyIcon");
            if (notifyIconType == null) return false;

            notifyIcon = Activator.CreateInstance(notifyIconType);
            if (notifyIcon == null) return false;

            SetProperty("Text", "Real Amadeus");
            TrySetDefaultIcon();
            return true;
        }
        catch (Exception ex)
        {
            UnityEngine.Debug.LogWarning($"[DesktopNotification] Failed to init NotifyIcon: {ex.Message}");
            return false;
        }
    }

    private static void TrySetDefaultIcon()
    {
        try
        {
            Assembly drawingAssembly = Assembly.Load("System.Drawing");
            Type systemIconsType = drawingAssembly?.GetType("System.Drawing.SystemIcons");
            PropertyInfo infoProp = systemIconsType?.GetProperty("Information", BindingFlags.Public | BindingFlags.Static);
            object infoIcon = infoProp?.GetValue(null);
            SetProperty("Icon", infoIcon);
        }
        catch
        {
            // Ignore icon setup failures.
        }
    }

    private static void SetProperty(string name, object value)
    {
        notifyIconType?.GetProperty(name)?.SetValue(notifyIcon, value);
    }

    private static string Trim(string text, int maxLen)
    {
        if (string.IsNullOrEmpty(text)) return string.Empty;
        string collapsed = text.Replace("\r", " ").Replace("\n", " ").Trim();
        if (collapsed.Length <= maxLen) return collapsed;
        return collapsed.Substring(0, maxLen - 1) + "…";
    }
}
