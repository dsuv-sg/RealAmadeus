using System.Collections;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UI;
using UnityEngine.EventSystems; // Added for EventSystem
using TMPro;

public class MenuPanelController : MonoBehaviour
{
    [Header("UI References")]
    public CanvasGroup menuCanvasGroup;

    [Header("Animation Settings")]
    public float fadeDuration = 0.3f;

    [Header("Controller References")]
    public SystemPanelController systemPanelController;
    public ConfirmationDialog confirmationDialog;
    public ConfigPanelController configPanelController;
    public StatusPanelController statusPanelController; // Restored Reference
    public BackLogController backLogPanelController; // [NEW] Added Reference
    public ChangeLogPanelController changeLogPanelController; // Added Reference
    public HelpPanelController helpPanelController; // Added Reference
    public AmadeusChatController chatController; // Chat Integration
    public Manager manager;

    [Header("Menu Images (Image Components)")]

    public Image backlogImage;
    public Image configImage;
    public Image statusImage;
    public Image changeLogImage;
    public Image helpImage;

    [Header("Menu Selected Sprites")]
    public Sprite backlogSelectedSprite;
    public Sprite configSelectedSprite;
    public Sprite statusSelectedSprite;
    public Sprite changeLogSelectedSprite;
    public Sprite helpSelectedSprite;

    // Internal storage for default sprites
    private Sprite backlogDefaultSprite;
    private Sprite configDefaultSprite;
    private Sprite statusDefaultSprite;
    private Sprite changeLogDefaultSprite;
    private Sprite helpDefaultSprite;

    [Header("Menu Texts (TMP)")]
    public TextMeshProUGUI fullscreenText;
    public TextMeshProUGUI logoutText;
    public TextMeshProUGUI shutdownText;
    public TextMeshProUGUI closeMenuText;

    [Header("Text Colors")]
    public Color normalTextColor = new Color(0.58f, 0.58f, 0.58f, 1f);
    public Color selectedTextColor = Color.white;

    public TMP_Dropdown screenModeDropdown;

    private Coroutine currentFadeCoroutine;

    /// <summary>Returns true when the menu is visible (alpha >= 0.95).</summary>
    public bool IsMenuOpen => menuCanvasGroup != null && menuCanvasGroup.alpha >= 0.95f;

    // 0:BACKLOG, 1:CONFIG, 2:STATUS, 3:FULLSCREEN, 4:CHANGELOG, 
    // 5:LOGOUT, 6:HELP, 7:SHUTDOWN, 8:CLOSEMENU
    private int selectedIndex = 0;

    private void Start()
    {
        if (chatController == null) chatController = FindObjectOfType<AmadeusChatController>();

        // Store default sprites
        if (backlogImage != null) backlogDefaultSprite = backlogImage.sprite;
        if (configImage != null) configDefaultSprite = configImage.sprite;
        if (statusImage != null) statusDefaultSprite = statusImage.sprite;
        if (changeLogImage != null) changeLogDefaultSprite = changeLogImage.sprite;
        if (helpImage != null) helpDefaultSprite = helpImage.sprite;

        // Initialize UI
        if (menuCanvasGroup == null)
        {
            menuCanvasGroup = GetComponent<CanvasGroup>();
        }

        if (menuCanvasGroup != null)
        {
            menuCanvasGroup.alpha = 0f;
            menuCanvasGroup.interactable = false;
            menuCanvasGroup.blocksRaycasts = false;
            menuCanvasGroup.gameObject.SetActive(true);
        }
        
        UpdateVisuals();
    }

    public bool IsAnySubPanelOpen
    {
        get
        {
            if (confirmationDialog != null && confirmationDialog.IsActive) return true;
            if (configPanelController != null && configPanelController.IsActive) return true;
            if (statusPanelController != null && statusPanelController.IsActive) return true;
            if (changeLogPanelController != null && changeLogPanelController.IsActive) return true;
            if (helpPanelController != null && helpPanelController.IsActive) return true;
            if (backLogPanelController != null && backLogPanelController.IsActive) return true;
            return false;
        }
    }

