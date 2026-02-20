#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;
using TMPro;

public class ChatPanelBuilder
{
    [MenuItem("Tools/Amadeus/Create Chat Panel")]
    public static void CreateChatPanel()
    {
        // Find MainPanel inside Canvas/SystemPanel/MainPanel
        GameObject mainPanel = GameObject.Find("Canvas/SystemPanel/MainPanel");
        if (mainPanel == null)
        {
            Debug.LogError("Canvas/SystemPanel/MainPanel not found!");
            return;
        }

        // Clean up existing chat
        Transform existing = mainPanel.transform.Find("ChatSystem");
        if (existing != null) GameObject.DestroyImmediate(existing.gameObject);

        // Try Loading from Assets/Fonts first (User custom path)
        TMP_FontAsset font = AssetDatabase.LoadAssetAtPath<TMP_FontAsset>("Assets/Fonts/MSMINCHO SDF.asset");
        if (font == null)
        {
            // Fallback to Resources
            font = Resources.Load<TMP_FontAsset>("Fonts & Materials/MSMINCHO SDF");
        }
        
        // If still null, try default
        if (font == null)
        {
             font = Resources.Load<TMP_FontAsset>("Fonts & Materials/LiberationSans SDF");
        }

        // === Root Container ===
        GameObject chatRoot = new GameObject("ChatSystem");
        chatRoot.transform.SetParent(mainPanel.transform, false);
        RectTransform rootRT = chatRoot.AddComponent<RectTransform>();
        rootRT.anchorMin = Vector2.zero;
        rootRT.anchorMax = Vector2.one;
        rootRT.offsetMin = Vector2.zero;
        rootRT.offsetMax = Vector2.zero;

        // Add components
        AIService aiService = chatRoot.AddComponent<AIService>();
        AmadeusChatController chatController = chatRoot.AddComponent<AmadeusChatController>();
        chatController.aiService = aiService;

        // === Dialogue Panel (Bottom area, visual novel style) ===
        GameObject dialoguePanel = CreateDialoguePanel(chatRoot.transform, font);
        chatController.dialoguePanel = dialoguePanel;

        // Character Name
        chatController.characterNameText = dialoguePanel.transform.Find("NameArea/NameBg/NameText")?.GetComponent<TextMeshProUGUI>();
        chatController.dialogueText = dialoguePanel.transform.Find("TextArea/DialogueText")?.GetComponent<TextMeshProUGUI>();
        chatController.waitingIndicator = dialoguePanel.transform.Find("TextArea/WaitingIndicator")?.GetComponent<TextMeshProUGUI>();

        // === Input Panel ===
        GameObject inputPanel = CreateInputPanel(chatRoot.transform, font);
        chatController.inputPanel = inputPanel;
        chatController.chatInput = inputPanel.GetComponentInChildren<TMP_InputField>();

        Debug.Log("Chat Panel Created in MainPanel.");
        EditorUtility.SetDirty(chatRoot);
    }

