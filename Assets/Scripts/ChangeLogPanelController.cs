using System;
using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.InputSystem;

public class ChangeLogPanelController : MonoBehaviour
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
            { "メニュー画面のクリック選択を実装しました。\n日本語/英語の言語切り替えを実装しました。\n各種画面に[閉じる/キャンセル/適用]ボタンを実装しました。\nチェンジログ/バックログ用に、スクロールバーを実装しました。\n視認性の向上のため、UIの一部を変更しました。\n軽量化版のリリースを開始しました。", 
              "- Implemented click selection for the menu screen.\n- Added language switching between Japanese and English.\n- Added [Close/Cancel/Apply] buttons to various screens.\n- Implemented scrollbars for ChangeLog and BackLog.\n- Updated UI elements for better visibility.\n- Started release of the lightweight version." },
            { "GPU使用率が異常に高くなってしまう問題を修正しました。\nフルスクリーン状態での最小化時に、ウィンドウが異常に小さくなってしまう問題を修正しました。\nログアウト後の再ログインが不可能になってしまう問題を修正しました。\n一部AIサービスの利用時にて、感情タグが表示されてしまう問題を修正しました。", 
              "- Fixed an issue where GPU usage was abnormally high.\n- Fixed a window scaling issue when minimizing in fullscreen.\n- Fixed an issue where re-logging in after logout was impossible.\n- Fixed a bug where emotion tags were appearing for some AI services." },
            { "リアルアマデウスの最初のバージョンをリリースしました。\n基本的な会話機能のみを備えています。", 
              "- Released the first version of Real Amadeus.\n- Includes basic conversation features." }
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
