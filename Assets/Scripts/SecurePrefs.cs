using System;
using System.Security.Cryptography;
using System.Text;
using UnityEngine;

public static class SecurePrefs
{
    private const string ProtectedPrefix = "enc:v1:";

    public static string GetProtectedString(string key, string defaultValue = "")
    {
        string raw = PlayerPrefs.GetString(key, "");
        if (string.IsNullOrEmpty(raw))
        {
            return defaultValue;
        }

        if (!raw.StartsWith(ProtectedPrefix, StringComparison.Ordinal))
        {
            return raw;
        }

        try
        {
            byte[] payload = Convert.FromBase64String(raw.Substring(ProtectedPrefix.Length));

/*
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
            try
            {
                byte[] unprotected = ProtectedData.Unprotect(payload, null, DataProtectionScope.CurrentUser);
                return Encoding.UTF8.GetString(unprotected);
            }
            catch
            {
                // Fall back to cross-platform decryption if DPAPI payload is not available.
            }
#endif
*/

            return DecryptFallback(payload);
        }
        catch (Exception ex)
        {
            Debug.LogWarning($"[SecurePrefs] Failed to decrypt key '{key}': {ex.Message}");
            return defaultValue;
        }
    }

    public static void SetProtectedString(string key, string value)
    {
        if (string.IsNullOrEmpty(value))
        {
            PlayerPrefs.DeleteKey(key);
            return;
        }

        byte[] utf8 = Encoding.UTF8.GetBytes(value);
        byte[] protectedBytes;

/*
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        try
        {
            protectedBytes = ProtectedData.Protect(utf8, null, DataProtectionScope.CurrentUser);
            PlayerPrefs.SetString(key, ProtectedPrefix + Convert.ToBase64String(protectedBytes));
            return;
        }
        catch
        {
            // Fall back to cross-platform encryption.
        }
#endif
*/

        protectedBytes = EncryptFallback(value);
        PlayerPrefs.SetString(key, ProtectedPrefix + Convert.ToBase64String(protectedBytes));
    }

    private static byte[] EncryptFallback(string plainText)
    {
        byte[] key = DeriveKey();
        using (Aes aes = Aes.Create())
        {
            aes.Key = key;
            aes.Mode = CipherMode.CBC;
            aes.Padding = PaddingMode.PKCS7;
            aes.GenerateIV();

            using (ICryptoTransform encryptor = aes.CreateEncryptor())
            {
                byte[] plain = Encoding.UTF8.GetBytes(plainText);
                byte[] cipher = encryptor.TransformFinalBlock(plain, 0, plain.Length);
                byte[] output = new byte[aes.IV.Length + cipher.Length];
                Buffer.BlockCopy(aes.IV, 0, output, 0, aes.IV.Length);
                Buffer.BlockCopy(cipher, 0, output, aes.IV.Length, cipher.Length);
                return output;
            }
        }
    }

    private static string DecryptFallback(byte[] payload)
    {
        if (payload.Length <= 16)
        {
            return string.Empty;
        }

        byte[] iv = new byte[16];
        byte[] cipher = new byte[payload.Length - 16];
        Buffer.BlockCopy(payload, 0, iv, 0, iv.Length);
        Buffer.BlockCopy(payload, iv.Length, cipher, 0, cipher.Length);

        using (Aes aes = Aes.Create())
        {
            aes.Key = DeriveKey();
            aes.IV = iv;
            aes.Mode = CipherMode.CBC;
            aes.Padding = PaddingMode.PKCS7;

            using (ICryptoTransform decryptor = aes.CreateDecryptor())
            {
                byte[] plain = decryptor.TransformFinalBlock(cipher, 0, cipher.Length);
                return Encoding.UTF8.GetString(plain);
            }
        }
    }

    private static byte[] DeriveKey()
    {
        string fingerprint = string.Join("|",
            Application.companyName,
            Application.productName,
            SystemInfo.deviceUniqueIdentifier,
            Environment.UserName);

        using (SHA256 sha = SHA256.Create())
        {
            return sha.ComputeHash(Encoding.UTF8.GetBytes(fingerprint));
        }
    }
}
