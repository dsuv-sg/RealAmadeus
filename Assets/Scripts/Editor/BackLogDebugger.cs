#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;
using TMPro;

public class BackLogDebugger : EditorWindow
{
    [MenuItem("Tools/Debug BackLog Items")]
    public static void DebugItems()
    {
        var controller = FindObjectOfType<BackLogController>();
        if (controller == null)
        {
            Debug.LogError("BackLogController not found in scene.");
            return;
        }

        if (controller.contentContainer == null)
        {
            Debug.LogError("BackLogController contentContainer is null.");
            return;
        }

        Debug.Log($"--- BackLog Debugger ---");
        Debug.Log($"BackLog Panel Active: {controller.backLogPanel.activeInHierarchy}");
        Debug.Log($"CanvasGroup Alpha: {(controller.panelCanvasGroup ? controller.panelCanvasGroup.alpha.ToString() : "N/A")}");
        Debug.Log($"Content Container Child Count: {controller.contentContainer.childCount}");

        int i = 0;
        foreach (Transform child in controller.contentContainer)
        {
            Debug.Log($"[Item {i}] Name: {child.name}, Active: {child.gameObject.activeSelf}");
            
            var rect = child.GetComponent<RectTransform>();
            if (rect) Debug.Log($"  Rect: {rect.rect}, Pivot: {rect.pivot}, SizeDelta: {rect.sizeDelta}");

            var texts = child.GetComponentsInChildren<TextMeshProUGUI>(true);
            if (texts.Length == 0) Debug.LogWarning($"  NO TextMeshProUGUI components found in children!");
            
            foreach (var t in texts)
            {
                Debug.Log($"  Text Component: '{t.name}' | Text: '{t.text}' | Color: {t.color} | Alpha: {t.alpha} | FontSize: {t.fontSize}");
                if (t.rectTransform.rect.width <= 0 || t.rectTransform.rect.height <= 0)
                    Debug.LogWarning($"    WARNING: Text '{t.name}' has zero width or height!");
            }
            i++;
        }
        Debug.Log($"--- End Debug ---");
    }
}
#endif
