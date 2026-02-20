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
}
