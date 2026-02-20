using System.Collections;
using UnityEngine;
using TMPro;

public class BootSequenceLineByLine : MonoBehaviour
{
    [SerializeField] private TextMeshProUGUI terminalText;

    [SerializeField] private GameObject loadingPanel;
    [SerializeField] private GameObject mainPanel;

    // ▼追加：ロゴのゲームオブジェクトをインスペクターでセットしてください
    [Header("End Sequence Settings")]
    [SerializeField] private GameObject amadeusLogo;
    [SerializeField] private float waitAfterBoot = 2.0f; // 完了後の待機時間
    [SerializeField] private float waitAfterLogo = 4.0f; // 完了後の待機時間

    [Header("Settings")]
    [SerializeField] private float lineDelay = 0.05f;
    [SerializeField] private float slowDelay = 0.4f;
    [SerializeField] private float startDelay = 0.5f;

    [Header("Memory Count Settings")]
    [SerializeField] private float countDuration = 1.5f;
    [SerializeField] private int maxMemory = 32767;

    [TextArea(10, 20)]
    [SerializeField] private string bootLog = @"
Amadeus System Ver 1.09.2 rev.2123
>>Initialize System ...  OK
>>Detecting boot device ... OK
>>Loading Kerner ...  OK
>>Detecting OS control device ...  OK
>>Booting ...
>>Processor 0 is Activate ...  OK
>>Processor 1 is Activate ...  OK
>>Processor 2 is Activate ...  OK
>>Processor 3 is Activate ...  OK
>>Memory Initialize [MEM]/32767MBytes


INIT: Kernel version 2.04 booting...


ROSS:


Mounting proc at /proc...<pos=30%>[OK]
Mounting sysfs at /sts...<pos=30%>[OK]
Initakising network<pos=30%>[OK]
Setting up localhost ...<pos=30%>[OK]
Setting up inet1 ...<pos=30%>[OK]
Setting up route ...<pos=30%>[OK]
Accessing Croud ...<pos=30%>[OK]
Starting system log at /log/sys...<pos=30%>[OK]
Cleaning /var/lock<pos=30%>[OK]
Cleaning /tmp<pos=30%>[OK]
Updating init.rc<pos=30%>[OK]


Boot Sequences Start...";

    void Start()
    {
        // Check Skip Loading
        if (PlayerPrefs.GetInt("Config_SkipLoading", 0) == 1)
        {
            if (loadingPanel != null) loadingPanel.SetActive(false);
            if (mainPanel != null) mainPanel.SetActive(true);
            if (amadeusLogo != null) amadeusLogo.SetActive(false);
            return; // Skip the rest of Start
        }

        terminalText.maxVisibleCharacters = 0;
        // 最初は [MEM] を 0 に置き換えてセットしておく
        terminalText.text = bootLog.Replace("[MEM]", "0");

        // ▼追加：開始時にロゴが表示されていたら隠す（安全策）
        if (amadeusLogo != null)
        {
            amadeusLogo.SetActive(false);
        }

        StartCoroutine(PlaySequence());
    }

    IEnumerator PlaySequence()
    {
        yield return new WaitForSeconds(startDelay);
        terminalText.ForceMeshUpdate();

        int lineCount = terminalText.textInfo.lineCount;

        for (int i = 0; i < lineCount; i++)
        {
            // --- 特別処理：メモリ行の検出 ---
            int firstCharIdx = terminalText.textInfo.lineInfo[i].firstCharacterIndex;
            int charCount = terminalText.textInfo.lineInfo[i].characterCount;
            string currentLineText = terminalText.text.Substring(firstCharIdx, charCount);

            if (currentLineText.Contains("Memory Initialize"))
            {
                int lastCharIndex = terminalText.textInfo.lineInfo[i].lastVisibleCharacterIndex;
                terminalText.maxVisibleCharacters = lastCharIndex + 1;

                yield return StartCoroutine(AnimateMemoryCount(i));
                yield return new WaitForSeconds(slowDelay);
                continue;
            }

            // --- 通常の行表示処理 ---
            int lineEndIndex = terminalText.textInfo.lineInfo[i].lastVisibleCharacterIndex;
            terminalText.maxVisibleCharacters = lineEndIndex + 1;

            if (currentLineText.Contains("...") || string.IsNullOrWhiteSpace(currentLineText))
            {
                yield return new WaitForSeconds(slowDelay);
            }
            else
            {
                yield return new WaitForSeconds(lineDelay);
            }
        }

        terminalText.maxVisibleCharacters = int.MaxValue;

        // ▼追加：シーケンス完了後の処理
        yield return new WaitForSeconds(waitAfterBoot); // 2秒待機

        terminalText.text = ""; // 文字を消す

        if (amadeusLogo != null)
        {
            amadeusLogo.SetActive(true); // ロゴを表示
            yield return new WaitForSeconds(waitAfterLogo); // 4秒待機
            amadeusLogo.SetActive(false); // ロゴを隠す
            loadingPanel.SetActive(false); // ローディングパネルを隠す
            mainPanel.SetActive(true); // メインパネルを表示
        }
    }

    IEnumerator AnimateMemoryCount(int currentLineIndex)
    {
        float timer = 0f;
        int currentVal = 0;

        while (timer < countDuration)
        {
            timer += Time.deltaTime;
            float progress = timer / countDuration;
            currentVal = (int)Mathf.Lerp(0, maxMemory, progress);

            terminalText.text = bootLog.Replace("[MEM]", currentVal.ToString());
            terminalText.ForceMeshUpdate();

            if (currentLineIndex < terminalText.textInfo.lineCount)
            {
                int lineEndIndex = terminalText.textInfo.lineInfo[currentLineIndex].lastVisibleCharacterIndex;
                terminalText.maxVisibleCharacters = lineEndIndex + 1;
            }

            yield return null;
        }

        terminalText.text = bootLog.Replace("[MEM]", maxMemory.ToString());
        terminalText.ForceMeshUpdate();
        int finalEndIndex = terminalText.textInfo.lineInfo[currentLineIndex].lastVisibleCharacterIndex;
        terminalText.maxVisibleCharacters = finalEndIndex + 1;
    }
}