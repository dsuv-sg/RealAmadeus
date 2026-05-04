using System;
using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.InputSystem;
using TMPro;

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

        // Map English to all other languages (0=JA, 1=EN, 2=ZH, 3=KO, 4=ES, 5=FR, 6=DE, 7=RU)
        
        string Close_JA = "閉じる";
        string Close_EN = "Close";
        string Close_ZH = "关闭";
        string Close_KO = "닫기";
        string Close_ES = "Cerrar";
        string Close_FR = "Fermer";
        string Close_DE = "Schließen";
        string Close_RU = "Закрыть";

        string V12_JA = "中国語/韓国語/スペイン語/フランス語/ドイツ語/ロシア語 のサポートを追加しました。\n視線トラッキングを追加しました。\nデスクトップ通知機能を実装しました。\n軽量化モードを追加しました。\nAlt+Enter、F11 での画面モード切替を実装しました。\nAPIプロバイダーにOllamaとOpenRouterを追加しました。\nUIのバグを修正しました。\nAPIキーのセキュリティを向上させました。\nパフォーマンスを改善しました。";
        string V12_EN = "Added support for Chinese, Korean, Spanish, French, German, and Russian.\nAdded eye tracking.\nImplemented desktop notification feature.\nAdded lightweight mode.\nImplemented screen mode toggle with Alt+Enter and F11.\nAdded Ollama and OpenRouter API providers.\nFixed UI bugs.\nImproved security for API keys.\nImproved performance.";
        string V12_ZH = "新增中文/韩语/西班牙语/法语/德语/俄语支持。\n新增视线追踪。\n实现了桌面通知功能。\n新增轻量模式。\n新增 Alt+Enter、F11 画面模式切换。\n添加了Ollama和OpenRouter的API提供者。\n修复UI错误。\n提高了API密钥等的安全性。\n改善性能。";
        string V12_KO = "중국어/한국어/스페인어/프랑스어/독일어/러시아어 지원 추가.\n시선 추적 추가.\n데스크톱 알림 기능을 구현했습니다.\n경량 모드 추가.\nAlt+Enter, F11 화면 모드 전환 추가.\nOllama/OpenRouter API 제공자를 추가했습니다.\nUI 버그 수정.\nAPI 키 등의 보안을 강화했습니다.\n성능 개선.";
        string V12_ES = "Se agregó soporte para chino, coreano, español, francés, alemán y ruso.\nSe agregó seguimiento ocular.\nSe implementó la función de notificaciones de escritorio.\nSe agregó modo ligero.\nSe implementó cambio de modo de pantalla con Alt+Enter y F11.\nSe agregaron los proveedores de API Ollama y OpenRouter.\nSe corrigieron errores de UI.\nSe mejoró la seguridad de las claves API.\nSe mejoró el rendimiento.";
        string V12_FR = "Ajout du support chinois, coréen, espagnol, français, allemand et russe.\nAjout du suivi du regard.\nImplémentation de la fonction de notifications de bureau.\nAjout du mode léger.\nAjout du changement de mode d'écran avec Alt+Entrée et F11.\nAjout des fournisseurs d'API Ollama et OpenRouter.\nCorrection de bugs UI.\nAmélioration de la sécurité pour les clés API.\nAmélioration des performances.";
        string V12_DE = "Unterstützung für Chinesisch, Koreanisch, Spanisch, Französisch, Deutsch und Russisch hinzugefügt.\nBlickverfolgung hinzugefügt.\nDesktop-Benachrichtigungsfunktion implementiert.\nLeichtmodus hinzugefügt.\nBildschirmmodus-Umschaltung mit Alt+Enter und F11 hinzugefügt.\nOllama/OpenRouter API-Anbieter hinzugefügt.\nUI-Fehler behoben.\nSicherheit für API-Schlüssel verbessert.\nLeistung verbessert.";
        string V12_RU = "Добавлена поддержка китайского, корейского, испанского, французского, немецкого и русского.\nДобавлено отслеживание взгляда.\nРеализована функция уведомлений на рабочем столе.\nДобавлен облегчённый режим.\nДобавлено переключение режима экрана клавишами Alt+Enter и F11.\nДобавлены API-провайдеры Ollama и OpenRouter.\nИсправлены ошибки интерфейса.\nУлучшена безопасность для ключей API.\nУлучшена производительность.";

        string V11_JA = "メニュー画面のクリック選択を実装しました。\n日本語/英語の言語切り替えを実装しました。\n各種画面に[閉じる/キャンセル/適用]ボタンを実装しました。\nチェンジログ/バックログ用に、スクロールバーを実装しました。\n視認性の向上のため、UIの一部を変更しました。\n軽量化版のリリースを開始しました。";
        string V11_EN = "Implemented click selection for the menu screen.\nAdded language switching between Japanese and English.\nAdded [Close/Cancel/Apply] buttons to various screens.\nImplemented scrollbars for ChangeLog and BackLog.\nUpdated UI elements for better visibility.\nStarted release of the lightweight version.";
        string V11_ZH = "实现了菜单画面的点击选择。\n实现了日语/英语的语言切换。\n在各种画面上实现了[关闭/取消/应用]按钮。\n为ChangeLog/BackLog实现了滚动条。\n为提高可视性更改了部分UI。\n开始发布轻量版。";
        string V11_KO = "메뉴 화면의 클릭 선택을 구현했습니다.\n일본어/영어 언어 전환을 구현했습니다.\n각종 화면에 [닫기/취소/적용] 버튼을 구현했습니다.\nChangeLog/BackLog용 스크롤바를 구현했습니다.\n가시성 향상을 위해 UI 일부를 변경했습니다.\n경량판 릴리스를 시작했습니다.";
        string V11_ES = "Se implementó la selección por clic en la pantalla de menú.\nSe agregó cambio de idioma entre japonés e inglés.\nSe agregaron botones [Cerrar/Cancelar/Aplicar] a varias pantallas.\nSe implementaron barras de desplazamiento para ChangeLog y BackLog.\nSe actualizaron elementos de UI para mejor visibilidad.\nSe inició el lanzamiento de la versión ligera.";
        string V11_FR = "Implémentation de la sélection par clic pour le menu.\nAjout du changement de langue entre japonais et anglais.\nAjout des boutons [Fermer/Annuler/Appliquer] à divers écrans.\nImplémentation des barres de défilement pour ChangeLog et BackLog.\nMise à jour de l'UI pour une meilleure visibilité.\nDébut de la publication de la version allégée.";
        string V11_DE = "Implementierung der Klickauswahl für den Menübildschirm.\nSprachumschaltung zwischen Japanisch und Englisch hinzugefügt.\n[Schließen/Abbrechen/Anwenden]-Schaltflächen zu verschiedenen Bildschirmen hinzugefügt.\nScrollbalken für ChangeLog und BackLog implementiert.\nUI-Elemente für bessere Sichtbarkeit aktualisiert.\nVeröffentlichung der leichten Version gestartet.";
        string V11_RU = "Реализован выбор в меню щелчком мыши.\nДобавлено переключение языка между японским и английским.\nДобавлены кнопки [Закрыть/Отмена/Применить] на различные экраны.\nРеализованы полосы прокрутки для ChangeLog и BackLog.\nОбновлены элементы интерфейса для лучшей видимости.\nНачат выпуск облегчённой версии.";

        string V101_JA = "GPU使用率が異常に高くなってしまう問題を修正しました。\nフルスクリーン状態での最小化時に、ウィンドウが異常に小さくなってしまう問題を修正しました。\nログアウト後の再ログインが不可能になってしまう問題を修正しました。\n一部AIサービスの利用時にて、感情タグが表示されてしまう問題を修正しました。";
        string V101_EN = "Fixed an issue where GPU usage was abnormally high.\nFixed a window scaling issue when minimizing in fullscreen.\nFixed an issue where re-logging in after logout was impossible.\nFixed a bug where emotion tags were appearing for some AI services.";
        string V101_ZH = "修复了GPU使用率异常高的问题。\n修复了全屏状态下最小化时窗口异常变小的问题。\n修复了登出后无法重新登录的问题。\n修复了使用部分AI服务时情感标签显示的问题。";
        string V101_KO = "GPU 사용률이 비정상적으로 높아지는 문제를 수정했습니다.\n전체 화면 상태에서 최소화 시 창이 비정상적으로 작아지는 문제를 수정했습니다.\n로그아웃 후 재로그인이 불가능해지는 문제를 수정했습니다.\n일부 AI 서비스 이용 시 감정 태그가 표시되는 문제를 수정했습니다.";
        string V101_ES = "Se corrigió un problema de uso anormalmente alto de GPU.\nSe corrigió un problema de escala de ventana al minimizar en pantalla completa.\nSe corrigió un problema donde re-iniciar sesión tras cerrarla era imposible.\nSe corrigió un error donde aparecían etiquetas de emoción en algunos servicios de AI.";
        string V101_FR = "Correction d'un problème d'utilisation anormalement élevée du GPU.\nCorrection d'un problème de mise à l'échelle lors de la minimisation en plein écran.\nCorrection d'un problème empêchant la reconnexion après déconnexion.\nCorrection d'un bug faisant apparaître des tags d'émotion pour certains services AI.";
        string V101_DE = "Problem mit abnormal hoher GPU-Auslastung behoben.\nProblem mit der Fensterskalierung bei Minimierung im Vollbild behoben.\nProblem behoben, bei dem eine erneute Anmeldung nach der Abmeldung unmöglich war.\nFehler behoben, bei dem Emotionstags bei einigen KI-Diensten angezeigt wurden.";
        string V101_RU = "Исправлена проблема с аномально высоким использованием GPU.\nИсправлена проблема с масштабированием окна при сворачивании в полноэкранном режиме.\nИсправлена проблема, при которой повторный вход после выхода был невозможен.\nИсправлена ошибка, при которой теги эмоций отображались для некоторых AI-сервисов.";

        string V10_JA = "リアルアマデウスの最初のバージョンをリリースしました。\n基本的な会話機能のみを備えています。";
        string V10_EN = "Released the first version of Real Amadeus.\nIncludes basic conversation features.";
        string V10_ZH = "发布了Real Amadeus的最初版本。\n仅具备基本的对话功能。";
        string V10_KO = "Real Amadeus의 최초 버전을 릴리스했습니다.\n기본적인 대화 기능만을 갖추고 있습니다.";
        string V10_ES = "Se lanzó la primera versión de Real Amadeus.\nIncluye funciones básicas de conversación.";
        string V10_FR = "Sortie de la première version de Real Amadeus.\nComprend les fonctionnalités de conversation de base.";
        string V10_DE = "Erste Version von Real Amadeus veröffentlicht.\nEnthält nur grundlegende Konversationsfunktionen.";
        string V10_RU = "Выпущена первая версия Real Amadeus.\nВключает только базовые функции разговора.";

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

        AddVariants(getTr(V12_JA, V12_EN, V12_ZH, V12_KO, V12_ES, V12_FR, V12_DE, V12_RU),
            V12_JA, V12_EN, V12_ZH, V12_KO, V12_ES, V12_FR, V12_DE, V12_RU);
        AddVariants(getTr(V11_JA, V11_EN, V11_ZH, V11_KO, V11_ES, V11_FR, V11_DE, V11_RU),
            V11_JA, V11_EN, V11_ZH, V11_KO, V11_ES, V11_FR, V11_DE, V11_RU);
        AddVariants(getTr(V101_JA, V101_EN, V101_ZH, V101_KO, V101_ES, V101_FR, V101_DE, V101_RU),
            V101_JA, V101_EN, V101_ZH, V101_KO, V101_ES, V101_FR, V101_DE, V101_RU);
        AddVariants(getTr(V10_JA, V10_EN, V10_ZH, V10_KO, V10_ES, V10_FR, V10_DE, V10_RU),
            V10_JA, V10_EN, V10_ZH, V10_KO, V10_ES, V10_FR, V10_DE, V10_RU);

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

        // Update Close button text
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
