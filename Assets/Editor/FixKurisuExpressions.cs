using UnityEngine;
using UnityEditor;
using Live2D.Cubism.Framework.Expression;

public class FixKurisuExpressions
{
    [MenuItem("Tools/Amadeus/FixExpressionsV2")]
    public static void Fix()
    {
        var go = GameObject.Find("Live2D紅莉栖forSDK5.0");
        if (go == null) 
        { 
            Debug.LogError("Live2D紅莉栖forSDK5.0 not found in scene."); 
            return; 
        }
        
        var ctrl = go.GetComponent<CubismExpressionController>();
        if (ctrl == null) 
        { 
            Debug.LogError("CubismExpressionController not found."); 
            return; 
        }

        string listPath = "Assets/AmadeusKurisu5.0/reama5.0/reama5.0.expressionList.asset";
        var list = AssetDatabase.LoadAssetAtPath<CubismExpressionList>(listPath);
        if (list == null) 
        { 
            Debug.LogError($"List Asset not found at {listPath}"); 
            return; 
        }

        ctrl.ExpressionsList = list;
        EditorUtility.SetDirty(go); // Mark object as dirty
        // If it's a prefab instance, we might need to record prefab override, but for now just dirtying the scene object is enough for runtime test.
        // If we want to save scene:
        // UnityEditor.SceneManagement.EditorSceneManager.MarkSceneDirty(go.scene);
        
        Debug.Log("Successfully assigned ExpressionsList to Kurisu!");
    }
}