    static GameObject CreateDialoguePanel(Transform parent, TMP_FontAsset font)
    {
        // Panel at the bottom of the screen
        GameObject panel = new GameObject("DialoguePanel");
        panel.transform.SetParent(parent, false);
        RectTransform rt = panel.AddComponent<RectTransform>();
        rt.anchorMin = new Vector2(0, 0);
        rt.anchorMax = new Vector2(1, 0.35f);
        rt.offsetMin = new Vector2(40, 20);
        rt.offsetMax = new Vector2(-40, 0);

        // Background
        Image bg = panel.AddComponent<Image>();
        bg.color = new Color(0, 0, 0, 0.85f);

        // Outline for sci-fi look
        Outline outline = panel.AddComponent<Outline>();
        outline.effectColor = new Color(1f, 0.4f, 0f, 0.4f); // Amber glow
        outline.effectDistance = new Vector2(1, -1);

        VerticalLayoutGroup vlg = panel.AddComponent<VerticalLayoutGroup>();
        vlg.padding = new RectOffset(30, 30, 15, 15);
        vlg.spacing = 5;
        vlg.childControlHeight = true;
        vlg.childControlWidth = true;
        vlg.childForceExpandHeight = false;
        vlg.childForceExpandWidth = true;

        // --- Name Area ---
        GameObject nameArea = new GameObject("NameArea");
        nameArea.transform.SetParent(panel.transform, false);
        HorizontalLayoutGroup nameHlg = nameArea.AddComponent<HorizontalLayoutGroup>();
        nameHlg.childForceExpandWidth = false;
        nameHlg.childControlWidth = true;
        nameHlg.childControlHeight = true;
        LayoutElement nameLE = nameArea.AddComponent<LayoutElement>();
        nameLE.minHeight = 36;
        nameLE.preferredHeight = 36;

        // Name background
        GameObject nameBg = new GameObject("NameBg");
        nameBg.transform.SetParent(nameArea.transform, false);
        Image nameBgImg = nameBg.AddComponent<Image>();
        nameBgImg.color = new Color(1f, 0.4f, 0f, 0.15f);
        LayoutElement nameBgLE = nameBg.AddComponent<LayoutElement>();
        nameBgLE.minWidth = 220;
        nameBgLE.preferredWidth = 220;

        // Name text
        GameObject nameTextObj = new GameObject("NameText");
        nameTextObj.transform.SetParent(nameBg.transform, false);
        TextMeshProUGUI nameTmp = nameTextObj.AddComponent<TextMeshProUGUI>();
        nameTmp.text = "牧瀬 紅莉栖";
        nameTmp.fontSize = 26;
        nameTmp.color = new Color(1f, 0.6f, 0f); // Amber
        nameTmp.alignment = TextAlignmentOptions.MidlineLeft;
        nameTmp.font = font;
        nameTmp.enableWordWrapping = false;
        RectTransform nameTmpRT = nameTextObj.GetComponent<RectTransform>();
        nameTmpRT.anchorMin = Vector2.zero;
        nameTmpRT.anchorMax = Vector2.one;
        nameTmpRT.offsetMin = new Vector2(10, 0);
        nameTmpRT.offsetMax = new Vector2(-10, 0);

        // --- Text Area ---
        GameObject textArea = new GameObject("TextArea");
        textArea.transform.SetParent(panel.transform, false);
        LayoutElement textLE = textArea.AddComponent<LayoutElement>();
        textLE.flexibleHeight = 1;

        // Dialogue text
        GameObject dialogueTextObj = new GameObject("DialogueText");
        dialogueTextObj.transform.SetParent(textArea.transform, false);
        TextMeshProUGUI dialogueTmp = dialogueTextObj.AddComponent<TextMeshProUGUI>();
        dialogueTmp.text = "";
        dialogueTmp.fontSize = 30;
        dialogueTmp.color = Color.white;
        dialogueTmp.alignment = TextAlignmentOptions.TopLeft;
        dialogueTmp.font = font;
        dialogueTmp.enableWordWrapping = true;
        dialogueTmp.overflowMode = TextOverflowModes.Overflow;
        RectTransform dialogueRT = dialogueTextObj.GetComponent<RectTransform>();
        dialogueRT.anchorMin = Vector2.zero;
        dialogueRT.anchorMax = Vector2.one;
        dialogueRT.offsetMin = new Vector2(5, 5);
        dialogueRT.offsetMax = new Vector2(-5, -5);

        // Waiting indicator (bottom right)
        GameObject waitObj = new GameObject("WaitingIndicator");
        waitObj.transform.SetParent(textArea.transform, false);
        TextMeshProUGUI waitTmp = waitObj.AddComponent<TextMeshProUGUI>();
        waitTmp.text = "▼";
        waitTmp.fontSize = 28;
        waitTmp.color = new Color(1f, 0.6f, 0f, 0.8f);
        waitTmp.alignment = TextAlignmentOptions.BottomRight;
        waitTmp.font = font;
        RectTransform waitRT = waitObj.GetComponent<RectTransform>();
        waitRT.anchorMin = Vector2.zero;
        waitRT.anchorMax = Vector2.one;
        waitRT.offsetMin = Vector2.zero;
        waitRT.offsetMax = new Vector2(-10, -5);

        panel.SetActive(false); // Start hidden
        return panel;
    }

