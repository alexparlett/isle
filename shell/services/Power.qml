pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.UPower

// Batteries (controllers, headsets, the laptop if there is one) and the power profile.
Singleton {
    id: root

    readonly property var devices: UPower.devices.values.filter(d => d.isPresent && d.type !== UPowerDeviceType.LinePower)
    readonly property var laptop: UPower.devices.values.find(d => d.isLaptopBattery) || null
    readonly property bool onBattery: UPower.onBattery

    // Peripherals with a battery: what the control panel footer and the Devices widget list.
    readonly property var peripherals: devices.filter(d => !d.isLaptopBattery && d.percentage > 0)

    readonly property string profile: PowerProfiles.profile === PowerProfile.Performance ? "performance"
        : PowerProfiles.profile === PowerProfile.PowerSaver ? "power-saver" : "balanced"
    readonly property string profileLabel: profile === "performance" ? "Performance" : profile === "power-saver" ? "Power saver" : "Balanced"
    readonly property bool hasPerformance: PowerProfiles.hasPerformanceProfile

    function setProfile(name) {
        PowerProfiles.profile = name === "performance" ? PowerProfile.Performance
            : name === "power-saver" ? PowerProfile.PowerSaver : PowerProfile.Balanced;
    }

    function glyphFor(d) {
        switch (d.type) {
        case UPowerDeviceType.GamingInput: return "gamepad-2";
        case UPowerDeviceType.Headset:
        case UPowerDeviceType.Headphones: return "headphones";
        case UPowerDeviceType.Keyboard: return "keyboard";
        case UPowerDeviceType.Mouse: return "mouse";
        case UPowerDeviceType.Phone: return "smartphone";
        default: return "battery";
        }
    }
}
