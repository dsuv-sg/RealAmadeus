#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;
using TMPro;

public class ConfigPanelBuilder : Editor
{
    [MenuItem("Tools/Amadeus/Create Config Panel (S;G Style)")]
    public static void CreateConfigPanel()
    {
        // 1. Find Main Canvas
        GameObject canvasObj = GameObject.Find("Canvas");
        if (canvasObj == null)
        {
            Canvas canvas = Object.FindFirstObjectByType<Canvas>();
            if (canvas != null) canvasObj = canvas.gameObject;
            else
            {
                Debug.LogError("No Canvas found!");
                return;
            }
        }

        // 2. Load Font
        TMP_FontAsset font = AssetDatabase.LoadAssetAtPath<TMP_FontAsset>("Assets/Fonts/MSMINCHO SDF.asset");
        if (font == null) Debug.LogWarning("MSMINCHO SDF font not found at Assets/Fonts/MSMINCHO SDF.asset");

        // 3. Create Panel Root
        GameObject panelObj = new GameObject("ConfigPanel");
        panelObj.transform.SetParent(canvasObj.transform, false);
        RectTransform panelRT = panelObj.AddComponent<RectTransform>();
        panelRT.anchorMin = Vector2.zero;
        panelRT.anchorMax = Vector2.one;
        panelRT.offsetMin = Vector2.zero;
        panelRT.offsetMax = Vector2.zero;

        Image panelBg = panelObj.AddComponent<Image>();
        panelBg.color = new Color(0, 0, 0, 0.9f);
        panelObj.AddComponent<CanvasGroup>();
        
        ConfigPanelController controller = panelObj.AddComponent<ConfigPanelController>();

        // 4. Main Layout (Horizontal: Sidebar | Content)
        HorizontalLayoutGroup mainLayout = panelObj.AddComponent<HorizontalLayoutGroup>();
        mainLayout.childForceExpandWidth = false;
        mainLayout.childForceExpandHeight = true;
        mainLayout.spacing = 10;
        mainLayout.padding = new RectOffset(20, 20, 20, 20);

        // --- SIDEBAR ---
        GameObject sidebar = new GameObject("Sidebar");
        sidebar.transform.SetParent(panelObj.transform, false);
        LayoutElement sideLE = sidebar.AddComponent<LayoutElement>();
        sideLE.minWidth = 600; // Increased from 450
        sideLE.flexibleHeight = 1;
        
        VerticalLayoutGroup sideLayout = sidebar.AddComponent<VerticalLayoutGroup>();
        sideLayout.spacing = 15;
        sideLayout.childAlignment = TextAnchor.UpperCenter;
        sideLayout.childControlHeight = false;
        sideLayout.childControlWidth = true;
        
        controller.categoryButtons = new System.Collections.Generic.List<Button>();

        // Create Category Buttons (Japanese)
        controller.categoryButtons.Add(CreateCategoryButton(sidebar.transform, "Btn_General", "基本設定", font));
        controller.categoryButtons.Add(CreateCategoryButton(sidebar.transform, "Btn_Text", "テキスト設定", font));
        controller.categoryButtons.Add(CreateCategoryButton(sidebar.transform, "Btn_Sound", "サウンド設定", font));
        controller.categoryButtons.Add(CreateCategoryButton(sidebar.transform, "Btn_Graphic", "グラフィック設定", font));
        controller.categoryButtons.Add(CreateCategoryButton(sidebar.transform, "Btn_API", "API設定", font));

        // --- CONTENT AREA ---
        GameObject contentArea = new GameObject("ContentArea");
        contentArea.transform.SetParent(panelObj.transform, false);
        LayoutElement contentLE = contentArea.AddComponent<LayoutElement>();
        contentLE.flexibleWidth = 1;
        contentLE.flexibleHeight = 1;
        
        VerticalLayoutGroup contentLayout = contentArea.AddComponent<VerticalLayoutGroup>();
        contentLayout.spacing = 20;
        contentLayout.padding = new RectOffset(40, 40, 20, 20);
        contentLayout.childControlHeight = false;
        contentLayout.childControlWidth = true;
        contentLayout.childAlignment = TextAnchor.UpperLeft;

        // Header
        controller.headerText = CreateLabel(contentArea.transform, "Header", "基本設定", Vector2.zero, 48, TextAlignmentOptions.Left, new Color(1f, 0.6f, 0f), font);
        
        // Separator
        GameObject sep = new GameObject("Separator");
        sep.transform.SetParent(contentArea.transform, false);
        LayoutElement sepLE = sep.AddComponent<LayoutElement>();
        sepLE.minHeight = 2; sepLE.preferredHeight = 2;
        Image sepImg = sep.AddComponent<Image>();
        sepImg.color = new Color(1f, 0.6f, 0f);

        // Pages Container
        GameObject pagesContainer = new GameObject("PagesContainer");
        pagesContainer.transform.SetParent(contentArea.transform, false);
        LayoutElement pagesLE = pagesContainer.AddComponent<LayoutElement>();
        pagesLE.flexibleHeight = 1;
        
        // Stack pages on top of each other
        RectTransform pagesRT = pagesContainer.GetComponent<RectTransform>();
        if (pagesRT == null) pagesRT = pagesContainer.AddComponent<RectTransform>();
        pagesRT.anchorMin = Vector2.zero; pagesRT.anchorMax = Vector2.one; 
        
        controller.categoryPages = new System.Collections.Generic.List<GameObject>();

        // --- PAGE 0: GENERAL ---
        GameObject pageGeneral = CreatePage(pagesContainer.transform, "Page_General");
        controller.categoryPages.Add(pageGeneral);
        CreateSettingRowToggle(pageGeneral.transform, "SkipWeb", "ロードスキップ", font, out controller.skipLoadingToggle);
        CreateSettingRowToggle(pageGeneral.transform, "RightClick", "右クリックメニュー", font, out controller.rightClickMenuToggle);

        // --- PAGE 1: TEXT ---
        GameObject pageText = CreatePage(pagesContainer.transform, "Page_Text");
        controller.categoryPages.Add(pageText);
        CreateSettingRowSlider(pageText.transform, "TextSpeed", "文字表示速度", font, out controller.textSpeedSlider, out controller.textSpeedValueText);
        CreateSettingRowSlider(pageText.transform, "AutoSpeed", "オート待機時間", font, out controller.autoSpeedSlider, out controller.autoSpeedValueText);
        CreateSettingRowToggle(pageText.transform, "AutoView", "オートモード", font, out controller.autoModeToggle);

        // --- PAGE 2: SOUND ---
        GameObject pageSound = CreatePage(pagesContainer.transform, "Page_Sound");
        controller.categoryPages.Add(pageSound);
        CreateSettingRowSlider(pageSound.transform, "MasterVol", "マスター音量", font, out controller.masterVolumeSlider, out controller.masterVolumeValueText);
        CreateSettingRowSlider(pageSound.transform, "BGMVol", "BGM音量", font, out controller.bgmVolumeSlider, out controller.bgmVolumeValueText);
        CreateSettingRowSlider(pageSound.transform, "SEVol", "SE音量", font, out controller.seVolumeSlider, out controller.seVolumeValueText);
        CreateSettingRowSlider(pageSound.transform, "VoiceVol", "ボイス音量", font, out controller.voiceVolumeSlider, out controller.voiceVolumeValueText);

        // --- PAGE 3: GRAPHIC ---
        GameObject pageGraphic = CreatePage(pagesContainer.transform, "Page_Graphic");
        controller.categoryPages.Add(pageGraphic);
        CreateSettingRowDropdown(pageGraphic.transform, "ScreenMode", "画面モード", font, out controller.screenModeDropdown);
        if (controller.screenModeDropdown) controller.screenModeDropdown.AddOptions(new System.Collections.Generic.List<string> { "ウィンドウ", "フルスクリーン", "ボーダーレス" });
        CreateSettingRowDropdown(pageGraphic.transform, "Resolution", "解像度", font, out controller.resolutionDropdown);
        if (controller.resolutionDropdown) controller.resolutionDropdown.AddOptions(new System.Collections.Generic.List<string> { "1920x1080", "1280x720", "テキストウィンドウのみ" });

        // --- PAGE 4: API ---
        GameObject pageAPI = CreatePage(pagesContainer.transform, "Page_API");
        controller.categoryPages.Add(pageAPI);
        CreateSettingRowDropdown(pageAPI.transform, "Provider", "APIプロバイダ", font, out controller.apiProviderDropdown);
        if (controller.apiProviderDropdown) controller.apiProviderDropdown.AddOptions(new System.Collections.Generic.List<string> { "OpenAI", "Google Gemini", "Anthropic Claude", "Local LLM" });
        CreateSettingRowInput(pageAPI.transform, "APIKey", "APIキー", font, out controller.apiKeyInputField, true);
        CreateSettingRowInput(pageAPI.transform, "ModelName", "モデル名", font, out controller.modelNameInputField, false);

        // Vertex Fields (Auto-hidden by controller if not Vertex)
        CreateSettingRowInput(pageAPI.transform, "VertexProject", "GCPプロジェクトID", font, out controller.vertexProjectInputField, false);
        CreateSettingRowInput(pageAPI.transform, "VertexLocation", "リージョン (例: us-central1)", font, out controller.vertexLocationInputField, false);
        CreateSettingRowToggle(pageAPI.transform, "VertexUseGcloud", "gcloud CLIを使う (上級者向け)", font, out controller.vertexUseGcloudToggle);
        CreateSettingRowInput(pageAPI.transform, "VertexClientId", "Web用 Client ID", font, out controller.vertexClientIdInputField, false);
        
        GameObject authBtnObj = CreateButton(pageAPI.transform, "Btn_VertexAuth", "Vertex AI 認証 (ブラウザ)", font);
        controller.vertexAuthButton = authBtnObj.GetComponent<Button>();
        authBtnObj.GetComponent<Image>().color = new Color(0.2f, 0.4f, 0.8f); // Blueish


        // Spacer to push Footer to bottom
        GameObject footerSpacer = new GameObject("FooterSpacer");
        footerSpacer.transform.SetParent(contentArea.transform, false);
        footerSpacer.AddComponent<LayoutElement>().flexibleHeight = 1;

        // Footer (Save/Cancel) - Bottom Right of Content Area
        GameObject footer = new GameObject("Footer");
        footer.transform.SetParent(contentArea.transform, false);
        LayoutElement footerLE = footer.AddComponent<LayoutElement>();
        footerLE.minHeight = 60; // Ensure it has height
        
        HorizontalLayoutGroup footerLayout = footer.AddComponent<HorizontalLayoutGroup>();
        footerLayout.childAlignment = TextAnchor.MiddleRight;
        footerLayout.spacing = 20;
        footerLayout.childControlWidth = false;
        footerLayout.childForceExpandWidth = false;
        
        GameObject btnSaveObj = CreateButton(footer.transform, "Btn_Save", "保存 & 閉じる", font);
        controller.saveButton = btnSaveObj.GetComponent<Button>();
        // Make save button distinct color
        btnSaveObj.GetComponent<Image>().color = new Color(1f, 0.6f, 0f);
        btnSaveObj.transform.Find("Text").GetComponent<TextMeshProUGUI>().color = Color.black;

        GameObject btnCancelObj = CreateButton(footer.transform, "Btn_Cancel", "キャンセル", font);
        controller.cancelButton = btnCancelObj.GetComponent<Button>();

        // Register Undo
        Undo.RegisterCreatedObjectUndo(panelObj, "Create Config Panel");
        Selection.activeObject = panelObj;
        Debug.Log("Config Panel (S;G Style) Created!");
    }

