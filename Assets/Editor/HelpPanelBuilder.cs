#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;
using TMPro;

public class HelpPanelBuilder
{
    [MenuItem("Tools/Amadeus/Create Help Panel")]
    public static void CreateHelpPanel()
    {
        Canvas canvas = GameObject.FindObjectOfType<Canvas>();
        if (canvas == null) { Debug.LogError("No Canvas found!"); return; }

        Transform existing = canvas.transform.Find("HelpPanel");
        if (existing != null) GameObject.DestroyImmediate(existing.gameObject);

        // Main Panel
        GameObject panelObj = new GameObject("HelpPanel");
        panelObj.transform.SetParent(canvas.transform, false);
        RectTransform panelRT = panelObj.AddComponent<RectTransform>();
        panelRT.anchorMin = Vector2.zero;
        panelRT.anchorMax = Vector2.one;
        panelRT.offsetMin = Vector2.zero;
        panelRT.offsetMax = Vector2.zero;

        Image panelBg = panelObj.AddComponent<Image>();
        panelBg.color = new Color(0, 0, 0, 0.95f);

        CanvasGroup cg = panelObj.AddComponent<CanvasGroup>();
        HelpPanelController controller = panelObj.AddComponent<HelpPanelController>();
        controller.panelCanvasGroup = cg;

        TMP_FontAsset font = Resources.Load<TMP_FontAsset>("Fonts & Materials/MSMINCHO SDF");

        // Layout
        VerticalLayoutGroup mainLayout = panelObj.AddComponent<VerticalLayoutGroup>();
        mainLayout.padding = new RectOffset(100, 100, 60, 60);
        mainLayout.spacing = 30;
        mainLayout.childControlHeight = true;
        mainLayout.childControlWidth = true;
        mainLayout.childForceExpandHeight = false;
        mainLayout.childForceExpandWidth = true;

        // -- Header --
        CreateHeader(panelObj.transform, "ヘルプ", font);

        // -- Scroll View --
        GameObject scrollObj = new GameObject("Scroll View");
        scrollObj.transform.SetParent(panelObj.transform, false);
        LayoutElement leScroll = scrollObj.AddComponent<LayoutElement>();
        leScroll.flexibleHeight = 1;

        ScrollRect scrollRect = scrollObj.AddComponent<ScrollRect>();
        controller.contentScrollRect = scrollRect;

        // Viewport
        GameObject viewport = new GameObject("Viewport");
        viewport.transform.SetParent(scrollObj.transform, false);
        RectTransform vpRT = viewport.AddComponent<RectTransform>();
        vpRT.anchorMin = Vector2.zero; vpRT.anchorMax = Vector2.one;
        vpRT.sizeDelta = Vector2.zero; // Stretch
        
        Image vpImg = viewport.AddComponent<Image>();
        vpImg.color = new Color(1, 1, 1, 0.05f); // Slight background
        Mask vpMask = viewport.AddComponent<Mask>();
        vpMask.showMaskGraphic = false;

        scrollRect.viewport = vpRT;

        // Content
        GameObject content = new GameObject("Content");
        content.transform.SetParent(viewport.transform, false);
        RectTransform contentRT = content.AddComponent<RectTransform>();
        contentRT.anchorMin = new Vector2(0, 1); contentRT.anchorMax = new Vector2(1, 1);
        contentRT.pivot = new Vector2(0.5f, 1);
        contentRT.sizeDelta = new Vector2(0, 0); // Height controlled by layout

        VerticalLayoutGroup contentLayout = content.AddComponent<VerticalLayoutGroup>();
        contentLayout.padding = new RectOffset(40, 40, 40, 40);
        contentLayout.spacing = 40;
        contentLayout.childControlHeight = true;
        contentLayout.childControlWidth = true;
        contentLayout.childForceExpandHeight = false;
        
        ContentSizeFitter csf = content.AddComponent<ContentSizeFitter>();
        csf.verticalFit = ContentSizeFitter.FitMode.PreferredSize;

        scrollRect.content = contentRT;
        scrollRect.vertical = true;
        scrollRect.horizontal = false;
        scrollRect.scrollSensitivity = 20;

        // -- Content Items --
        CreateHelpEntry(content.transform, "メニュー開閉", "Tab / 右クリック", font);
        CreateHelpEntry(content.transform, "保存して閉じる", "Backspace", font);
        CreateHelpEntry(content.transform, "項目選択", "WASD / ↑←↓→", font);
        CreateHelpEntry(content.transform, "選択 / 会話を進める", "Enter", font);

        // -- Footer --
        GameObject footer = new GameObject("Footer");
        footer.transform.SetParent(panelObj.transform, false);
        HorizontalLayoutGroup footerLayout = footer.AddComponent<HorizontalLayoutGroup>();
        footerLayout.childAlignment = TextAnchor.LowerRight;
        footerLayout.childControlHeight = true;
        footerLayout.childControlWidth = true;

        Button closeBtn = CreateButton(footer.transform, "CloseButton", "CLOSE", font);
        controller.closeButton = closeBtn;

        // Auto-Link
        MenuPanelController menu = GameObject.FindObjectOfType<MenuPanelController>();
        if (menu != null)
        {
             menu.GetType().GetField("helpPanelController")?.SetValue(menu, controller);
             EditorUtility.SetDirty(menu);
        }

        Debug.Log("Help Panel Created.");
    }