    void Update()
    {
        // Block input if any sub-panel is open
        if (IsAnySubPanelOpen) return;
        // NOTE: Removed IsInteractionActive guard — when the menu is open, menu input takes priority over dialogue.

        // Only handle input if menu is fully visible
        if (menuCanvasGroup != null && menuCanvasGroup.alpha >= 0.95f)
        {
            HandleInput();
            
            // Confirm selection with Enter (Optional/Legacy support)
            if (Keyboard.current.enterKey.wasPressedThisFrame)
            {
                ExecuteSelection();
            }
        }
    }

    private void HandleInput()
    {
        bool up = Keyboard.current.wKey.wasPressedThisFrame || Keyboard.current.upArrowKey.wasPressedThisFrame;
        bool down = Keyboard.current.sKey.wasPressedThisFrame || Keyboard.current.downArrowKey.wasPressedThisFrame;
        bool left = Keyboard.current.aKey.wasPressedThisFrame || Keyboard.current.leftArrowKey.wasPressedThisFrame;
        bool right = Keyboard.current.dKey.wasPressedThisFrame || Keyboard.current.rightArrowKey.wasPressedThisFrame;

        int nextIndex = selectedIndex;

        switch (selectedIndex)
        {
            case 0: // BACKLOG
                if (down) nextIndex = 1; // -> CONFIG
                if (up) nextIndex = 6;   // -> HELP (Logically wraps or explicitly defined?) Check req: "Backlog W -> Help"
                if (right) nextIndex = 8; // -> CLOSEMENU
                break;

            case 1: // CONFIG
                if (down) nextIndex = 2; // -> STATUS
                if (up) nextIndex = 0;   // -> BACKLOG
                if (right) nextIndex = 8; // -> CLOSEMENU
                break;

            case 2: // STATUS
                if (down) nextIndex = 4; // -> CHANGELOG
                if (up) nextIndex = 1;   // -> CONFIG
                if (left) nextIndex = 3; // -> FULLSCREEN
                if (right) nextIndex = 8; // -> CLOSEMENU
                break;

            case 3: // FULLSCREEN
                // W -> SHUTDOWN
                if (up) nextIndex = 7;
                // A -> CLOSEMENU
                if (left) nextIndex = 8;
                // S -> LOGOUT
                if (down) nextIndex = 5;
                // D or D+W -> STATUS
                if (right) nextIndex = 2; 
                break;

            case 4: // CHANGELOG
                // A and W simultaneous -> FULLSCREEN
                if (IsSimultaneous(Key.A, Key.W) || (left && Keyboard.current.wKey.isPressed) || (up && Keyboard.current.aKey.isPressed))
                {
                    nextIndex = 3; 
                }
                // A or A+S -> LOGOUT
                else if (left || (IsSimultaneous(Key.A, Key.S)))
                {
                    nextIndex = 5; 
                }
                else if (down) nextIndex = 6; // -> HELP
                else if (up) nextIndex = 2;   // -> STATUS
                else if (right) nextIndex = 8; // -> CLOSEMENU
                break;

            case 5: // LOGOUT
                // D and W -> CHANGELOG
                if ((right && Keyboard.current.wKey.isPressed) || (up && Keyboard.current.dKey.isPressed))
                {
                    nextIndex = 4;
                }
                // D and S -> HELP
                else if ((right && Keyboard.current.sKey.isPressed) || (down && Keyboard.current.dKey.isPressed))
                {
                    nextIndex = 6;
                }
                else if (right) nextIndex = 4; // D -> ChangeLog (Request says "D or D+W -> ChangeLog")
                else if (up) nextIndex = 3;    // W -> FULLSCREEN
                else if (left) nextIndex = 8;  // A -> CLOSEMENU
                else if (down) nextIndex = 7;  // S -> SHUTDOWN
                break;

            case 6: // HELP
                 // A and W -> LOGOUT
                if (IsSimultaneous(Key.A, Key.W) || (left && Keyboard.current.wKey.isPressed) || (up && Keyboard.current.aKey.isPressed))
                {
                    nextIndex = 5;
                }
                // A or A+S -> SHUTDOWN
                else if (left || IsSimultaneous(Key.A, Key.S))
                {
                    nextIndex = 7;
                }
                else if (down) nextIndex = 0; // S -> BACKLOG
                else if (up) nextIndex = 4;   // W -> CHANGELOG
                else if (right) nextIndex = 8; // D -> CLOSEMENU
                break;

            case 7: // SHUTDOWN
                if (right) nextIndex = 6; // -> HELP
                if (left) nextIndex = 8;  // -> CLOSEMENU
                if (up) nextIndex = 5;    // -> LOGOUT
                if (down) nextIndex = 3;  // -> FULLSCREEN
                break;

            case 8: // CLOSEMENU
                if (right) nextIndex = 7; // D -> SHUTDOWN
                if (left) nextIndex = 6;  // A -> HELP
                 if (Keyboard.current.enterKey.wasPressedThisFrame) OnCloseMenu();
                break;
        }

        if (nextIndex != selectedIndex)
        {
            selectedIndex = nextIndex;
            UpdateVisuals();
        }

        if (Keyboard.current.backspaceKey.wasPressedThisFrame)
        {
            OnCloseMenu();
        }
    }