    static GameObject CreatePage(Transform parent, string name)
    {
        GameObject page = new GameObject(name);
        page.transform.SetParent(parent, false);
        RectTransform rt = page.AddComponent<RectTransform>();
        rt.anchorMin = Vector2.zero; rt.anchorMax = Vector2.one; 
        rt.offsetMin = Vector2.zero; rt.offsetMax = Vector2.zero;
        
        VerticalLayoutGroup vlg = page.AddComponent<VerticalLayoutGroup>();
        vlg.spacing = 15;
        vlg.childControlHeight = false;
        vlg.childControlWidth = true;
        vlg.childAlignment = TextAnchor.UpperLeft;
        
        // Default hidden, controller will show first one
        page.SetActive(false);
        return page;
    }

    // --- HELPER METHODS (Manual Construction) ---

    // Updated with Font
    static Button CreateCategoryButton(Transform parent, string name, string text, TMP_FontAsset font)
    {
        GameObject obj = new GameObject(name);
        obj.transform.SetParent(parent, false);
        LayoutElement le = obj.AddComponent<LayoutElement>();
        le.minHeight = 150; // Increased from 120
        Image img = obj.AddComponent<Image>();
        img.color = new Color(0.2f, 0.2f, 0.2f);
        
        Button btn = obj.AddComponent<Button>();
        btn.targetGraphic = img;

        Outline outline = obj.AddComponent<Outline>();
        outline.effectColor = Color.white;
        outline.effectDistance = new Vector2(2, -2);
        outline.enabled = false;

        GameObject txtObj = new GameObject("Text");
        txtObj.transform.SetParent(obj.transform, false);
        RectTransform txtRT = txtObj.AddComponent<RectTransform>();
        txtRT.anchorMin = Vector2.zero; txtRT.anchorMax = Vector2.one; txtRT.offsetMin = Vector2.zero; txtRT.offsetMax = Vector2.zero;
        
        TextMeshProUGUI tmp = txtObj.AddComponent<TextMeshProUGUI>();
        tmp.text = text;
        tmp.color = new Color(1f, 0.6f, 0f); // S;G Orange
        tmp.alignment = TextAlignmentOptions.Center;
        tmp.fontSize = 48; // Increased from 36
        if (font != null) tmp.font = font;

        return btn;
    }

