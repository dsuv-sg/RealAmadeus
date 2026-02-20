#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;
using TMPro;

public class BackLogBuilder : EditorWindow
{
    [MenuItem("Tools/Build BackLog UI")]
    public static void BuildUI()
    {
        // 1. Find or Create Canvas
        Canvas canvas = FindObjectOfType<Canvas>();
        if (canvas == null)
        {
            GameObject canvasObj = new GameObject("Canvas");
            canvas = canvasObj.AddComponent<Canvas>();
            canvas.renderMode = RenderMode.ScreenSpaceOverlay;
            canvasObj.AddComponent<CanvasScaler>();
            canvasObj.AddComponent<GraphicRaycaster>();
        }

        // 2. Create BackLog Panel (Root)
        GameObject root = new GameObject("BackLogPanel");
        root.transform.SetParent(canvas.transform, false);
        RectTransform rootRT = root.AddComponent<RectTransform>();
        rootRT.anchorMin = Vector2.zero;
        rootRT.anchorMax = Vector2.one;
        rootRT.offsetMin = Vector2.zero;
        rootRT.offsetMax = Vector2.zero;

        Image rootBg = root.AddComponent<Image>();
        rootBg.color = new Color(0.05f, 0.05f, 0.05f, 0.95f); // Very dark gray

        // 3. Main Layout (Horizontal: Left Bar | Right Content)
        GameObject mainLayout = new GameObject("MainLayout");
        mainLayout.transform.SetParent(root.transform, false);
        RectTransform mainRT = mainLayout.AddComponent<RectTransform>();
        mainRT.anchorMin = Vector2.zero;
        mainRT.anchorMax = Vector2.one;
        mainRT.offsetMin = new Vector2(50, 50); // Padding
        mainRT.offsetMax = new Vector2(-50, -50);

        HorizontalLayoutGroup hlg = mainLayout.AddComponent<HorizontalLayoutGroup>();
        hlg.childControlHeight = true;
        hlg.childControlWidth = true;
        hlg.spacing = 20;

        // 4. Left Sidebar
        GameObject sidebar = new GameObject("Sidebar");
        sidebar.transform.SetParent(mainLayout.transform, false);
        sidebar.AddComponent<RectTransform>();
        LayoutElement sideLe = sidebar.AddComponent<LayoutElement>();
        sideLe.preferredWidth = 300;
        sideLe.flexibleWidth = 0;

        Image sideBg = sidebar.AddComponent<Image>();
        sideBg.color = new Color(0, 0, 0, 0.5f);

        // Sidebar Content
        GameObject titleObj = new GameObject("Title");
        titleObj.transform.SetParent(sidebar.transform, false);
        TextMeshProUGUI titleText = titleObj.AddComponent<TextMeshProUGUI>();
        titleText.text = "BACKLOG";
        titleText.fontSize = 48;
        titleText.color = new Color(1f, 0.6f, 0f); // Orange
        titleText.alignment = TextAlignmentOptions.Center;
        titleText.enableWordWrapping = false;
        
        // Position Title (Simple top align)
        RectTransform titleRT = titleObj.GetComponent<RectTransform>();
        titleRT.anchorMin = new Vector2(0, 1);
        titleRT.anchorMax = new Vector2(1, 1);
        titleRT.pivot = new Vector2(0.5f, 1);
        titleRT.anchoredPosition = new Vector2(0, -50);
        titleRT.sizeDelta = new Vector2(0, 100);

        // 5. Right Content Area (Scroll View)
        GameObject scrollView = new GameObject("Scroll View");
        scrollView.transform.SetParent(mainLayout.transform, false);
        scrollView.AddComponent<RectTransform>();
        // Add ScrollRect
        ScrollRect scrollRect = scrollView.AddComponent<ScrollRect>();
        scrollView.AddComponent<Image>().color = new Color(0,0,0,0.2f); // Slight darken
        scrollView.AddComponent<Mask>().showMaskGraphic = false;

        // Viewport
        GameObject viewport = new GameObject("Viewport");
        viewport.transform.SetParent(scrollView.transform, false);
        RectTransform viewRT = viewport.AddComponent<RectTransform>();
        viewRT.anchorMin = Vector2.zero;
        viewRT.anchorMax = Vector2.one;
        viewRT.pivot = Vector2.up;
        viewRT.sizeDelta = Vector2.zero;
        viewport.AddComponent<Image>().color = new Color(0,0,0,0); // Invisible mask base
        viewport.AddComponent<Mask>().showMaskGraphic = false;

        // Content
        GameObject content = new GameObject("Content");
        content.transform.SetParent(viewport.transform, false);
        RectTransform contentRT = content.AddComponent<RectTransform>();
        contentRT.anchorMin = new Vector2(0, 1);
        contentRT.anchorMax = new Vector2(1, 1);
        contentRT.pivot = new Vector2(0.5f, 1);
        contentRT.sizeDelta = new Vector2(0, 300); // Height grows

        VerticalLayoutGroup contentVLG = content.AddComponent<VerticalLayoutGroup>();
        contentVLG.padding = new RectOffset(20, 20, 20, 20);
        contentVLG.spacing = 30;
        contentVLG.childControlHeight = true;
        contentVLG.childControlWidth = true;
        contentVLG.childForceExpandHeight = false;

        ContentSizeFitter csf = content.AddComponent<ContentSizeFitter>();
        csf.verticalFit = ContentSizeFitter.FitMode.PreferredSize;

        // Link ScrollRect
        scrollRect.content = contentRT;
        scrollRect.viewport = viewRT;
        scrollRect.horizontal = false;
        scrollRect.vertical = true;
        scrollRect.scrollSensitivity = 20;

        // 6. Log Item Prefab (Hidden in scene, to be linked)
        GameObject prefab = new GameObject("LogItemPrefab");
        prefab.transform.SetParent(root.transform, false);
        VerticalLayoutGroup prefabVLG = prefab.AddComponent<VerticalLayoutGroup>();
        prefabVLG.spacing = 5;
        prefabVLG.padding = new RectOffset(10, 10, 10, 10);
        ContentSizeFitter prefabCSF = prefab.AddComponent<ContentSizeFitter>();
        prefabCSF.verticalFit = ContentSizeFitter.FitMode.PreferredSize;
        
        // Background for selection highlight (optional)
        Image prefabBg = prefab.AddComponent<Image>();
        prefabBg.color = new Color(1, 1, 1, 0.05f); // Faint highlight

        // Name Text
        GameObject nameObj = new GameObject("NameText");
        nameObj.transform.SetParent(prefab.transform, false);
        TextMeshProUGUI nameTMP = nameObj.AddComponent<TextMeshProUGUI>();
        nameTMP.fontSize = 28;
        nameTMP.color = new Color(1f, 0.6f, 0f); // Orange
        nameTMP.fontStyle = FontStyles.Bold;

        // Message Text
        GameObject msgObj = new GameObject("MessageText");
        msgObj.transform.SetParent(prefab.transform, false);
        TextMeshProUGUI msgTMP = msgObj.AddComponent<TextMeshProUGUI>();
        msgTMP.fontSize = 24;
        msgTMP.color = Color.white;
        msgTMP.enableWordWrapping = true;

        prefab.SetActive(false); // Hide prefab

        // 7. Setup Controller
        BackLogController controller = root.AddComponent<BackLogController>();
        controller.backLogPanel = root;
        controller.contentContainer = contentRT;
        controller.logItemPrefab = prefab;
        controller.scrollRect = scrollRect;
        
        // Find Close Button (add a simple one)
        GameObject closeBtnObj = new GameObject("CloseButton");
        closeBtnObj.transform.SetParent(root.transform, false);
        RectTransform closeRT = closeBtnObj.AddComponent<RectTransform>();
        closeRT.anchorMin = new Vector2(1, 1);
        closeRT.anchorMax = new Vector2(1, 1);
        closeRT.anchoredPosition = new Vector2(-50, -50);
        closeRT.sizeDelta = new Vector2(50, 50);
        Image closeImg = closeBtnObj.AddComponent<Image>();
        closeImg.color = Color.red;
        Button closeBtn = closeBtnObj.AddComponent<Button>();
        controller.closeButton = closeBtn;

        // 8. Link to AmadeusChatController
        AmadeusChatController chat = FindObjectOfType<AmadeusChatController>();
        if (chat != null)
        {
            chat.backLog = controller;
            EditorUtility.SetDirty(chat);
        }

        Debug.Log("BackLog UI Created Successfully!");
        root.SetActive(false); // Hide by default
    }
}
#endif
