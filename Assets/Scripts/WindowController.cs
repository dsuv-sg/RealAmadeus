using System;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.EventSystems;

public class WindowController : MonoBehaviour, IPointerDownHandler
{
    private const int GWL_STYLE = -16;

    private const int WS_BORDER = 0x00800000;
    private const int WS_CAPTION = 0x00C00000;
    private const int WS_SYSMENU = 0x00080000;
    private const int WS_THICKFRAME = 0x00040000;
    private const int WS_MINIMIZEBOX = 0x00020000;

    private const int WM_NCLBUTTONDOWN = 0xA1;
    private const int HT_CAPTION = 0x2;

    private const int SWP_NOMOVE = 0x0002;
    private const int SWP_NOSIZE = 0x0001;
    private const int SWP_FRAMECHANGED = 0x0020;

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

    void Awake()
    {
#if !UNITY_EDITOR
        IntPtr hwnd = GetActiveWindow();

        int style = GetWindowLong(hwnd, GWL_STYLE);
        style &= ~(WS_BORDER | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX);
        SetWindowLong(hwnd, GWL_STYLE, style);

        // Åö Ç±ÇÍÇ™í¥èdóv
        SetWindowPos(hwnd, IntPtr.Zero, 0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED);
#endif
    }

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
        ShowWindow(GetActiveWindow(), 2); // SW_SHOWMINIMIZED
#endif
    }

    public void OnClose()
    {
        Application.Quit();
    }
}