    static GameObject CreateButton(Transform parent, string name, string text, TMP_FontAsset font)
    {
        GameObject obj = new GameObject(name);
        obj.transform.SetParent(parent, false);
        LayoutElement le = obj.AddComponent<LayoutElement>();
        le.minWidth = 150; le.minHeight = 40;
        Image img = obj.AddComponent<Image>();
        img.color = new Color(0.3f, 0.3f, 0.3f);
        Button btn = obj.AddComponent<Button>();
        btn.targetGraphic = img;

        GameObject txtObj = new GameObject("Text");
        txtObj.transform.SetParent(obj.transform, false);
        RectTransform txtRT = txtObj.AddComponent<RectTransform>();
        txtRT.anchorMin = Vector2.zero; txtRT.anchorMax = Vector2.one; txtRT.offsetMin = Vector2.zero; txtRT.offsetMax = Vector2.zero;
        
        TextMeshProUGUI tmp = txtObj.AddComponent<TextMeshProUGUI>();
        tmp.text = text;
        tmp.color = Color.white;
        tmp.alignment = TextAlignmentOptions.Center;
        if (font != null) tmp.font = font;
        return obj;
    }

    static TextMeshProUGUI CreateLabel(Transform parent, string name, string text, Vector2 pos, float fontSize, TextAlignmentOptions align, Color color, TMP_FontAsset font)
    {
        GameObject obj = new GameObject(name);
        obj.transform.SetParent(parent, false);
        RectTransform rt = obj.AddComponent<RectTransform>();
        rt.anchoredPosition = pos;
        rt.sizeDelta = new Vector2(1000, 80); // Increased width
        
        TextMeshProUGUI tmp = obj.AddComponent<TextMeshProUGUI>();
        tmp.text = text;
        tmp.fontSize = fontSize;
        tmp.alignment = align;
        tmp.color = color;
        tmp.enableWordWrapping = false;
        tmp.overflowMode = TextOverflowModes.Overflow;
        if (font != null) tmp.font = font;
        return tmp;
    }

