using UnityEngine;

public class SystemPanelController : MonoBehaviour
{
    public RectTransform systemPanel;
    public float duration = 2.0f;

    // Target states
    private Vector3 defaultScale = Vector3.one;
    private Vector2 defaultPos = Vector2.zero;

    private Vector3 targetScaleVec = new Vector3(0.6f, 0.6f, 0.6f);
    private Vector2 targetPosVec = new Vector2(150f, 12f);

    // 0 = Default, 1 = Active (Target)
    private float currentT = 0f;
    private float targetT = 0f;

    public MenuPanelController sideMenuController;
    public CanvasGroup systemCanvasGroup; // Added CanvasGroup reference

    void Start()
    {
        if (systemPanel != null)
        {
            // Auto-find CanvasGroup if not assigned
            if (systemCanvasGroup == null) systemCanvasGroup = systemPanel.GetComponent<CanvasGroup>();
            // If still null, try adding one? Or just warn? For now, try get from this component if panel is separate
            if (systemCanvasGroup == null) systemCanvasGroup = GetComponent<CanvasGroup>();

            // Initialize with default state (Maximized)
            currentT = 0f;
            targetT = 0f;
            systemPanel.localScale = defaultScale;
            systemPanel.anchoredPosition = defaultPos;
        }
    }

    public void SetInteractable(bool state)
    {
        if (systemCanvasGroup != null)
        {
            systemCanvasGroup.interactable = state;
            systemCanvasGroup.blocksRaycasts = state;
        }
    }

    public void ToggleState()
    {
        targetT = (targetT == 0f) ? 1f : 0f;
        
        // If going back to default (0), hide menu immediately
        if (targetT == 0f)
        {
            if (sideMenuController != null) sideMenuController.Hide();
        }
    }

    public void Maximize()
    {
        targetT = 0f;
        if (sideMenuController != null) sideMenuController.Hide();
    }

    void Update()
    {
        // Toggle target on key press
        if (Input.GetKeyDown(KeyCode.Tab))
        {
            ToggleState();
        }

        // Right Click Toggle (if enabled)
        if (Input.GetMouseButtonDown(1) && PlayerPrefs.GetInt("Config_RightClickMenu", 1) == 1)
        {
             ToggleState();
        }

        // Smoothly move currentT towards targetT
        if (Mathf.Abs(currentT - targetT) > Mathf.Epsilon)
        {
            currentT = Mathf.MoveTowards(currentT, targetT, Time.deltaTime / duration);

            if (systemPanel != null)
            {
                // Interpolate
                systemPanel.localScale = Vector3.Lerp(defaultScale, targetScaleVec, currentT);
                systemPanel.anchoredPosition = Vector2.Lerp(defaultPos, targetPosVec, currentT);
            }

            // Show menu when animation is almost complete and we are targeting active state (1)
            if (targetT == 1f && currentT >= 0.95f)
            {
                 if (sideMenuController != null) sideMenuController.Show();
            }
        }
    }
}
