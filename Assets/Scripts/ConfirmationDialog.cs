using System;
using UnityEngine;
using TMPro;
using UnityEngine.UI;
using UnityEngine.InputSystem;

public class ConfirmationDialog : MonoBehaviour
{
    [Header("UI References")]
    public TextMeshProUGUI messageText;
    public Image yesButtonImage;
    public Image noButtonImage;
    public TextMeshProUGUI yesButtonText;
    public TextMeshProUGUI noButtonText;
    public CanvasGroup canvasGroup;
    public Image dialogBox; // Added for Outline target
    public Image borderImage; // Added for outline // Added for fade

    [Header("Settings")]
    public float fadeDuration = 0.2f; 
    public bool useImageTint = false; // Default false as requested
    public bool useTextTint = true;  // Toggle for text color change
    public Color selectedColor = Color.white;
    public Color normalColor = new Color(0.5f, 0.5f, 0.5f, 1f); 
    public Color textSelectedColor = Color.black;
    public Color textNormalColor = Color.white;
    public Color borderColor = new Color(1f, 0.788f, 0f); // #FFC900
    public int borderWidth = 3; // 3px default

    private Action onYesAction;
    private Action onNoAction;
    
    // true = Yes, false = No
    private bool isYesSelected = false; 

    private Coroutine currentFadeCoroutine;

    public bool IsActive => gameObject.activeSelf;

    void Awake()
    {
         if (canvasGroup == null) canvasGroup = GetComponent<CanvasGroup>();
         // Ensure hidden on start
         if (canvasGroup != null) 
         {
             canvasGroup.alpha = 0f;
             canvasGroup.blocksRaycasts = false;
         }
         gameObject.SetActive(false);
    }

    public void Show(string message, Action onYes, Action onNo)
    {
        messageText.text = message;
        onYesAction = onYes;
        onNoAction = onNo;
        
        // Default to check "Yes"
        isYesSelected = true; 
        
        // Generate Border Lines if dialogBox is assigned
        if (dialogBox != null)
        {
            CreateBorderLines();
        }

        gameObject.SetActive(true);
        UpdateVisuals();

        // Fade In
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(0f, 1f));
    }

    public void Hide()
    {
        // Fade Out
        if (gameObject.activeInHierarchy)
        {
            if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
            currentFadeCoroutine = StartCoroutine(FadeCanvas(1f, 0f, true));
        }
        else
        {
            gameObject.SetActive(false);
        }
    }

    void Update()
    {
        if (!IsActive) return;

        HandleInput();
    }

    private void HandleInput()
    {
        // Navigation (Left/Right)
        // Swap: Left Arrow -> No (isYesSelected = false)
        if (Keyboard.current.aKey.wasPressedThisFrame || Keyboard.current.leftArrowKey.wasPressedThisFrame)
        {
            isYesSelected = false;
            UpdateVisuals();
        }
        // Swap: Right Arrow -> Yes (isYesSelected = true)
        else if (Keyboard.current.dKey.wasPressedThisFrame || Keyboard.current.rightArrowKey.wasPressedThisFrame)
        {
            isYesSelected = true;
            UpdateVisuals();
        }

        // Confirm
        if (Keyboard.current.enterKey.wasPressedThisFrame)
        {
            if (isYesSelected)
            {
                OnYes();
            }
            else
            {
                OnNo();
            }
        }
    }

    private void OnYes()
    {
        // Hide(); // Moved to inside actions to allow animation if needed, but here simple hide is ok
        // Fade out then action? Or action immediately?
        // Usually UI actions are immediate logic.
        // Let's hide first.
        Hide();
        onYesAction?.Invoke();
    }

    private void OnNo()
    {
        Hide();
        onNoAction?.Invoke();
    }

    private void UpdateVisuals()
    {
        if (useImageTint)
        {
            if (yesButtonImage) yesButtonImage.color = isYesSelected ? selectedColor : normalColor;
            if (noButtonImage) noButtonImage.color = !isYesSelected ? selectedColor : normalColor;
        }

        if (useTextTint)
        {
            if (yesButtonText) yesButtonText.color = isYesSelected ? textSelectedColor : textNormalColor;
            if (noButtonText) noButtonText.color = !isYesSelected ? textSelectedColor : textNormalColor;
        }
    }

    private void CreateBorderLines()
    {
        // Check if borders already exist
        if (dialogBox.transform.Find("Border_Top") != null) return;

        // Create 4 lines
        CreateLine("Border_Top",    Anchor.Top);
        CreateLine("Border_Bottom", Anchor.Bottom);
        CreateLine("Border_Left",   Anchor.Left);
        CreateLine("Border_Right",  Anchor.Right);
    }

    private enum Anchor { Top, Bottom, Left, Right }

    private void CreateLine(string name, Anchor anchor)
    {
        GameObject lineObj = new GameObject(name);
        lineObj.transform.SetParent(dialogBox.transform, false);
        Image lineImage = lineObj.AddComponent<Image>();
        lineImage.color = borderColor;
        
        RectTransform rt = lineObj.GetComponent<RectTransform>();
        
        switch (anchor)
        {
            case Anchor.Top:
                // Stretch X, Anchor Top
                rt.anchorMin = new Vector2(0, 1);
                rt.anchorMax = new Vector2(1, 1);
                rt.pivot = new Vector2(0.5f, 1);
                rt.sizeDelta = new Vector2(0, borderWidth); // Width 0 means stretch
                rt.anchoredPosition = Vector2.zero;
                break;
            case Anchor.Bottom:
                // Stretch X, Anchor Bottom
                rt.anchorMin = new Vector2(0, 0);
                rt.anchorMax = new Vector2(1, 0);
                rt.pivot = new Vector2(0.5f, 0);
                rt.sizeDelta = new Vector2(0, borderWidth);
                rt.anchoredPosition = Vector2.zero;
                break;
            case Anchor.Left:
                // Stretch Y, Anchor Left
                rt.anchorMin = new Vector2(0, 0);
                rt.anchorMax = new Vector2(0, 1);
                rt.pivot = new Vector2(0, 0.5f);
                rt.sizeDelta = new Vector2(borderWidth, 0); // Height 0 means stretch
                rt.anchoredPosition = Vector2.zero;
                break;
            case Anchor.Right:
                // Stretch Y, Anchor Right
                rt.anchorMin = new Vector2(1, 0);
                rt.anchorMax = new Vector2(1, 1);
                rt.pivot = new Vector2(1, 0.5f);
                rt.sizeDelta = new Vector2(borderWidth, 0);
                rt.anchoredPosition = Vector2.zero;
                break;
        }
    }

    private System.Collections.IEnumerator FadeCanvas(float start, float end, bool disableOnFinish = false)
    {
        if (canvasGroup == null) yield break;

        float t = 0f;
        canvasGroup.alpha = start;
        canvasGroup.blocksRaycasts = (end > 0.5f);

        while (t < fadeDuration)
        {
            t += Time.deltaTime;
            canvasGroup.alpha = Mathf.Lerp(start, end, t / fadeDuration);
            yield return null;
        }

        canvasGroup.alpha = end;
        if (disableOnFinish) gameObject.SetActive(false);
    }
}