    // --- ROW CREATORS ---

    static GameObject CreateRowBase(Transform parent, string name, string labelText, TMP_FontAsset font)
    {
        GameObject row = new GameObject(name);
        row.transform.SetParent(parent, false);
        LayoutElement le = row.AddComponent<LayoutElement>();
        le.minHeight = 50;
        
        HorizontalLayoutGroup hlg = row.AddComponent<HorizontalLayoutGroup>();
        hlg.childControlWidth = false; hlg.childForceExpandWidth = false; hlg.spacing = 20; hlg.childAlignment = TextAnchor.MiddleLeft;

        // Label
        GameObject txtObj = new GameObject("Label");
        txtObj.transform.SetParent(row.transform, false);
        LayoutElement txtLE = txtObj.AddComponent<LayoutElement>();
        txtLE.minWidth = 300; 
        
        TextMeshProUGUI tmp = txtObj.AddComponent<TextMeshProUGUI>();
        tmp.text = labelText;
        tmp.color = Color.white;
        tmp.fontSize = 24;
        tmp.alignment = TextAlignmentOptions.Left;
        if (font != null) tmp.font = font;
        
        return row;
    }

    static void CreateSettingRowSlider(Transform parent, string name, string label, TMP_FontAsset font, out Slider slider, out TextMeshProUGUI valueText)
    {
        GameObject row = CreateRowBase(parent, name, label, font);
        
        GameObject sliderObj = new GameObject("Slider");
        sliderObj.transform.SetParent(row.transform, false);
        RectTransform rt = sliderObj.AddComponent<RectTransform>();
        rt.sizeDelta = new Vector2(300, 20);
        slider = sliderObj.AddComponent<Slider>();
        
        // Background
        GameObject bg = new GameObject("Background");
        bg.transform.SetParent(sliderObj.transform, false);
        Image bgImg = bg.AddComponent<Image>();
        bgImg.color = new Color(0.2f, 0.2f, 0.2f);
        RectTransform bgRT = bg.GetComponent<RectTransform>();
        bgRT.anchorMin = new Vector2(0, 0.25f); bgRT.anchorMax = new Vector2(1, 0.75f); bgRT.offsetMin = Vector2.zero; bgRT.offsetMax = Vector2.zero;

        // Fill
        GameObject fillArea = new GameObject("Fill Area");
        fillArea.transform.SetParent(sliderObj.transform, false);
        RectTransform faRT = fillArea.AddComponent<RectTransform>();
        faRT.anchorMin = new Vector2(0, 0.25f); faRT.anchorMax = new Vector2(1, 0.75f); faRT.offsetMin = new Vector2(5, 0); faRT.offsetMax = new Vector2(-5, 0);
        GameObject fill = new GameObject("Fill");
        fill.transform.SetParent(fillArea.transform, false);
        Image fillImg = fill.AddComponent<Image>();
        fillImg.color = new Color(1f, 0.6f, 0f);
        RectTransform fRT = fill.GetComponent<RectTransform>();
        fRT.anchorMin = Vector2.zero; fRT.anchorMax = Vector2.one; fRT.sizeDelta = Vector2.zero;
        slider.fillRect = fRT;

        // Handle
        GameObject handleArea = new GameObject("Handle Slide Area");
        handleArea.transform.SetParent(sliderObj.transform, false);
        RectTransform haRT = handleArea.AddComponent<RectTransform>();
        haRT.anchorMin = Vector2.zero; haRT.anchorMax = Vector2.one; haRT.offsetMin = new Vector2(5, 0); haRT.offsetMax = new Vector2(-5, 0);
        GameObject handle = new GameObject("Handle");
        handle.transform.SetParent(handleArea.transform, false);
        Image hImg = handle.AddComponent<Image>();
        hImg.color = Color.white;
        RectTransform hRT = handle.GetComponent<RectTransform>();
        hRT.sizeDelta = new Vector2(20, 20);
        slider.handleRect = hRT;
        slider.targetGraphic = hImg;
        slider.direction = Slider.Direction.LeftToRight;

        // Value
        GameObject valObj = new GameObject("Value");
        valObj.transform.SetParent(row.transform, false);
        RectTransform valRT = valObj.AddComponent<RectTransform>();
        valRT.sizeDelta = new Vector2(80, 40);
        valueText = valObj.AddComponent<TextMeshProUGUI>();
        valueText.text = "100%";
        valueText.alignment = TextAlignmentOptions.Right;
        valueText.color = Color.white;
        if (font != null) valueText.font = font;
    }

