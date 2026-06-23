using UnityEngine;

/// <summary>
/// Application bootstrap that ensures singleton managers exist at startup.
/// Registers a startup update check, and when a new version is detected,
/// prompts the user with the standard ConfirmationDialog (Logout/Shutdown confirmation).
/// </summary>
public class AppBootstrap : MonoBehaviour
{
    [SerializeField] private bool autoCheckUpdates = true;

    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
    private static void AutoInitialize()
    {
        if (FindObjectOfType<AppBootstrap>(true) == null)
        {
            var go = new GameObject("[AppBootstrap]");
            go.AddComponent<AppBootstrap>();
        }
    }

    void Awake()
    {
        DontDestroyOnLoad(gameObject);

        // Make sure both managers exist before anything else accesses them.
        LocalizationManager.GetOrCreate();

        if (autoCheckUpdates)
        {
            var updateMgr = UpdateManager.GetOrCreate();
            updateMgr.OnUpdateCheckCompleted += OnUpdateCheckCompleted;
            updateMgr.CheckForUpdate();
        }
    }

    void OnDestroy()
    {
        if (UpdateManager.Instance != null)
        {
            UpdateManager.Instance.OnUpdateCheckCompleted -= OnUpdateCheckCompleted;
        }
    }

    private void OnUpdateCheckCompleted(bool success, bool hasUpdate)
    {
        if (!success)
        {
            Debug.Log("[AppBootstrap] Update check failed (offline or rate-limited).");
            return;
        }
        if (!hasUpdate)
        {
            Debug.Log("[AppBootstrap] No update available. Current=" + UpdateManager.Instance.CurrentVersion);
            return;
        }

        Debug.Log("[AppBootstrap] New version available: " + UpdateManager.Instance.LatestVersion);

        // Show the confirmation dialog (uses the game's standard ConfirmationDialog)
        var dialog = FindObjectOfType<ConfirmationDialog>(true);
        if (dialog != null)
        {
            LocalizationManager lm = LocalizationManager.GetOrCreate();
            string template = lm.T("update_confirm", "New version %1 is available.\nDo you want to update?");
            string version = string.IsNullOrEmpty(UpdateManager.Instance.LatestVersion) ? "?" : UpdateManager.Instance.LatestVersion;
            string message = template.Replace("%1", version);

            dialog.Show(
                message,
                () => {
                    int lang = PlayerPrefs.GetInt(LocalizationManager.PREF_LANGUAGE, 0);
                    UpdateManager.Instance.StartUpdate(lang);
                },
                () => {
                    Debug.Log("[AppBootstrap] Update cancelled by user.");
                }
            );
        }
        else
        {
            Debug.LogWarning("[AppBootstrap] ConfirmationDialog not found in scene; cannot show update dialog.");
        }
    }
}
