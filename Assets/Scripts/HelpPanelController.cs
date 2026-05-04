using System;
using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.InputSystem;
using TMPro;

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
            Transform btn = FindDeepChild(transform, "Btn_Close");
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
        UpdateLanguage();
        
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

    public void UpdateLanguage()
    {
        int langIdx = PlayerPrefs.GetInt("Config_Language", 0);

        string getTr(string ja, string en, string zh, string ko, string es, string fr, string de, string ru)
        {
            if (langIdx == 1) return en;
            if (langIdx == 2) return zh;
            if (langIdx == 3) return ko;
            if (langIdx == 4) return es;
            if (langIdx == 5) return fr;
            if (langIdx == 6) return de;
            if (langIdx == 7) return ru;
            return ja;
        }

        string Close_JA = "閉じる", Close_EN = "Close", Close_ZH = "关闭", Close_KO = "닫기", Close_ES = "Cerrar", Close_FR = "Fermer", Close_DE = "Schließen", Close_RU = "Закрыть";

        var map = new System.Collections.Generic.Dictionary<string, string>();
        void AddVariants(string translated, params string[] variants)
        {
            foreach (var variant in variants)
            {
                string key = Normalize(variant);
                if (!map.ContainsKey(key))
                {
                    map[key] = translated;
                }
            }
        }

        AddVariants(
            getTr("Tab / 右クリック", "Tab / Right Click", "Tab / 右键", "Tab / 우클릭", "Tab / Clic derecho", "Tab / Clic droit", "Tab / Rechtsklick", "Tab / Правый клик"),
            "Tab / 右クリック", "Tab / Right Click", "Tab / 右键", "Tab / 우클릭", "Tab / Clic derecho", "Tab / Clic droit", "Tab / Rechtsklick", "Tab / Правый клик"
        );
        AddVariants(
            getTr("メニュー開閉", "Toggle Menu", "打开/关闭菜单", "메뉴 열기/닫기", "Abrir/cerrar menú", "Ouvrir/fermer menu", "Menü umschalten", "Открыть/закрыть меню"),
            "メニュー開閉", "Toggle Menu", "打开/关闭菜单", "메뉴 열기/닫기", "Abrir/cerrar menú", "Ouvrir/fermer menu", "Menü umschalten", "Открыть/закрыть меню"
        );
        AddVariants(
            getTr("保存して閉じる", "Save and Close", "保存并关闭", "저장하고 닫기", "Guardar y cerrar", "Sauvegarder et fermer", "Speichern und schließen", "Сохранить и закрыть"),
            "保存して閉じる", "Save and Close", "保存并关闭", "저장하고 닫기", "Guardar y cerrar", "Sauvegarder et fermer", "Speichern und schließen", "Сохранить и закрыть"
        );
        AddVariants(
            getTr("項目選択", "Select Item", "选择项目", "항목 선택", "Seleccionar elemento", "Sélectionner", "Element auswählen", "Выбор элемента"),
            "項目選択", "Select Item", "选择项目", "항목 선택", "Seleccionar elemento", "Sélectionner", "Element auswählen", "Выбор элемента"
        );
        AddVariants(
            getTr("決定 / 会話を進める", "Confirm / Advance", "确认 / 推进对话", "결정 / 대화 진행", "Confirmar / Avanzar", "Confirmer / Avancer", "Bestätigen / Fortfahren", "Подтвердить / Продолжить"),
            "決定 / 会話を進める", "Confirm / Advance", "确认 / 推进对话", "결정 / 대화 진행", "Confirmar / Avanzar", "Confirmer / Avancer", "Bestätigen / Fortfahren", "Подтвердить / Продолжить"
        );
        AddVariants(
            getTr("保存して閉じる", "Apply and Close", "保存并关闭", "저장 후 닫기", "Guardar y cerrar", "Enregistrer et fermer", "Speichern und schließen", "Сохранить и закрыть"),
            "保存して閉じる", "Apply and Close", "Save and Close", "保存并关闭", "저장 후 닫기", "Guardar y cerrar", "Enregistrer et fermer", "Speichern und schließen", "Сохранить и закрыть"
        );
        AddVariants(
            getTr("WASD / ↑←↓→", "WASD / Arrows", "WASD / 方向键", "WASD / 화살표", "WASD / Flechas", "WASD / Flèches", "WASD / Pfeiltasten", "WASD / Стрелки"),
            "WASD / ↑←↓→", "WASD / Arrows", "WASD / 方向键", "WASD / 화살표", "WASD / Flechas", "WASD / Flèches", "WASD / Pfeiltasten", "WASD / Стрелки"
        );
        AddVariants(
            getTr("項目選択", "Navigate Items", "导航项目", "항목 탐색", "Navegar elementos", "Naviguer entre les éléments", "Elemente navigieren", "Навигация по пунктам"),
            "項目選択", "Navigate Items", "Select Item", "导航项目", "항목 탐색", "Navegar elementos", "Naviguer entre les éléments", "Elemente navigieren", "Навигация по пунктам"
        );
        AddVariants(
            getTr("決定 / 会話を進める", "Select / Advance Conv.", "确认 / 推进对话", "확인 / 대화 진행", "Seleccionar / Avanzar", "Sélectionner / Avancer", "Auswählen / Fortfahren", "Выбрать / Продолжить"),
            "決定 / 会話を進める", "Select / Advance Conv.", "Confirm / Advance", "确认 / 推进对话", "확인 / 대화 진행", "Seleccionar / Avanzar", "Sélectionner / Avancer", "Auswählen / Fortfahren", "Выбрать / Продолжить"
        );

        var texts = GetComponentsInChildren<TMPro.TextMeshProUGUI>(true);
        foreach (var tmp in texts)
        {
            if (tmp == null || string.IsNullOrEmpty(tmp.text)) continue;

            string currentText = Normalize(tmp.text);

            foreach (var kv in map)
            {
                if (currentText == Normalize(kv.Key))
                {
                    tmp.text = GetSpacingTaggedText(kv.Value, langIdx);
                    break;
                }
            }
        }

        var closeButtonText = ResolveCloseButtonText();
        if (closeButtonText != null)
        {
            closeButtonText.text = GetSpacingTaggedText(getTr(Close_JA, Close_EN, Close_ZH, Close_KO, Close_ES, Close_FR, Close_DE, Close_RU), langIdx);
        }
    }

    private TextMeshProUGUI ResolveCloseButtonText()
    {
        if (closeButton == null)
        {
            Transform btn = FindDeepChild(transform, "Btn_Close");
            if (btn != null) closeButton = btn.GetComponent<Button>();
        }

        if (closeButton == null) return null;
        return closeButton.GetComponentInChildren<TextMeshProUGUI>(true);
    }

    private Transform FindDeepChild(Transform parent, string targetName)
    {
        if (parent == null) return null;
        if (parent.name == targetName) return parent;

        for (int i = 0; i < parent.childCount; i++)
        {
            Transform found = FindDeepChild(parent.GetChild(i), targetName);
            if (found != null) return found;
        }

        return null;
    }

    private string GetSpacingTaggedText(string text, int lang)
    {
        if (lang != 7 || string.IsNullOrEmpty(text)) return text;
        return System.Text.RegularExpressions.Regex.Replace(text, @"([\u0400-\u04FF]+)", "<cspace=-8.4px>$1</cspace>");
    }

    private string Normalize(string s)
    {
        if (s == null) return "";
        string normalized = s.Replace("\r\n", "\n").Replace("\r", "\n");
        normalized = System.Text.RegularExpressions.Regex.Replace(normalized, "<.*?>", string.Empty);
        return normalized.Trim();
    }
}