    static void CreateSettingRowToggle(Transform parent, string name, string label, TMP_FontAsset font, out Toggle toggle)
    {
        GameObject row = CreateRowBase(parent, name, label, font);
        
        GameObject toggleObj = new GameObject("Toggle");
        toggleObj.transform.SetParent(row.transform, false);
        RectTransform rt = toggleObj.AddComponent<RectTransform>();
        rt.sizeDelta = new Vector2(40, 40);
        toggle = toggleObj.AddComponent<Toggle>();

        GameObject bg = new GameObject("Background");
        bg.transform.SetParent(toggleObj.transform, false);
        Image bgImg = bg.AddComponent<Image>();
        bgImg.color = new Color(0.2f, 0.2f, 0.2f);
        RectTransform bgRT = bg.GetComponent<RectTransform>();
        bgRT.anchorMin = Vector2.zero; bgRT.anchorMax = Vector2.one; bgRT.offsetMin = Vector2.zero; bgRT.offsetMax = Vector2.zero;
        toggle.targetGraphic = bgImg;

        GameObject check = new GameObject("Checkmark");
        check.transform.SetParent(bg.transform, false);
        Image chkImg = check.AddComponent<Image>();
        chkImg.color = new Color(1f, 0.6f, 0f);
        RectTransform chkRT = check.GetComponent<RectTransform>();
        chkRT.anchorMin = new Vector2(0.2f, 0.2f); chkRT.anchorMax = new Vector2(0.8f, 0.8f); chkRT.offsetMin = Vector2.zero; chkRT.offsetMax = Vector2.zero;
        toggle.graphic = chkImg;
        toggle.isOn = true;
    }

