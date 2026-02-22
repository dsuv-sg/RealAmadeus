using System;
using System.Collections;
using System.Collections.Generic;
using System.Text;
using System.Threading.Tasks;
using UnityEngine;
using UnityEngine.Networking;
using System.Security.Cryptography;

#if !UNITY_WEBGL || UNITY_EDITOR
using System.Net;
#endif

/// <summary>
/// Handles OAuth 2.0 flow for Google Cloud (Vertex AI) to get Access Tokens.
/// Uses Local Loopback (HttpListener) on Desktop and Implicit Flow redirect for WebGL.
/// </summary>
public class VertexOAuthService : MonoBehaviour
{
    private static VertexOAuthService _instance;
    public static VertexOAuthService Instance
    {
        get
        {
            if (_instance == null)
            {
                var go = new GameObject("VertexOAuthService");
                _instance = go.AddComponent<VertexOAuthService>();
                DontDestroyOnLoad(go);
            }
            return _instance;
        }
    }

    // Known gcloud client ID (Works with Loopback flow on Desktop)
    private const string DEFAULT_CLIENT_ID = "32555940559.apps.googleusercontent.com";
    private const string DEFAULT_CLIENT_SECRET = "ZmssLNjJy2998hD4CTg2ejr2";

    public const string PREF_VERTEX_CLIENT_ID = "Config_VertexClientId"; // For WebGL (must be registered in user's GCP)
    public const string PREF_VERTEX_REFRESH_TOKEN = "Config_VertexRefreshToken";
    public const string PREF_VERTEX_ACCESS_TOKEN = "Config_VertexAccessToken";
    public const string PREF_VERTEX_TOKEN_EXPIRY = "Config_VertexTokenExpiry";

    private string CurrentAccessToken
    {
        get => PlayerPrefs.GetString(PREF_VERTEX_ACCESS_TOKEN, "");
        set => PlayerPrefs.SetString(PREF_VERTEX_ACCESS_TOKEN, value);
    }
    private string CurrentRefreshToken
    {
        get => PlayerPrefs.GetString(PREF_VERTEX_REFRESH_TOKEN, "");
        set => PlayerPrefs.SetString(PREF_VERTEX_REFRESH_TOKEN, value);
    }
    private long TokenExpiryTicks
    {
        get => PlayerPrefs.GetString(PREF_VERTEX_TOKEN_EXPIRY, "") != "" ? long.Parse(PlayerPrefs.GetString(PREF_VERTEX_TOKEN_EXPIRY)) : 0;
        set => PlayerPrefs.SetString(PREF_VERTEX_TOKEN_EXPIRY, value.ToString());
    }

    private void Start()
    {
#if UNITY_WEBGL && !UNITY_EDITOR
        ParseWebGLRedirectHash();
#endif
    }

    /// <summary>
    /// Gets a valid access token. Uses cache, refreshes if expired, or errors if not logged in.
    /// </summary>
    public void GetValidAccessToken(Action<string> onSuccess, Action<string> onError)
    {
        if (string.IsNullOrEmpty(CurrentAccessToken) && string.IsNullOrEmpty(CurrentRefreshToken))
        {
            onError?.Invoke("認証されていません。CONFIGからVertex AIの認証を行ってください。");
            return;
        }

        // Check if token is valid (Adding 5 minute buffer)
        DateTime expiry = new DateTime(TokenExpiryTicks);
        if (!string.IsNullOrEmpty(CurrentAccessToken) && DateTime.UtcNow.AddMinutes(5) < expiry)
        {
            onSuccess?.Invoke(CurrentAccessToken);
            return;
        }

        // Needs refresh
        if (!string.IsNullOrEmpty(CurrentRefreshToken))
        {
            StartCoroutine(RefreshAccessTokenCoroutine(CurrentRefreshToken, onSuccess, onError));
        }
        else
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            // WebGL might only have Implicit flow token which cannot be refreshed
            onError?.Invoke("トークンの有効期限が切れました。再度認証を行ってください。");
#else
            onError?.Invoke("リフレッシュトークンがありません。再度認証を行ってください。");
#endif
        }
    }

    /// <summary>
    /// Starts the OAuth flow by opening the browser.
    /// </summary>
    public void Authenticate(Action onSuccess, Action<string> onError)
    {
#if UNITY_WEBGL && !UNITY_EDITOR
        StartWebGLAuthFlow(onError);
#else
        StartCoroutine(DesktopAuthFlowCoroutine(onSuccess, onError));
#endif
    }

    // ─────────────────────────────────────────
    // Desktop Flow (gcloud CLI)
    // ─────────────────────────────────────────
