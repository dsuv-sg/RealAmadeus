using System;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.EventSystems;

public class WindowController : MonoBehaviour, IPointerDownHandler
{
    private const int GWL_STYLE = -16;

    private const int WS_BORDER      = 0x00800000;
    private const int WS_CAPTION     = 0x00C00000;
    private const int WS_SYSMENU     = 0x00080000;
    private const int WS_THICKFRAME  = 0x00040000;
    private const int WS_MINIMIZEBOX = 0x00020000;

    private const int WM_NCLBUTTONDOWN = 0xA1;
    private const int HT_CAPTION       = 0x2;

    private const int SWP_NOMOVE       = 0x0002;
    private const int SWP_NOSIZE       = 0x0001;
    private const int SWP_FRAMECHANGED = 0x0020;
    private const int SWP_NOACTIVATE   = 0x0010;

    private const int SW_SHOWMINIMIZED = 2;
    private const int SW_SHOWMAXIMIZED = 3;

    private static readonly IntPtr HWND_BOTTOM = new IntPtr(1);
    private static readonly IntPtr HWND_NOTOPMOST = new IntPtr(-2);

    [DllImport("user32.dll")] static extern IntPtr GetActiveWindow();
    [DllImport("user32.dll")] static extern int GetWindowLong(IntPtr hWnd, int nIndex);
    [DllImport("user32.dll")] static extern int SetWindowLong(IntPtr hWnd, int nIndex, int dwNewLong);
    [DllImport("user32.dll")]
    static extern bool SetWindowPos(
        IntPtr hWnd, IntPtr hWndInsertAfter,
        int X, int Y, int cx, int cy, uint uFlags);

    [DllImport("user32.dll")] static extern bool ReleaseCapture();
    [DllImport("user32.dll")] static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    private bool _wasMinimized = false;
    private int _lastDesktopBackgroundValue = -1;

    void Awake()
    {
        Application.targetFrameRate = 60; // Limit FPS to reduce GPU usage
#if !UNITY_EDITOR
        ApplyBorderlessStyle();
        ApplyDesktopBackgroundMode();
#endif
    }

    void Update()
    {
#if !UNITY_EDITOR
        int current = PlayerPrefs.GetInt("Config_Experimental_DesktopBackgroundMode", 0);
        if (current != _lastDesktopBackgroundValue)
        {
            ApplyDesktopBackgroundMode();
        }
#endif
    }

#if !UNITY_EDITOR
    private void ApplyBorderlessStyle()
    {
        IntPtr hwnd = GetActiveWindow();
        int style = GetWindowLong(hwnd, GWL_STYLE);
        style &= ~(WS_BORDER | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX);
        SetWindowLong(hwnd, GWL_STYLE, style);
        SetWindowPos(hwnd, IntPtr.Zero, 0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED);
    }

    private void OnApplicationFocus(bool hasFocus)
    {
        if (hasFocus && _wasMinimized)
        {
            _wasMinimized = false;
            IntPtr hwnd = GetActiveWindow();
            ShowWindow(hwnd, SW_SHOWMAXIMIZED);
            ApplyBorderlessStyle();
            ApplyDesktopBackgroundMode();
        }
    }
#endif

#if !UNITY_EDITOR
    private void ApplyDesktopBackgroundMode()
    {
        _lastDesktopBackgroundValue = PlayerPrefs.GetInt("Config_Experimental_DesktopBackgroundMode", 0);
        IntPtr hwnd = GetActiveWindow();
        bool desktopMode = _lastDesktopBackgroundValue == 1;
        uint flags = SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED | SWP_NOACTIVATE;
        SetWindowPos(hwnd, desktopMode ? HWND_BOTTOM : HWND_NOTOPMOST, 0, 0, 0, 0, flags);
        Application.runInBackground = desktopMode;
    }
#endif

    public void OnPointerDown(PointerEventData eventData)
    {
#if !UNITY_EDITOR
        ReleaseCapture();
        SendMessage(GetActiveWindow(), WM_NCLBUTTONDOWN, HT_CAPTION, 0);
#endif
    }

    public void OnMinimize()
    {
#if !UNITY_EDITOR
        _wasMinimized = true;
        ShowWindow(GetActiveWindow(), SW_SHOWMINIMIZED);
#endif
    }

    public void OnClose()
    {
        Application.Quit();
    }
}
