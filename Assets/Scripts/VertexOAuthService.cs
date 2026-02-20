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
    // Desktop Flow (Loopback HttpListener + PKCE)
    // ─────────────────────────────────────────
#if !UNITY_WEBGL || UNITY_EDITOR
    private IEnumerator DesktopAuthFlowCoroutine(Action onSuccess, Action<string> onError)
    {
        int port = 50000;
        string redirectUri = $"http://127.0.0.1:{port}/";
        string codeVerifier = GenerateCodeVerifier();
        string codeChallenge = GenerateCodeChallenge(codeVerifier);

        HttpListener listener = null;
        try
        {
            listener = new HttpListener();
            listener.Prefixes.Add(redirectUri);
            listener.Start();
        }
        catch (Exception e)
        {
            onError?.Invoke("認証用ローカルサーバーの起動に失敗しました: " + e.Message);
            if (listener != null) listener.Close();
            yield break;
        }

        string authUrl = $"https://accounts.google.com/o/oauth2/v2/auth?" +
                         $"client_id={DEFAULT_CLIENT_ID}&" +
                         $"redirect_uri={Uri.EscapeDataString(redirectUri)}&" +
                         $"response_type=code&" +
                         $"scope=https://www.googleapis.com/auth/cloud-platform&" +
                         $"code_challenge={codeChallenge}&" +
                         $"code_challenge_method=S256";

        Application.OpenURL(authUrl);

        HttpListenerContext context = null;
        while (true)
        {
            var contextTask = listener.GetContextAsync();
            
            while (!contextTask.IsCompleted)
            {
                yield return null;
            }

            if (contextTask.IsFaulted)
            {
                onError?.Invoke("認証コールバックの待機中にエラーが発生しました。");
                listener.Close();
                yield break;
            }

            context = contextTask.Result;

            // Ignore requests for favicon.ico or other paths
            if (context.Request.Url.AbsolutePath != "/")
            {
                context.Response.StatusCode = 404;
                context.Response.Close();
                continue;
            }

            break; // Valid root request received
        }

        var req = context.Request;
        var res = context.Response;

        string code = req.QueryString.Get("code");
        string error = req.QueryString.Get("error");

        if (!string.IsNullOrEmpty(error))
        {
            SendLocalResponse(res, $"<html><body><h2>認証エラー</h2><p>{error}</p></body></html>");
            listener.Close();
            onError?.Invoke($"OAuth Error: {error}");
            yield break;
        }

        SendLocalResponse(res, "<html><body><h2>認証成功</h2><p>ブラウザを閉じてアプリケーションに戻ってください。</p></body></html>");
        listener.Close();

        if (string.IsNullOrEmpty(code))
        {
            onError?.Invoke("認証コードが取得できませんでした。");
            yield break;
        }

        // Exchange code for token
        yield return StartCoroutine(ExchangeCodeForTokensCoroutine(code, codeVerifier, redirectUri, onSuccess, onError));
    }

    private void SendLocalResponse(HttpListenerResponse res, string text)
    {
        try
        {
            byte[] bytes = Encoding.UTF8.GetBytes(text);
            res.ContentLength64 = bytes.Length;
            res.OutputStream.Write(bytes, 0, bytes.Length);
            res.OutputStream.Close();
            res.Close();
        }
        catch { }
    }

    private IEnumerator ExchangeCodeForTokensCoroutine(string code, string codeVerifier, string redirectUri, Action onSuccess, Action<string> onError)
    {
        string tokenUrl = "https://oauth2.googleapis.com/token";

        WWWForm form = new WWWForm();
        form.AddField("client_id", DEFAULT_CLIENT_ID);
        form.AddField("client_secret", DEFAULT_CLIENT_SECRET);
        form.AddField("code", code);
        form.AddField("code_verifier", codeVerifier);
        form.AddField("redirect_uri", redirectUri);
        form.AddField("grant_type", "authorization_code");

        using (UnityWebRequest req = UnityWebRequest.Post(tokenUrl, form))
        {
            yield return req.SendWebRequest();

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Token Exchange Error: {req.error}\n{req.downloadHandler.text}");
            }
            else
            {
                ProcessTokenResponse(req.downloadHandler.text);
                onSuccess?.Invoke();
            }
        }
    }

    private IEnumerator RefreshAccessTokenCoroutine(string refreshToken, Action<string> onSuccess, Action<string> onError)
    {
        string tokenUrl = "https://oauth2.googleapis.com/token";

        WWWForm form = new WWWForm();
        form.AddField("client_id", DEFAULT_CLIENT_ID);
        form.AddField("client_secret", DEFAULT_CLIENT_SECRET);
        form.AddField("refresh_token", refreshToken);
        form.AddField("grant_type", "refresh_token");

        using (UnityWebRequest req = UnityWebRequest.Post(tokenUrl, form))
        {
            yield return req.SendWebRequest();

            if (req.result != UnityWebRequest.Result.Success)
            {
                Debug.LogWarning($"Token Refresh Failed: {req.error}\n{req.downloadHandler.text}");
                CurrentAccessToken = "";
                CurrentRefreshToken = ""; // Invalidate
                PlayerPrefs.Save();
                onError?.Invoke($"トークンの再取得に失敗しました。再度認証を行ってください。({req.error})");
            }
            else
            {
                ProcessTokenResponse(req.downloadHandler.text);
                onSuccess?.Invoke(CurrentAccessToken);
            }
        }
    }