#if !UNITY_WEBGL || UNITY_EDITOR
    private IEnumerator DesktopAuthFlowCoroutine(Action onSuccess, Action<string> onError)
    {
        string token = "";
        string errorOutput = "";
        bool isDone = false;

        // Run gcloud cli in a background thread to avoid freezing Unity
        Task.Run(() =>
        {
            try
            {
                var processInfo = new System.Diagnostics.ProcessStartInfo("cmd", "/c gcloud auth print-access-token")
                {
                    CreateNoWindow = true,
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };

                using (var process = System.Diagnostics.Process.Start(processInfo))
                {
                    token = process.StandardOutput.ReadToEnd().Trim();
                    errorOutput = process.StandardError.ReadToEnd().Trim();
                    process.WaitForExit();
                }
            }
            catch (Exception ex)
            {
                errorOutput = ex.Message;
            }
            finally
            {
                isDone = true;
            }
        });

        // Wait for task to finish
        while (!isDone)
        {
            yield return null;
        }

        if (!string.IsNullOrEmpty(token) && token.StartsWith("ya29."))
        {
            CurrentAccessToken = token;
            CurrentRefreshToken = ""; // Since we fetch a fresh one via CLI, we don't strictly need a refresh token
            
            // Tokens retrieved via CLI are typically valid for 1 hour. We set expiry to 60 mins.
            TokenExpiryTicks = DateTime.UtcNow.AddMinutes(60).Ticks;
            PlayerPrefs.Save();
            
            Debug.Log("[Vertex AI] OAuthトークンをgcloud CLI経由で取得しました。");
            onSuccess?.Invoke();
        }
        else
        {
            string msg = "gcloud CLIでのトークン取得に失敗しました。\n" +
                         "エラー: " + errorOutput + "\n" +
                         "コマンドプロンプトで 'gcloud auth application-default login' または 'gcloud auth login' を実行し、gcloudがインストールされているか確認してください。";
            Debug.LogError(msg);
            onError?.Invoke(msg);
        }
    }

    private IEnumerator RefreshAccessTokenCoroutine(string refreshToken, Action<string> onSuccess, Action<string> onError)
    {
        // For Desktop gcloud flow, "refreshing" is just getting a new token from the CLI
        yield return StartCoroutine(DesktopAuthFlowCoroutine(
            () => onSuccess?.Invoke(CurrentAccessToken), 
            onError
        ));
    }
#endif

    // ─────────────────────────────────────────
    // WebGL Flow (Not Supported)
    // ─────────────────────────────────────────
#if UNITY_WEBGL && !UNITY_EDITOR
    private void StartWebGLAuthFlow(Action<string> onError)
    {
        onError?.Invoke("WebGL環境ではVertex AIはサポートされていません。他のAPIプロバイダーを選択してください。");
    }

    private void ParseWebGLRedirectHash()
    {
        // No-op
    }
    
    // WebGL doesn't use refresh tokens with implicit flow
    private IEnumerator RefreshAccessTokenCoroutine(string refreshToken, Action<string> onSuccess, Action<string> onError)
    {
        onError?.Invoke("WebGL環境ではVertex AIはサポートされていません。");
        yield break;
    }
#endif

    // ─────────────────────────────────────────
    // Utility
    // ─────────────────────────────────────────
    private void ProcessTokenResponse(string json)
    {
        try
        {
            // Simple string extraction to avoid dependency on heavy JSON libs
            string token = ExtractJsonField(json, "access_token");
            string refresh = ExtractJsonField(json, "refresh_token");
            string expires = ExtractJsonField(json, "expires_in");

            if (!string.IsNullOrEmpty(token)) CurrentAccessToken = token;
            if (!string.IsNullOrEmpty(refresh)) CurrentRefreshToken = refresh;
            
            int expSeconds = 3600;
            if (!string.IsNullOrEmpty(expires)) int.TryParse(expires, out expSeconds);
            TokenExpiryTicks = DateTime.UtcNow.AddSeconds(expSeconds).Ticks;

            PlayerPrefs.Save();
            Debug.Log("[Vertex AI] OAuthトークンを取得/更新しました。");
        }
        catch (Exception e)
        {
            Debug.LogError("Error parsing OAuth token response: " + e.Message);
        }
    }

    private string ExtractJsonField(string json, string field)
    {
        int idx = json.IndexOf($"\"{field}\"");
        if (idx < 0) return null;
        int colon = json.IndexOf(":", idx);
        
        // Is it a string or a number?
        int firstChar = -1;
        for (int i = colon + 1; i < json.Length; i++)
        {
            if (json[i] == ' ' || json[i] == '\t' || json[i] == '\n' || json[i] == '\r') continue;
            firstChar = i;
            break;
        }

        if (json[firstChar] == '"') // string
        {
            int end = json.IndexOf("\"", firstChar + 1);
            return json.Substring(firstChar + 1, end - firstChar - 1);
        }
        else // number/boolean
        {
            int comma = json.IndexOf(",", firstChar);
            int brace = json.IndexOf("}", firstChar);
            int end = comma;
            if (end < 0 || (brace > 0 && brace < comma)) end = brace;
            if (end < 0) end = json.Length;

            return json.Substring(firstChar, end - firstChar).Trim();
        }
    }

    // PKCE Generators
#if !UNITY_WEBGL || UNITY_EDITOR
    private string GenerateCodeVerifier()
    {
        byte[] bytes = new byte[32];
        using (var rng = new RNGCryptoServiceProvider())
        {
            rng.GetBytes(bytes);
        }
        return Base64UrlEncode(bytes);
    }

    private string GenerateCodeChallenge(string codeVerifier)
    {
        using (var sha256 = SHA256.Create())
        {
            byte[] challengeBytes = sha256.ComputeHash(Encoding.UTF8.GetBytes(codeVerifier));
            return Base64UrlEncode(challengeBytes);
        }
    }

    private string Base64UrlEncode(byte[] input)
    {
        string base64 = Convert.ToBase64String(input);
        return base64.Replace("+", "-").Replace("/", "_").Replace("=", "");
    }
#endif
}
