using UnityEngine;
using UnityEngine.UI;
using UnityEngine.EventSystems;
using TMPro;

[RequireComponent(typeof(Toggle))]
public class DropdownHoverText : MonoBehaviour, IPointerEnterHandler, IPointerExitHandler, ISelectHandler, IDeselectHandler
{
    public TextMeshProUGUI itemLabel;
    public Color normalColor = new Color(1f, 0.6f, 0f, 1f); // #FF9900
    public Color hoverColor = Color.white;
    private Toggle toggle;

    void Awake() {
        toggle = GetComponent<Toggle>();
        if (itemLabel == null) itemLabel = GetComponentInChildren<TextMeshProUGUI>();
        UpdateColor(false);
    }

    public void OnPointerEnter(PointerEventData eventData) => UpdateColor(true);
    public void OnPointerExit(PointerEventData eventData) => UpdateColor(toggle.isOn);
    public void OnSelect(BaseEventData eventData) => UpdateColor(true);
    public void OnDeselect(BaseEventData eventData) => UpdateColor(toggle.isOn);

    void UpdateColor(bool isHoverOrSelected) {
        if (itemLabel) itemLabel.color = isHoverOrSelected ? hoverColor : normalColor;
    }
}