    static void CreateHelpEntry(Transform parent, string actionName, string keys, TMP_FontAsset font)
    {
        GameObject entry = new GameObject($"Entry_{actionName}");
        entry.transform.SetParent(parent, false);
        VerticalLayoutGroup vlg = entry.AddComponent<VerticalLayoutGroup>();
        vlg.spacing = 10;
        vlg.childControlHeight = true;
        vlg.childControlWidth = true;

        // Header Line (Keys)
        GameObject headerObj = new GameObject("HeaderLine");
        headerObj.transform.SetParent(entry.transform, false);
        HorizontalLayoutGroup hlg = headerObj.AddComponent<HorizontalLayoutGroup>();
        hlg.spacing = 20;
        hlg.childControlWidth = true;
        hlg.childForceExpandWidth = false;

        TextMeshProUGUI vText = CreateText(headerObj.transform, keys, 36, new Color(1f, 0.6f, 0f), font);
        vText.fontStyle = FontStyles.Bold;

        // Divider
        GameObject line = new GameObject("Divider");
        line.transform.SetParent(entry.transform, false);
        RectTransform rt = line.AddComponent<RectTransform>();
        rt.sizeDelta = new Vector2(0, 1);
        Image img = line.AddComponent<Image>();
        img.color = new Color(1f, 1f, 1f, 0.1f);
        LayoutElement le = line.AddComponent<LayoutElement>();
        le.minHeight = 1;

        // Details (Action Name)
        TextMeshProUGUI dText = CreateText(entry.transform, actionName, 28, new Color(0.9f, 0.9f, 0.9f), font);
        dText.enableWordWrapping = true;
    }

    static void CreateHeader(Transform parent, string text, TMP_FontAsset font)
    {
        GameObject obj = new GameObject("Header");
        obj.transform.SetParent(parent, false);
        TextMeshProUGUI tmp = obj.AddComponent<TextMeshProUGUI>();
        tmp.text = text;
        tmp.color = new Color(1f, 0.6f, 0f);
        tmp.fontSize = 64;
        tmp.alignment = TextAlignmentOptions.Left;
        tmp.font = font;

        LayoutElement le = obj.AddComponent<LayoutElement>();
        le.minHeight = 80;

        GameObject line = new GameObject("Underline");
        line.transform.SetParent(obj.transform, false);
        RectTransform rt = line.AddComponent<RectTransform>();
        rt.anchorMin = new Vector2(0, 0); rt.anchorMax = new Vector2(1, 0);
        rt.sizeDelta = new Vector2(0, 2);
        rt.anchoredPosition = new Vector2(0, 10);
        line.AddComponent<Image>().color = new Color(1f, 0.6f, 0f);
    }

    static TextMeshProUGUI CreateText(Transform parent, string content, float size, Color color, TMP_FontAsset font)
    {
        GameObject obj = new GameObject("Text");
        obj.transform.SetParent(parent, false);
        TextMeshProUGUI tmp = obj.AddComponent<TextMeshProUGUI>();
        tmp.text = content;
        tmp.fontSize = size;
        tmp.color = color;
        tmp.font = font;
        return tmp;
    }

    static Button CreateButton(Transform parent, string name, string text, TMP_FontAsset font)
    {
        GameObject obj = new GameObject(name);
        obj.transform.SetParent(parent, false);
        Image img = obj.AddComponent<Image>();
        img.color = new Color(0.1f, 0.1f, 0.1f);
        Button btn = obj.AddComponent<Button>();
        btn.targetGraphic = img;

        GameObject txtObj = new GameObject("Text");
        txtObj.transform.SetParent(obj.transform, false);
        TextMeshProUGUI tmp = txtObj.AddComponent<TextMeshProUGUI>();
        tmp.text = text;
        tmp.color = Color.white;
        tmp.fontSize = 32;
        tmp.alignment = TextAlignmentOptions.Center;
        tmp.font = font;

        RectTransform tRT = txtObj.GetComponent<RectTransform>();
        tRT.anchorMin = Vector2.zero; tRT.anchorMax = Vector2.one;
        tRT.offsetMin = Vector2.zero; tRT.offsetMax = Vector2.zero;

        obj.AddComponent<LayoutElement>().minWidth = 200;
        obj.AddComponent<LayoutElement>().minHeight = 60;
        return btn;
    }
}
#endif