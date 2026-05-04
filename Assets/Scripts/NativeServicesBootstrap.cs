using UnityEngine;

/// <summary>
/// Bootstraps all native AI services (TTS, STT, RAG) at game startup.
/// No manual scene placement required.
/// </summary>
public class NativeServicesBootstrap : MonoBehaviour
{
    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
    private static void Initialize()
    {
        var go = new GameObject("NativeServices");
        go.AddComponent<NativeTTSService>();
        go.AddComponent<NativeSTTService>();
        go.AddComponent<NativeRAGService>();
        DontDestroyOnLoad(go);
        Debug.Log("[NativeServicesBootstrap] Native TTS/STT/RAG services initialized.");
    }
}