    static void CreateSettingRowInput(Transform parent, string name, string label, TMP_FontAsset font, out TMP_InputField input, bool isPassword = false)
    {
        GameObject row = CreateRowBase(parent, name, label, font);
        
        GameObject inputObj = new GameObject("InputField");
        inputObj.transform.SetParent(row.transform, false);
        RectTransform rt = inputObj.AddComponent<RectTransform>();
        rt.sizeDelta = new Vector2(400, 40);
        Image bg = inputObj.AddComponent<Image>();
        bg.color = new Color(0.1f, 0.1f, 0.1f);
        input = inputObj.AddComponent<TMP_InputField>();
        input.targetGraphic = bg;
        
        GameObject textArea = new GameObject("TextArea");
        textArea.transform.SetParent(inputObj.transform, false);
        RectTransform taRT = textArea.AddComponent<RectTransform>();
        taRT.anchorMin = Vector2.zero; taRT.anchorMax = Vector2.one; taRT.offsetMin = new Vector2(10, 5); taRT.offsetMax = new Vector2(-10, -5);
        RectMask2D mask = textArea.AddComponent<RectMask2D>();
        
        GameObject placeholder = new GameObject("Placeholder");
        placeholder.transform.SetParent(textArea.transform, false);
        TextMeshProUGUI phTMP = placeholder.AddComponent<TextMeshProUGUI>();
        phTMP.text = "Enter...";
        phTMP.color = new Color(0.5f, 0.5f, 0.5f);
        phTMP.fontSize = 20;
        if (font != null) phTMP.font = font;
        input.placeholder = phTMP;
        RectTransform phRT = placeholder.GetComponent<RectTransform>();
        phRT.anchorMin = Vector2.zero; phRT.anchorMax = Vector2.one; phRT.offsetMin = Vector2.zero; phRT.offsetMax = Vector2.zero;

        GameObject text = new GameObject("Text");
        text.transform.SetParent(textArea.transform, false);
        TextMeshProUGUI tTMP = text.AddComponent<TextMeshProUGUI>();
        tTMP.text = "";
        tTMP.color = Color.white;
        tTMP.fontSize = 20;
        if (font != null) tTMP.font = font;
        input.textComponent = tTMP;
        RectTransform tRT = text.GetComponent<RectTransform>();
        tRT.anchorMin = Vector2.zero; tRT.anchorMax = Vector2.one; tRT.offsetMin = Vector2.zero; tRT.offsetMax = Vector2.zero;
        
        input.textViewport = taRT;
        if (isPassword) input.contentType = TMP_InputField.ContentType.Password;
    }
    
