import QtQuick
import QtQuick.Controls
import phototux_ui

/// Combo box drawn from `Theme` tokens, including its drop-down list.
///
/// Same reason as `ThemedCheckBox`: with no Controls style configured the shell
/// runs the Basic style, whose hardcoded light palette put white fields and
/// white popup lists into dark editor chrome. Raster editors keep field chrome
/// darker than the panel it sits on, so the closed control reads as sunken.
ComboBox {
    id: control

    /// Row key whose changes band the popup, or "" for an unbanded list.
    ///
    /// A list long enough to need banding (the blend modes are twenty-seven)
    /// reads as a wall of words without one; the separator is drawn above the
    /// first row of each run.
    property string familyRole: ""

    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontBodySm

    background: Rectangle {
        implicitWidth: 96
        implicitHeight: Theme.controlHeight
        color: control.enabled
               ? (control.hovered ? Theme.surfaceContainerHigh : Theme.surfaceSunken)
               : Theme.surfaceContainer
        radius: Theme.radiusSm
        border.width: control.visualFocus ? 2 : 1
        border.color: control.visualFocus ? Theme.focusRing : Theme.borderEffective
    }

    contentItem: Text {
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceXs
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.colorOnSurfaceEffective : Theme.colorOnSurfaceDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    indicator: ThemedIcon {
        x: control.width - width - Theme.spaceXs
        y: control.topPadding + (control.availableHeight - height) / 2
        size: Theme.iconMd
        source: Theme.iconUrl(AppSession.iconRoot, "caret-down")
        color: control.enabled ? Theme.colorOnSurfaceVariant : Theme.iconDisabledEffective
    }

    // The popup owns its rows over the source array rather than sharing the
    // ComboBox's `delegateModel`.
    //
    // Sharing it left the row at `currentIndex` blank on the popup's first open
    // — drawn, sized, but with no text — and neither reading the delegate's own
    // `model`, nor indexing the array, nor re-evaluating on `visible` fixed it.
    // A ListView over the plain array has no shared item to contend for.
    /// Floor for the drop-down's width, in pixels.
    ///
    /// A combo squeezed into half a row is narrower than its own longest label,
    /// and a list that elides to "Pass Thro…" and "Darker Col…" is a list you
    /// cannot read. The closed control may be as narrow as the layout wants;
    /// the open list is sized by its contents.
    property int popupMinimumWidth: 180

    popup: Popup {
        y: control.height
        width: Math.max(control.width, control.popupMinimumWidth)
        height: Math.min(contentItem.implicitHeight + 2, 280)
        padding: 1

        contentItem: ListView {
            id: optionList
            clip: true
            implicitHeight: contentHeight
            model: control.popup.visible ? control.model : null
            currentIndex: control.currentIndex
            ScrollIndicator.vertical: ScrollIndicator {}

            delegate: ItemDelegate {
                id: option
                required property var modelData
                required property int index

                /// This row opens a new band, so it wears the divider.
                readonly property bool bandStart: {
                    if (control.familyRole.length === 0 || index <= 0)
                        return false
                    var rows = optionList.model
                    if (!rows || !rows[index - 1] || !option.modelData)
                        return false
                    return rows[index - 1][control.familyRole]
                            !== option.modelData[control.familyRole]
                }

                width: optionList.width
                implicitHeight: Theme.controlHeight + (bandStart ? 1 : 0)
                topPadding: bandStart ? 1 : 0
                highlighted: index === control.currentIndex

                Rectangle {
                    width: parent.width
                    height: 1
                    color: Theme.borderEffective
                    visible: option.bandStart
                }

                // Opaque: the Basic style paints a light plate behind a
                // delegate, and a transparent background let it through under
                // text coloured for dark chrome.
                background: Rectangle {
                    color: option.highlighted
                           ? Theme.primary
                           : (option.hovered ? Theme.surfaceContainerHigh : Theme.surfaceOverlay)
                }

                contentItem: Text {
                    leftPadding: Theme.spaceSm
                    text: {
                        var row = option.modelData
                        if (row === undefined || row === null)
                            return ""
                        if (control.textRole.length > 0 && row[control.textRole] !== undefined)
                            return row[control.textRole]
                        return row
                    }
                    font.family: control.font.family
                    font.pixelSize: control.font.pixelSize
                    font.weight: option.highlighted ? Font.DemiBold : Font.Normal
                    color: option.highlighted ? Theme.primaryOn : Theme.colorOnSurfaceEffective
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }

                onClicked: {
                    control.currentIndex = option.index
                    control.activated(option.index)
                    control.popup.close()
                }
            }
        }

        background: Rectangle {
            color: Theme.surfaceOverlay
            radius: Theme.radiusSm
            border.width: 1
            border.color: Theme.borderEffective
        }
    }
}
