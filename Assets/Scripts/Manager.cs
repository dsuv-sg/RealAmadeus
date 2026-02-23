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

    private const string PREF_OPERATOR_NAME = "Config_OperatorName";

    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Start()
    {
        loginPanel.SetActive(true);
    }

    // Update is called once per frame
    void Update()
    {
        if(loginPanel.gameObject.activeSelf == true && Keyboard.current != null && Keyboard.current.enterKey.wasPressedThisFrame)
        {
            Debug.Log("Enter Login Successful");
            OnLoginButtonPressed();
        }
    }
    public void OnLoginButtonPressed()
    {
        if (loginIdInputField.text == loginId && passwordInputField.text == password)
        {
            Debug.Log("Login Successful");

            // Save operator name from login input
            string operatorName = loginIdInputField.text;
            PlayerPrefs.SetString(PREF_OPERATOR_NAME, operatorName);
            PlayerPrefs.Save();
            UpdateOperatorDisplay(operatorName);

            loadingPanel.SetActive(true);
            loginPanel.SetActive(false);
        }
        else
        {
            Debug.Log("Login Failed");
            // ログイン失敗時の処理をここに追加
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
            // Clear inputs
            if (loginIdInputField != null) loginIdInputField.text = "";
            if (passwordInputField != null) passwordInputField.text = "";
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
            Transform row = statusPanel.transform.Find("InfoGrid/LeftCol/Row_OPERATOR");
            if (row != null)
            {
                foreach (Transform child in row)
                {
                    if (child.name == "Text" && child.localPosition.x > 0)
                    {
                        var tmp = child.GetComponent<TextMeshProUGUI>();
                        if (tmp != null)
                        {
                            tmp.text = name;
                        }
                        break;
                    }
                }
            }
        }
    }
}
