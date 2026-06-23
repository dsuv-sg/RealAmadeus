using System;
using TMPro;
using UnityEngine;
using UnityEngine.InputSystem;

public class Manager : MonoBehaviour
{
    public string loginId = "Salieri";
    public string password = "MakiseKurisu";
    public TMP_InputField loginIdInputField;
    public TMP_InputField passwordInputField;
    public GameObject loginPanel;
    public GameObject loadingPanel;
    public GameObject mainPanel;
    public TextMeshProUGUI loginErrorText;
    public TMP_FontAsset loginErrorFont;

    private const string PREF_OPERATOR_NAME = "Config_OperatorName";
    private const string ACCESS_DENIED_TEXT = "ACCESS DENIED";

    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Start()
    {
        loginPanel.SetActive(true);
        EnsureLoginErrorText();
        SetLoginErrorVisible(false);
        UpdateLanguage();
        UpdateOperatorDisplay("---");
    }

    // Update is called once per frame
    void Update()
    {
        if(loginPanel.gameObject.activeSelf == true && Keyboard.current != null && Keyboard.current.enterKey.wasPressedThisFrame)
        {
            // NEW: Prevent login via Enter if the menu is open (Unity parity/guard)
            var menu = FindObjectOfType<MenuPanelController>();
            if (menu != null && menu.IsMenuOpen)
            {
                Debug.Log("Login via Enter blocked: Menu is open.");
                return;
            }

            Debug.Log("Enter Login Successful");
            OnLoginButtonPressed();
        }
    }
    public void OnLoginButtonPressed()
    {
        if (loginIdInputField.text == loginId && passwordInputField.text == password)
        {
            Debug.Log("Login Successful");
            SetLoginErrorVisible(false);

            // Save operator name from login input
            string operatorName = "Salieri";
            PlayerPrefs.SetString(PREF_OPERATOR_NAME, operatorName);
            PlayerPrefs.Save();
            UpdateOperatorDisplay(operatorName);

            loadingPanel.SetActive(true);
            loginPanel.SetActive(false);
        }
        else
        {
            Debug.Log("Login Failed");
            SetLoginErrorText(ACCESS_DENIED_TEXT);
        }
    }

    public void Logout()
    {
        Debug.Log("Logging out...");

        // Clear operator name
        PlayerPrefs.DeleteKey(PREF_OPERATOR_NAME);
        PlayerPrefs.Save();
        UpdateOperatorDisplay("---");

        // Clear backlog
        var backLog = FindObjectOfType<BackLogController>(true);
        if (backLog != null)
        {
            backLog.ClearLogs();
        }

        if (mainPanel != null) mainPanel.SetActive(false);
        if (loadingPanel != null) loadingPanel.SetActive(false); // Ensure loading is off
        
        if (loginPanel != null)
        {
            loginPanel.SetActive(true);
            SetLoginErrorVisible(false);
            // Clear inputs
            if (loginIdInputField != null) loginIdInputField.text = "";
            if (passwordInputField != null) passwordInputField.text = "";
        }
    }

private void EnsureLoginErrorText()
    {
        if (loginErrorText == null) return;

        TMP_FontAsset resolvedFont = loginErrorFont != null ? loginErrorFont : loginErrorText.font;
        if (resolvedFont == null) resolvedFont = TMP_Settings.defaultFontAsset;

        if (resolvedFont != null)
        {
            loginErrorText.font = resolvedFont;
        }

        loginErrorText.fontSize = 20f;
        loginErrorText.fontStyle = FontStyles.Bold;
        loginErrorText.color = new Color(1f, 0f, 0f, 1f);
        loginErrorText.alignment = TextAlignmentOptions.Center;
        loginErrorText.enableWordWrapping = false;
        loginErrorText.overflowMode = TextOverflowModes.Overflow;
        loginErrorText.raycastTarget = false;
        loginErrorText.text = ACCESS_DENIED_TEXT;

        // Ensure it is rendered above other login UI elements.
        loginErrorText.transform.SetAsLastSibling();
    }

    private void SetLoginErrorText(string message)
    {
        EnsureLoginErrorText();
        if (loginErrorText == null) return;

        loginErrorText.text = message;
        loginErrorText.gameObject.SetActive(true);
    }

    private void SetLoginErrorVisible(bool isVisible)
    {
        if (loginErrorText == null)
        {
            EnsureLoginErrorText();
        }
        if (loginErrorText != null)
        {
            loginErrorText.gameObject.SetActive(isVisible);
        }
    }

    /// <summary>
    /// Updates the Row_OPERATOR value text in StatusPanel.
    /// </summary>
    private void UpdateOperatorDisplay(string name)
    {
        var statusPanel = FindObjectOfType<StatusPanelController>(true);
        if (statusPanel != null)
        {
            statusPanel.SetOperatorName(name);
        }
    }

    public void UpdateLanguage()
    {
        if (loginPanel == null) return;
        var texts = loginPanel.GetComponentsInChildren<TextMeshProUGUI>(true);
        foreach (var tmp in texts)
        {
            if (tmp == null || string.IsNullOrEmpty(tmp.text)) continue;
            // Skip actual typed input text
            if (tmp.transform.parent != null && tmp.transform.parent.name.Contains("TextArea")) continue;

            string clean = System.Text.RegularExpressions.Regex.Replace(tmp.text, @"<[^>]+>", "").Trim();
            string key = LocalizationManager.Instance.LookupKey(clean);
            if (key != null)
            {
                tmp.text = LocalizationManager.Instance.T(key);
            }
            else if (clean.Equals("ACCESS DENIED", StringComparison.OrdinalIgnoreCase))
            {
                tmp.text = LocalizationManager.Instance.T("login_access_denied", "ACCESS DENIED");
            }
            else if (clean.Equals("USER ID", StringComparison.OrdinalIgnoreCase))
            {
                tmp.text = LocalizationManager.Instance.T("login_user_id", "USER ID");
            }
            else if (clean.Equals("PASSWORD", StringComparison.OrdinalIgnoreCase))
            {
                tmp.text = LocalizationManager.Instance.T("login_password", "PASSWORD");
            }
        }
    }
}
