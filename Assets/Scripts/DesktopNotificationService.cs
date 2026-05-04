using System;
using System.Reflection;
using UnityEngine;

public static class DesktopNotificationService
{
    private static object notifyIcon;
    private static Type notifyIconType;
    private static bool setupAttempted;

    public static void Show(string title, string message)
    {
        if (PlayerPrefs.GetInt("Config_DesktopNotifications", 1) != 1)
        {
            return;
        }

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        try
        {
            if (!EnsureNotifyIcon())
            {
                return;
            }

            SetProperty("Visible", true);
            SetProperty("BalloonTipTitle", Trim(title, 64));
            SetProperty("BalloonTipText", Trim(message, 220));

            MethodInfo showMethod = notifyIconType.GetMethod("ShowBalloonTip", new[] { typeof(int) });
            showMethod?.Invoke(notifyIcon, new object[] { 3000 });
        }
        catch (Exception ex)
        {
            Debug.LogWarning($"[DesktopNotification] Failed to show toast: {ex.Message}");
        }
#endif
    }

    private static bool EnsureNotifyIcon()
    {
        if (notifyIcon != null && notifyIconType != null)
        {
            return true;
        }
        if (setupAttempted)
        {
            return false;
        }
        setupAttempted = true;

        Assembly formsAssembly = Assembly.Load("System.Windows.Forms");
        if (formsAssembly == null)
        {
            return false;
        }

        notifyIconType = formsAssembly.GetType("System.Windows.Forms.NotifyIcon");
        if (notifyIconType == null)
        {
            return false;
        }

        notifyIcon = Activator.CreateInstance(notifyIconType);
        if (notifyIcon == null)
        {
            return false;
        }

        SetProperty("Text", "Real Amadeus");
        TrySetDefaultIcon();
        return true;
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
        if (string.IsNullOrEmpty(text))
        {
            return string.Empty;
        }
        string collapsed = text.Replace("\r", " ").Replace("\n", " ").Trim();
        if (collapsed.Length <= maxLen)
        {
            return collapsed;
        }
        return collapsed.Substring(0, maxLen - 1) + "…";
    }
}