    static GameObject CreateInputPanel(Transform parent, TMP_FontAsset font)
    {
        // Input area at bottom
        GameObject panel = new GameObject("InputPanel");
        panel.transform.SetParent(parent, false);
        RectTransform rt = panel.AddComponent<RectTransform>();
        rt.anchorMin = new Vector2(0, 0);
        rt.anchorMax = new Vector2(1, 0);
        rt.pivot = new Vector2(0.5f, 0);
        rt.sizeDelta = new Vector2(-80, 70); // padded
        rt.anchoredPosition = new Vector2(0, 20);

        HorizontalLayoutGroup hlg = panel.AddComponent<HorizontalLayoutGroup>();
        hlg.spacing = 10;
        hlg.childControlHeight = true;
        hlg.childControlWidth = true;
        hlg.childForceExpandWidth = true;
        hlg.padding = new RectOffset(10, 10, 5, 5);

        // Input field
        GameObject inputObj = new GameObject("ChatInput");
        inputObj.transform.SetParent(panel.transform, false);
        Image inputBg = inputObj.AddComponent<Image>();
        inputBg.color = new Color(0.05f, 0.05f, 0.08f, 0.9f);

        // Outline
        Outline inputOutline = inputObj.AddComponent<Outline>();
        inputOutline.effectColor = new Color(1f, 0.4f, 0f, 0.3f);
        inputOutline.effectDistance = new Vector2(1, -1);

        TMP_InputField inputField = inputObj.AddComponent<TMP_InputField>();

        // Text area inside input
        GameObject textAreaObj = new GameObject("Text Area");
        textAreaObj.transform.SetParent(inputObj.transform, false);
        RectTransform taRT = textAreaObj.AddComponent<RectTransform>();
        taRT.anchorMin = Vector2.zero;
        taRT.anchorMax = Vector2.one;
        taRT.offsetMin = new Vector2(15, 5);
        taRT.offsetMax = new Vector2(-15, -5);

        // Input text
        GameObject inputTextObj = new GameObject("Text");
        inputTextObj.transform.SetParent(textAreaObj.transform, false);
        TextMeshProUGUI inputTmp = inputTextObj.AddComponent<TextMeshProUGUI>();
        inputTmp.fontSize = 28;
        inputTmp.color = Color.white;
        inputTmp.font = font;
        inputTmp.enableWordWrapping = false;
        RectTransform inputTmpRT = inputTextObj.GetComponent<RectTransform>();
        inputTmpRT.anchorMin = Vector2.zero;
        inputTmpRT.anchorMax = Vector2.one;
        inputTmpRT.offsetMin = Vector2.zero;
        inputTmpRT.offsetMax = Vector2.zero;

        // Placeholder
        GameObject placeholderObj = new GameObject("Placeholder");
        placeholderObj.transform.SetParent(textAreaObj.transform, false);
        TextMeshProUGUI placeholderTmp = placeholderObj.AddComponent<TextMeshProUGUI>();
        placeholderTmp.text = "Enter Message..."; // Changed to English
        placeholderTmp.fontSize = 28;
        placeholderTmp.color = new Color(0.5f, 0.5f, 0.5f, 0.6f);
        placeholderTmp.font = font;
        placeholderTmp.fontStyle = FontStyles.Italic;
        placeholderTmp.enableWordWrapping = false;
        RectTransform phRT = placeholderObj.GetComponent<RectTransform>();
        phRT.anchorMin = Vector2.zero;
        phRT.anchorMax = Vector2.one;
        phRT.offsetMin = Vector2.zero;
        phRT.offsetMax = Vector2.zero;

        // Link input field components
        inputField.textViewport = taRT;
        inputField.textComponent = inputTmp;
        inputField.placeholder = placeholderTmp;
        inputField.fontAsset = font;
        inputField.pointSize = 28;
        inputField.caretColor = new Color(1f, 0.6f, 0f);
        inputField.selectionColor = new Color(1f, 0.6f, 0f, 0.3f);

        return panel;
    }
}
#endif
