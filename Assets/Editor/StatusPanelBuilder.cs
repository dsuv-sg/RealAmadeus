#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;
using TMPro;
using System.Collections.Generic;

public class StatusPanelBuilder
{
    [MenuItem("Tools/Amadeus/Create Status Panel")]
    public static void CreateStatusPanel()
    {
        Canvas canvas = GameObject.FindObjectOfType<Canvas>();
        if (canvas == null) { Debug.LogError("No Canvas found!"); return; }

        Transform existing = canvas.transform.Find("StatusPanel");
        if (existing != null) GameObject.DestroyImmediate(existing.gameObject);

        // Main Panel
        GameObject panelObj = new GameObject("StatusPanel");
        panelObj.transform.SetParent(canvas.transform, false);
        RectTransform panelRT = panelObj.AddComponent<RectTransform>();
        panelRT.anchorMin = Vector2.zero;
        panelRT.anchorMax = Vector2.one;
        panelRT.offsetMin = Vector2.zero;
        panelRT.offsetMax = Vector2.zero;

        Image panelBg = panelObj.AddComponent<Image>();
        panelBg.color = new Color(0, 0, 0, 0.95f);

        CanvasGroup cg = panelObj.AddComponent<CanvasGroup>();
        StatusPanelController controller = panelObj.AddComponent<StatusPanelController>();
        controller.panelCanvasGroup = cg;

        TMP_FontAsset font = Resources.Load<TMP_FontAsset>("Fonts & Materials/MSMINCHO SDF");

        // Main Layout
        VerticalLayoutGroup mainLayout = panelObj.AddComponent<VerticalLayoutGroup>();
        mainLayout.padding = new RectOffset(50, 50, 50, 50);
        mainLayout.spacing = 30;
        mainLayout.childControlHeight = true;
        mainLayout.childControlWidth = true;
        mainLayout.childForceExpandHeight = false;
        mainLayout.childForceExpandWidth = true;

        // -- Header --
        CreateHeader(panelObj.transform, "SYSTEM STATUS", font);

        // -- Info Grid (2 Columns) --
        GameObject gridObj = new GameObject("InfoGrid");
        gridObj.transform.SetParent(panelObj.transform, false);
        HorizontalLayoutGroup gridLayout = gridObj.AddComponent<HorizontalLayoutGroup>();
        gridLayout.spacing = 50;
        gridLayout.childControlHeight = true;
        gridLayout.childControlWidth = true;
        gridLayout.childForceExpandWidth = true;

        // Left Column
        GameObject col1 = CreateColumn(gridObj.transform, "LeftCol");
        CreateDetailRow(col1.transform, "SYSTEM ID", "AMADEUS-K-004", font);
        CreateDetailRow(col1.transform, "VERSION", "1.05 Beta", font);
        CreateDetailRow(col1.transform, "OPERATOR", "Guest", font);

        // Right Column
        GameObject col2 = CreateColumn(gridObj.transform, "RightCol");
        CreateDetailRow(col2.transform, "DIVERGENCE", "1.048596%", font, new Color(1f, 0.2f, 0.2f));
        CreateDetailRow(col2.transform, "CONNECTION", "Encrypted (2048bit)", font);
        CreateDetailRow(col2.transform, "LOCATION", "Akihabara, Tokyo", font);

        // -- Clock --
        CreateClock(panelObj.transform, font, out TextMeshProUGUI clockTmp);
        controller.clockText = clockTmp;

        // -- Metrics Area --
        CreateMetrics(panelObj.transform, font, controller);

        // -- Spacer --
        GameObject spacer = new GameObject("Spacer");
        spacer.transform.SetParent(panelObj.transform, false);
        spacer.AddComponent<LayoutElement>().flexibleHeight = 1;

        // -- Credits --
        CreateCredits(panelObj.transform, font);

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
            menu.statusPanelController = controller;
            EditorUtility.SetDirty(menu);
        }

