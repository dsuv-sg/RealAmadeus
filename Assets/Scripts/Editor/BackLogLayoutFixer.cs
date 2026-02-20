#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;
using UnityEngine.UI;

public class BackLogLayoutFixer : EditorWindow
{
    [MenuItem("Tools/Fix BackLog Layout")]
    public static void FixLayout()
    {
        GameObject backLogPanel = GameObject.Find("BackLogPanel");
        if (backLogPanel == null)
        {
            Debug.LogError("BackLogPanel not found!");
            return;
        }

        // 1. Adjust Root Panel Margins
        RectTransform rootRT = backLogPanel.GetComponent<RectTransform>();
        if (rootRT != null)
        {
            // Anchors: Full Stretch
            rootRT.anchorMin = Vector2.zero;
            rootRT.anchorMax = Vector2.one;
            
            // Margins: Top 100 (Header), Bottom 40, Left 0, Right 0
            // offsetMin = (Left, Bottom)
            // offsetMax = (-Right, -Top)
            rootRT.offsetMin = new Vector2(0, 40);
            rootRT.offsetMax = new Vector2(0, -100); 
            
            Debug.Log("Updated BackLogPanel RectTransform.");
        }

        // 2. Remove Sidebar (if exists) since user wants full horizontal width
        Transform sidebar = backLogPanel.transform.Find("MainLayout/Sidebar");
        if (sidebar != null)
        {
            // DestroyImmediate(sidebar.gameObject); 
            // Or just disable it to be safe
            sidebar.gameObject.SetActive(false);
            Debug.Log("Disabled Sidebar.");
        }

        // 3. Adjust Scroll View to fill space
        Transform mainLayoutT = backLogPanel.transform.Find("MainLayout");
        if (mainLayoutT != null)
        {
             // Remove padding from Horizontal Layout Group if it exists
             HorizontalLayoutGroup hlg = mainLayoutT.GetComponent<HorizontalLayoutGroup>();
             if (hlg != null)
             {
                 hlg.padding = new RectOffset(20, 20, 0, 0); // Minimal padding
                 hlg.spacing = 0;
             }

             // Find Scroll View
             Transform scrollViewT = mainLayoutT.Find("Scroll View");
             if (scrollViewT != null)
             {
                 // Ensure it expands
                 LayoutElement le = scrollViewT.GetComponent<LayoutElement>();
                 if (le == null) le = scrollViewT.gameObject.AddComponent<LayoutElement>();
                 le.flexibleWidth = 1;
             }
        }
    }
}
#endif
