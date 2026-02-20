using UnityEngine;
using UnityEditor;
using System.Reflection;
using System.Collections;

public class KurisuDebugTool
{
    [MenuItem("Tools/Amadeus/Dump Kurisu Components")]
    public static void Dump()
    {
        string targetName = "Live2D紅莉栖forSDK5.0";
        GameObject go = GameObject.Find(targetName);
        if (go == null)
        {
            Debug.LogError($"'{targetName}' not found in scene.");
            return;
        }

        Debug.Log($"Inspecting {go.name}...");
        Component[] components = go.GetComponents<Component>();
        foreach (var c in components)
        {
            if (c == null) continue;
            string typeName = c.GetType().Name;
            
            if (typeName.Contains("Expression"))
            {
                Debug.Log($"Found Expression Controller: {typeName}");
                DumpProperties(c);
            }
            if (typeName.Contains("Animator"))
            {
                 Debug.Log($"Found Animator");
                 DumpAnimator((Animator)c);
            }
        }
    }

    static void DumpProperties(object obj)
    {
        var type = obj.GetType();
        foreach (var p in type.GetProperties(BindingFlags.Public | BindingFlags.Instance))
        {
            try
            {
                var val = p.GetValue(obj);
                Debug.Log($" [Prop] {p.Name}: {val}");
                
                // If it's the ExpressionsList, try to inspect it
                if (p.Name.Contains("List") && val != null)
                {
                     DumpList(val);
                }
            }
            catch {}
        }
        foreach (var f in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
        {
             try
            {
                var val = f.GetValue(obj);
                Debug.Log($" [Field] {f.Name}: {val}");
                 if (f.Name.Contains("List") && val != null)
                {
                     DumpList(val);
                }
            }
            catch {}
        }
    }

    static void DumpList(object listObj)
    {
        // Try to enumerate
        if (listObj is IEnumerable list)
        {
            int i = 0;
            foreach (var item in list)
            {
                 Debug.Log($"   Item [{i}]: {item}");
                 // If item is an object (Asset), print name
                 if (item is Object unityObj)
                 {
                     Debug.Log($"     -> Name: {unityObj.name}");
                 }
                 i++;
            }
        }
    }
    
    static void DumpAnimator(Animator anim)
    {
        if (anim.runtimeAnimatorController == null)
        {
            Debug.Log("  No RuntimeController");
            return;
        }
        Debug.Log($"  Controller: {anim.runtimeAnimatorController.name}");
        for(int i=0; i<anim.parameterCount; i++)
        {
            var p = anim.GetParameter(i);
            Debug.Log($"    Param: {p.name} ({p.type})");
        }
    }
}