    static void CreateSettingRowDropdown(Transform parent, string name, string label, TMP_FontAsset font, out TMP_Dropdown dropdown)
    {
        GameObject row = CreateRowBase(parent, name, label, font);
        
        GameObject ddObj = new GameObject("Dropdown");
        ddObj.transform.SetParent(row.transform, false);
        RectTransform rt = ddObj.AddComponent<RectTransform>();
        rt.sizeDelta = new Vector2(400, 40);
        Image bg = ddObj.AddComponent<Image>();
        bg.color = new Color(0.1f, 0.1f, 0.1f);
        dropdown = ddObj.AddComponent<TMP_Dropdown>();
        dropdown.targetGraphic = bg;
        
        GameObject labelObj = new GameObject("Label");
        labelObj.transform.SetParent(ddObj.transform, false);
        TextMeshProUGUI lTMP = labelObj.AddComponent<TextMeshProUGUI>();
        lTMP.text = "Option A";
        lTMP.color = Color.white;
        lTMP.fontSize = 20;
        lTMP.alignment = TextAlignmentOptions.Left;
        if (font != null) lTMP.font = font;
        dropdown.captionText = lTMP;
        RectTransform lRT = labelObj.GetComponent<RectTransform>();
        lRT.anchorMin = Vector2.zero; lRT.anchorMax = Vector2.one; lRT.offsetMin = new Vector2(10, 0); lRT.offsetMax = new Vector2(-30, 0);

        GameObject arrow = new GameObject("Arrow");
        arrow.transform.SetParent(ddObj.transform, false);
        RectTransform aRT = arrow.AddComponent<RectTransform>();
        aRT.anchorMin = new Vector2(1, 0.5f); aRT.anchorMax = new Vector2(1, 0.5f); aRT.sizeDelta = new Vector2(20, 20); aRT.anchoredPosition = new Vector2(-15, 0);
        Image aImg = arrow.AddComponent<Image>();
        aImg.color = new Color(1f, 0.6f, 0f); // Orange

        // Template (Hidden initially) - Minimal setup for dropdown to work? 
        // Dropdown needs a specific template structure. Skipped for brevity, might be broken.
        // User might need to fix dropdown template manually or just use the input field for now.
        // Or create a dummy template.
        
        // Minimal template:
        GameObject template = new GameObject("Template");
        template.transform.SetParent(ddObj.transform, false);
        template.SetActive(false);
        
        // Add Canvas to break out of masks
        Canvas templateCanvas = template.AddComponent<Canvas>();
        templateCanvas.overrideSorting = true;
        templateCanvas.sortingOrder = 30000;
        template.AddComponent<GraphicRaycaster>();
        
        Image tImg = template.AddComponent<Image>();
        tImg.color = new Color(0.05f, 0.05f, 0.05f);
        ScrollRect sr = template.AddComponent<ScrollRect>();
        template.AddComponent<RectMask2D>();
        dropdown.template = template.GetComponent<RectTransform>();
        
        GameObject viewport = new GameObject("Viewport");
        viewport.transform.SetParent(template.transform, false);
        RectTransform vRT = viewport.AddComponent<RectTransform>();
        vRT.anchorMin = Vector2.zero; vRT.anchorMax = Vector2.one;
        viewport.AddComponent<Image>();
        viewport.AddComponent<Mask>();
        sr.viewport = vRT;

        GameObject content = new GameObject("Content");
        content.transform.SetParent(viewport.transform, false);
        RectTransform cRT = content.AddComponent<RectTransform>();
        cRT.anchorMin = new Vector2(0, 1); cRT.anchorMax = new Vector2(1, 1); cRT.pivot = new Vector2(0.5f, 1);
        sr.content = cRT;

        GameObject item = new GameObject("Item");
        item.transform.SetParent(content.transform, false);
        Toggle itemToggle = item.AddComponent<Toggle>();
        item.AddComponent<LayoutElement>().minHeight = 30;
        
        GameObject itemBg = new GameObject("Item Background");
        itemBg.transform.SetParent(item.transform, false);
        Image iBg = itemBg.AddComponent<Image>();
        iBg.color = new Color(0.2f, 0.2f, 0.2f);
        itemToggle.targetGraphic = iBg;
        
        GameObject itemLabel = new GameObject("Item Label");
        itemLabel.transform.SetParent(item.transform, false);
        TextMeshProUGUI ilTMP = itemLabel.AddComponent<TextMeshProUGUI>();
        ilTMP.text = "Option";
        ilTMP.color = Color.white;
        if (font != null) ilTMP.font = font;
        dropdown.itemText = ilTMP;
    }
}
#endif