#endif

    // ─────────────────────────────────────────
    // WebGL Flow (Implicit Flow / hash fragment)
    // ─────────────────────────────────────────
#if UNITY_WEBGL && !UNITY_EDITOR
    private void StartWebGLAuthFlow(Action<string> onError)
    {
        string clientId = PlayerPrefs.GetString(PREF_VERTEX_CLIENT_ID, "");
        if (string.IsNullOrEmpty(clientId))
        {
            onError?.Invoke("WebGL環境では独自のOAuth Client IDの設定が必要です。[Config] → [Vertex Client ID] を設定してください。");
            return;
        }

        string redirectUri = "https://example.com/callback"; // Default fallback
        try
        {
            string url = Application.absoluteURL;
            int hashIndex = url.IndexOf('#');
            if (hashIndex > 0) url = url.Substring(0, hashIndex);
            int queryIndex = url.IndexOf('?');
            if (queryIndex > 0) url = url.Substring(0, queryIndex);
            
            redirectUri = url;
            if (redirectUri.EndsWith(".html")) 
            {
                // Most GCP setups require exact match including/excluding .html
            }
        }
        catch { }

        string authUrl = $"https://accounts.google.com/o/oauth2/v2/auth?" +
                         $"client_id={clientId}&" +
                         $"redirect_uri={Uri.EscapeDataString(redirectUri)}&" +
                         $"response_type=token&" +
                         $"scope=https://www.googleapis.com/auth/cloud-platform";

        Application.OpenURL(authUrl);
    }

    private void ParseWebGLRedirectHash()
    {
        try
        {
            string url = Application.absoluteURL;
            if (string.IsNullOrEmpty(url)) return;

            int hashIdx = url.IndexOf("#");
            if (hashIdx < 0) return;

            string hashObj = url.Substring(hashIdx + 1);
            string[] parts = hashObj.Split('&');
            string token = null;
            string expiresIn = "3600";

            foreach (var p in parts)
            {
                if (p.StartsWith("access_token=")) token = p.Substring(13);
                if (p.StartsWith("expires_in=")) expiresIn = p.Substring(11);
            }

            if (!string.IsNullOrEmpty(token))
            {
                CurrentAccessToken = token;
                // WebGL Implicit Flow doesn't provide refresh token
                CurrentRefreshToken = "";
                
                int expSeconds = 3600;
                int.TryParse(expiresIn, out expSeconds);
                TokenExpiryTicks = DateTime.UtcNow.AddSeconds(expSeconds).Ticks;
                
                PlayerPrefs.Save();
                Debug.Log("[Vertex AI] WebGL Auth successful via URL hash.");
                
                // Optional: remove hash using js
                // Application.ExternalEval("history.replaceState('', document.title, window.location.pathname + window.location.search);");
            }
        }
        catch(Exception e)
        {
            Debug.LogError("Failed to parse WebGL hash: " + e.Message);
        }
    }
    
    // WebGL doesn't use refresh tokens with implicit flow
    private IEnumerator RefreshAccessTokenCoroutine(string refreshToken, Action<string> onSuccess, Action<string> onError)
    {
        onError?.Invoke("WebGLでは自動更新はサポートされていません。");
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