    private bool IsSimultaneous(Key key1, Key key2)
    {
         bool k1Pressed = Keyboard.current[key1].wasPressedThisFrame;
         bool k1Held = Keyboard.current[key1].isPressed;
         bool k2Pressed = Keyboard.current[key2].wasPressedThisFrame;
         bool k2Held = Keyboard.current[key2].isPressed;

         return (k1Pressed && k2Held) || (k2Pressed && k1Held);
    }

    private void UpdateVisuals()
    {
        SetImageState(backlogImage, backlogDefaultSprite, backlogSelectedSprite, selectedIndex == 0);
        SetImageState(configImage, configDefaultSprite, configSelectedSprite, selectedIndex == 1);
        SetImageState(statusImage, statusDefaultSprite, statusSelectedSprite, selectedIndex == 2);
        SetImageState(changeLogImage, changeLogDefaultSprite, changeLogSelectedSprite, selectedIndex == 4);
        SetImageState(helpImage, helpDefaultSprite, helpSelectedSprite, selectedIndex == 6);

        SetTextState(fullscreenText, selectedIndex == 3);
        if (fullscreenText != null)
        {
            // Display CURRENT state as requested:
            // "現在の状態がフルスクリーンだったら、MenuPanelのFullScreenTextをFULL\nSCREEN"
            // "現在の状態がウィンドウだったら、MenuPanelのFullScreenTextをWINDOW"
            fullscreenText.text = Screen.fullScreenMode != FullScreenMode.FullScreenWindow ? "WINDOW" : "FULL\nSCREEN";
            screenModeDropdown.value = Screen.fullScreenMode != FullScreenMode.FullScreenWindow ? 0 : 1;
        }

        SetTextState(logoutText, selectedIndex == 5);
        SetTextState(shutdownText, selectedIndex == 7);
        SetTextState(closeMenuText, selectedIndex == 8);
    }

    private void SetImageState(Image img, Sprite def, Sprite sel, bool isSelected)
    {
        if (img == null) return;
        img.sprite = isSelected ? (sel != null ? sel : def) : def;
    }

    private void SetTextState(TextMeshProUGUI txt, bool isSelected)
    {
        if (txt == null) return;
        txt.color = isSelected ? selectedTextColor : normalTextColor;
    }

    private void ExecuteSelection()
    {
         switch (selectedIndex)
        {
            case 0: OnBackLog(); break;
            case 1: OnConfig(); break;
            case 2: OnStatus(); break;
            case 3: OnFullscreen(); break;
            case 4: OnChangeLog(); break;
            case 5: OnLogout(); break; 
            case 6: OnHelp(); break;
            case 7: OnExitSystem(); break;
            case 8: OnCloseMenu(); break;
        }
    }

    public void Show()
    {
        if (menuCanvasGroup == null) return;
        menuCanvasGroup.gameObject.SetActive(true);
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvasGroup(0f, 1f));
        
        // Disable SystemPanel Interaction
        if (systemPanelController != null) systemPanelController.SetInteractable(false);
        // Deselect any active input field
        EventSystem.current.SetSelectedGameObject(null);
        