        Debug.Log("Status Panel Created (Refined Layout + Credits).");
    }

    static void CreateCredits(Transform parent, TMP_FontAsset font)
    {
        GameObject obj = new GameObject("Credits");
        obj.transform.SetParent(parent, false);
        TextMeshProUGUI tmp = obj.AddComponent<TextMeshProUGUI>();
        tmp.text = "System Design: Future Gadget Lab / Amadeus Project\nVersion 1.05 Beta (Build 2026.02.12)";
        tmp.fontSize = 24;
        tmp.color = new Color(0.5f, 0.5f, 0.5f);
        tmp.alignment = TextAlignmentOptions.BottomRight;
        tmp.font = font;
        tmp.enableWordWrapping = false;
        
        LayoutElement le = obj.AddComponent<LayoutElement>();
        le.minHeight = 40;
    }

    static GameObject CreateColumn(Transform parent, string name)
    {
        GameObject col = new GameObject(name);
        col.transform.SetParent(parent, false);
        VerticalLayoutGroup vlg = col.AddComponent<VerticalLayoutGroup>();
        vlg.spacing = 15;
        // Force Expand width to prevent squashing
        vlg.childControlWidth = true;
        vlg.childForceExpandWidth = true; 
        
        col.AddComponent<LayoutElement>().flexibleWidth = 1; 
        return col;
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
        tmp.enableWordWrapping = false; // Fix: Prevent wrapping

        // Layout Element for Header height
        LayoutElement le = obj.AddComponent<LayoutElement>();
        le.minHeight = 80;

        // Underline
        GameObject line = new GameObject("Underline");
        line.transform.SetParent(obj.transform, false);
        RectTransform rt = line.AddComponent<RectTransform>();
        rt.anchorMin = new Vector2(0, 0); rt.anchorMax = new Vector2(1, 0);
        rt.sizeDelta = new Vector2(0, 2);
        rt.anchoredPosition = new Vector2(0, 10);
        line.AddComponent<Image>().color = new Color(1f, 0.6f, 0f);
    }

    static void CreateDetailRow(Transform parent, string label, string value, TMP_FontAsset font, Color? valueColor = null)
    {
        GameObject row = new GameObject($"Row_{label}");
        row.transform.SetParent(parent, false);
        HorizontalLayoutGroup hlg = row.AddComponent<HorizontalLayoutGroup>();
        hlg.spacing = 20;
        hlg.childControlWidth = true;
        hlg.childForceExpandWidth = false;

        // Label: Fixed Width for alignment
        TextMeshProUGUI lTmp = AddText(row, label, 32, Color.gray, font);
        LayoutElement leL = lTmp.gameObject.AddComponent<LayoutElement>();
        leL.minWidth = 250; 
        leL.preferredWidth = 250;

        // Value: Flexible
        TextMeshProUGUI vTmp = AddText(row, value, 32, valueColor ?? Color.white, font);
        LayoutElement leV = vTmp.gameObject.AddComponent<LayoutElement>();
        leV.flexibleWidth = 1;
    }

    static void CreateMetrics(Transform parent, TMP_FontAsset font, StatusPanelController ctrl)
    {
        GameObject set = new GameObject("Metrics");
        set.transform.SetParent(parent, false);
        VerticalLayoutGroup vlg = set.AddComponent<VerticalLayoutGroup>();
        vlg.spacing = 20;
        
        CreateMetricBar(set.transform, "CPU_LOAD", font, out ctrl.cpuSlider, out ctrl.cpuText);
        CreateMetricBar(set.transform, "MEMORY", font, out ctrl.memorySlider, out ctrl.memoryText);
        CreateMetricBar(set.transform, "SYNCHRONIZATION", font, out ctrl.syncSlider, out TextMeshProUGUI dummy, true);
        if (ctrl.syncSlider) ctrl.syncSlider.value = 0.98f;
    }

    static void CreateMetricBar(Transform parent, string label, TMP_FontAsset font, out Slider slider, out TextMeshProUGUI valText, bool hideValue = false)
    {
        GameObject row = new GameObject($"Metric_{label}");
        row.transform.SetParent(parent, false);
        HorizontalLayoutGroup hlg = row.AddComponent<HorizontalLayoutGroup>();
        hlg.spacing = 20;
        hlg.childControlWidth = true;
        hlg.childForceExpandWidth = false;

        // Label: Fixed Width
        TextMeshProUGUI lTmp = AddText(row, label, 28, Color.gray, font);
        LayoutElement leL = lTmp.gameObject.AddComponent<LayoutElement>();
        leL.minWidth = 300; 

        // Slider: Flexible
        GameObject sliderObj = new GameObject("Slider");
        sliderObj.transform.SetParent(row.transform, false);
        LayoutElement leS = sliderObj.AddComponent<LayoutElement>();
        leS.flexibleWidth = 1;
        leS.minHeight = 20; // Reduced from 40 to 20

        slider = sliderObj.AddComponent<Slider>();
        
        // Slider Visuals
        GameObject bg = new GameObject("Background");
        bg.transform.SetParent(sliderObj.transform, false);
        RectTransform bgRT = bg.AddComponent<RectTransform>();
        bgRT.anchorMin = Vector2.zero; bgRT.anchorMax = Vector2.one;
        Image bgImg = bg.AddComponent<Image>();
        bgImg.color = new Color(0.2f, 0.2f, 0.2f);
        slider.targetGraphic = bgImg;

        GameObject fillArea = new GameObject("Fill Area");
        fillArea.transform.SetParent(sliderObj.transform, false);
        RectTransform faRT = fillArea.AddComponent<RectTransform>();
        faRT.anchorMin = Vector2.zero; faRT.anchorMax = Vector2.one;
        faRT.sizeDelta = new Vector2(-10, -10); // Padding

        GameObject fill = new GameObject("Fill");
        fill.transform.SetParent(fillArea.transform, false);
        RectTransform fRT = fill.AddComponent<RectTransform>();
        fRT.anchorMin = Vector2.zero; fRT.anchorMax = Vector2.one;
        Image fillImg = fill.AddComponent<Image>();
        fillImg.color = new Color(1f, 0.6f, 0f);
        slider.fillRect = fRT;
        slider.direction = Slider.Direction.LeftToRight;

        // Value: Fixed Width
        if (!hideValue)
        {
            valText = AddText(row, "0%", 28, Color.white, font);
            valText.alignment = TextAlignmentOptions.Right;
            LayoutElement leV = valText.gameObject.AddComponent<LayoutElement>();
            leV.minWidth = 100;
        }
        else
        {
            valText = null;
            // Add spacer for alignment
            GameObject spacer = new GameObject("Spacer");
            spacer.transform.SetParent(row.transform, false);
            LayoutElement leSp = spacer.AddComponent<LayoutElement>();
            leSp.minWidth = 100;
        }
    }

    static TextMeshProUGUI AddText(GameObject parent, string content, float size, Color color, TMP_FontAsset font)
    {
        GameObject obj = new GameObject("Text");
        obj.transform.SetParent(parent.transform, false);
        TextMeshProUGUI tmp = obj.AddComponent<TextMeshProUGUI>();
        tmp.text = content;
        tmp.fontSize = size;
        tmp.color = color;
        tmp.alignment = TextAlignmentOptions.MidlineLeft;
        tmp.enableWordWrapping = false; // Fix: Crucial for preventing vertical stacking
        tmp.overflowMode = TextOverflowModes.Ellipsis; // Or Truncate
        tmp.font = font;
        return tmp;
    }

    static void CreateClock(Transform parent, TMP_FontAsset font, out TextMeshProUGUI clockTmp)
    {
        GameObject obj = new GameObject("Clock");
        obj.transform.SetParent(parent, false);
        clockTmp = obj.AddComponent<TextMeshProUGUI>();
        clockTmp.text = "0000/00/00 00:00:00";
        clockTmp.fontSize = 54;
        clockTmp.color = Color.white;
        clockTmp.alignment = TextAlignmentOptions.Center;
        clockTmp.font = font;
        obj.AddComponent<LayoutElement>().minHeight = 80;
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
