using System;
using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.InputSystem;

public class HelpPanelController : MonoBehaviour
{
    [Header("UI References")]
    public CanvasGroup panelCanvasGroup;
    public Button closeButton;
    public ScrollRect contentScrollRect;

    [Header("Settings")]
    public float fadeDuration = 0.3f;

    private Coroutine currentFadeCoroutine;
    private Action onCloseCallback;

    public bool IsActive => gameObject.activeSelf;

    void Awake()
    {
        if (panelCanvasGroup == null) panelCanvasGroup = GetComponent<CanvasGroup>();
        if (contentScrollRect == null) contentScrollRect = GetComponentInChildren<ScrollRect>();

        if (panelCanvasGroup != null)
        {
            panelCanvasGroup.alpha = 0f;
            panelCanvasGroup.interactable = false;
            panelCanvasGroup.blocksRaycasts = false;
        }
        gameObject.SetActive(false);

        if (closeButton == null)
        {
            Transform btn = transform.Find("Btn_Close");
            if (btn != null) closeButton = btn.GetComponent<Button>();
        }

        if (closeButton) closeButton.onClick.AddListener(OnCloseClicked);
    }

    void Update()
    {
        if (!IsActive) return;

        // Handle Backspace to close
        if (Keyboard.current != null && Keyboard.current.backspaceKey.wasPressedThisFrame)
        {
            OnCloseClicked();
        }
    }

    public void Show(Action onClose = null)
    {
        onCloseCallback = onClose;
        gameObject.SetActive(true);
        
        // Reset scroll position
        if (contentScrollRect != null)
        {
            contentScrollRect.verticalNormalizedPosition = 1f;
        }

        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(0f, 1f));
    }

    public void Hide()
    {
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(1f, 0f, true));
        onCloseCallback?.Invoke();
    }

    private void OnCloseClicked()
    {
        Hide();
    }

    private IEnumerator FadeCanvas(float start, float end, bool disableOnFinish = false)
    {
        float t = 0f;
        if (panelCanvasGroup)
        {
            panelCanvasGroup.alpha = start;
            panelCanvasGroup.blocksRaycasts = (end > 0.5f);
            panelCanvasGroup.interactable = (end > 0.5f);
        }

        while (t < fadeDuration)
        {
            t += Time.deltaTime;
            if (panelCanvasGroup) panelCanvasGroup.alpha = Mathf.Lerp(start, end, t / fadeDuration);
            yield return null;
        }

        if (panelCanvasGroup) panelCanvasGroup.alpha = end;
        if (disableOnFinish) gameObject.SetActive(false);
    }

    /// <summary>
    /// Refreshes the text content based on the current language setting.
    /// </summary>
    public void UpdateLanguage()
    {
        bool isEn = PlayerPrefs.GetInt("Config_Language", 0) == 1;

        // Translation Map
        var map = new System.Collections.Generic.Dictionary<string, string>
        {
            { "閉じる", "Close" },
            { "Tab / 右クリック", "Tab / Right Click" },
            { "メニュー開閉", "Open/Close Menu" },
            { "保存して閉じる", "Apply and Close" },
            { "WASD / ↑←↓→", "WASD / Arrows" },
            { "項目選択", "Navigate Items" },
            { "決定 / 会話を進める", "Select / Advance Conv." }
        };

        var texts = GetComponentsInChildren<TMPro.TextMeshProUGUI>(true);
        foreach (var tmp in texts)
        {
            if (tmp == null || string.IsNullOrEmpty(tmp.text)) continue;

            string currentText = Normalize(tmp.text);

            if (isEn)
            {
                foreach (var kv in map)
                {
                    if (currentText == Normalize(kv.Key))
                    {
                        tmp.text = kv.Value;
                        break;
                    }
                }
            }
            else
            {
                foreach (var kv in map)
                {
                    if (currentText == Normalize(kv.Value))
                    {
                        tmp.text = kv.Key;
                        break;
                    }
                }
            }
        }
    }

    private string Normalize(string s)
    {
        if (s == null) return "";
        return s.Replace("\r\n", "\n").Replace("\r", "\n").Trim();
    }
}