        UpdateVisuals();
    }

    public void Hide()
    {
        if (menuCanvasGroup == null) return;
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvasGroup(menuCanvasGroup.alpha, 0f, true));
        
        // Enable SystemPanel Interaction
        if (systemPanelController != null) systemPanelController.SetInteractable(true);
    }

    private IEnumerator FadeCanvasGroup(float startAlpha, float endAlpha, bool disableAfterFade = false)
    {
        float elapsedTime = 0f;
        bool isShowing = endAlpha > 0.5f;
        menuCanvasGroup.interactable = isShowing;
        menuCanvasGroup.blocksRaycasts = isShowing;

        while (elapsedTime < fadeDuration)
        {
            elapsedTime += Time.deltaTime;
            float newAlpha = Mathf.Lerp(startAlpha, endAlpha, elapsedTime / fadeDuration);
            menuCanvasGroup.alpha = newAlpha;
            yield return null;
        }

        menuCanvasGroup.alpha = endAlpha;
        if (disableAfterFade)
        {
            menuCanvasGroup.gameObject.SetActive(false);
        }
    }

    // --- Action Handlers ---

    public void OnBackLog()
    {
        Debug.Log("Back Log selected");
        if (backLogPanelController != null)
        {
             backLogPanelController.Show();
             // Note: BackLog handles its own close input (Backspace)
        }
    }

    public void OnConfig()
    {
        Debug.Log("Config selected");
        if (configPanelController != null)
        {
            configPanelController.Show(() => {
                // Return focus to menu when closed
                // UpdateVisuals(); // Optional refresh
            });
        }
    }
    public void OnStatus()
    {
        Debug.Log("Status selected");
        if (statusPanelController != null)
        {
            statusPanelController.Show(() => {
                // Return focus to menu if needed
            });
        }
    }

    public void OnChangeLog()
    {
        Debug.Log("ChangeLog selected");
        if (changeLogPanelController != null)
        {
            changeLogPanelController.Show(() => {
                // Return focus
            });
        }
    }

    public void OnLoad() => Debug.Log("Load selected");
    public void OnTips() => Debug.Log("Tips selected");
    
    public void OnHelp()
    {
        Debug.Log("Help selected");
        if (helpPanelController != null)
        {
            helpPanelController.Show(() => {
                // Return focus
            });
        }
    }
    
    public void OnFullscreen()
    {
        Debug.Log($"Fullscreen toggled. Current: {Screen.fullScreen}");
        if (Screen.fullScreen)
        {
            Screen.fullScreenMode = FullScreenMode.Windowed;

            // Sync with ConfigPanel: 0 = Windowed
            PlayerPrefs.SetInt("Config_ScreenMode", 1);
        }
        else
        {
            Screen.fullScreenMode = FullScreenMode.FullScreenWindow;
            // Sync with ConfigPanel: 2 = Borderless
            PlayerPrefs.SetInt("Config_ScreenMode", 0);
        }
        PlayerPrefs.Save();
        fullscreenText.text = Screen.fullScreenMode == FullScreenMode.FullScreenWindow ? "WINDOW" : "FULL\nSCREEN";
    }

    public void OnReturnTitle() => Debug.Log("Return Title");

    public void OnExitSystem()
    {
        Debug.Log("Exit System Request");
        if (confirmationDialog != null)
        {
            confirmationDialog.Show("ゲームを終了しますか？",
                () => { 
                    Debug.Log("Exiting...");
                    Application.Quit();
                    #if UNITY_EDITOR
                    UnityEditor.EditorApplication.isPlaying = false;
                    #endif
                },
                () => { Debug.Log("Exit Cancelled"); }
            );
        }
        else
        {
            // Fallback if no dialog assigned
            Application.Quit();
        }
    }

    public void OnLogout()
    {
         Debug.Log("Logout Request");
         if (confirmationDialog != null)
         {
             confirmationDialog.Show("ログアウトしますか？",
                () => {
                    Debug.Log("Logging out...");
                    if (manager != null) manager.Logout();
                    OnCloseMenu(); // Ensure menu is closed/hidden
                },
                () => { Debug.Log("Logout Cancelled"); }
             );
         }
         else
         {
             if (manager != null) manager.Logout();
             OnCloseMenu();
         }
    }

    public void OnCloseMenu()
    {
        Debug.Log("Close Menu");
        Hide();
        if (systemPanelController != null)
        {
            systemPanelController.Maximize();
        }
    }
}