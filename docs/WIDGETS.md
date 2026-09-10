# Writing widgets

A widget is a folder. The shell's own live in `shell/widgets/<id>/`; yours
live in `~/.config/isle/widgets/<id>/` and are what this document is about.
The dashboard's library lists both, and the store (Browse) lists what other
people have published.

```
~/.config/isle/widgets/pomodoro-timer/
  widget.json        the manifest: what it is, who made it, what it needs
  Widget.qml         what it draws (not needed for a text widget)
  README.md          the listing's text
  CHANGELOG.md       one heading per version
  screenshots/       card.png, taken with Picture in the store
```

The quickest start is the store: open the dashboard, Edit, Browse, the
Yours tab, New widget. That writes the folder above with a manifest that
validates, a `Widget.qml` that says hello, and the README and CHANGELOG
to fill in. Edit the files in any editor; the card redraws the next time
the dashboard opens.

## The manifest

```json
{
  "id": "pomodoro-timer",
  "name": "Pomodoro Timer",
  "glyph": "timer",
  "category": "Yours",
  "description": "A 25-minute focus timer with a break between",
  "version": "1.0.0",
  "author": { "name": "Sam Rivers", "url": "https://example.org/sam" },
  "license": "MIT",
  "homepage": "https://example.org/sam/pomodoro",
  "sizes": ["3x2", "3x4"],
  "default": "3x2",
  "multiple": false,
  "permissions": ["notifications.write"],
  "settings": [
    { "key": "minutes", "label": "Focus length", "type": "number", "min": 5, "max": 90, "default": 25 }
  ]
}
```

| field | meaning |
|---|---|
| `id` | the folder name: lower-case letters, digits and dashes |
| `name`, `description` | what the library and the store show |
| `glyph` | a Lucide icon name from `shell/assets/icons.txt` |
| `category` | Shell, System, Hardware, Media or Yours; published widgets keep the category the author chose |
| `version` | three numbers; the store compares them to offer updates |
| `author` | `{ "name", "url" }`; the name is shown, the url is the link on it |
| `license`, `homepage` | shown on the sheet |
| `sizes` | the spans the card may take, columns × rows on a 12 × 8 grid; a row is an eighth of the dashboard, so a one-row card is a slim strip |
| `default` | the span it is placed at; must be one of `sizes` |
| `min`, `max` | optional bounds for resizing, otherwise the extremes of `sizes` |
| `multiple` | more than one instance makes sense (a second clock) |
| `permissions` | what the widget may reach through `host`, see below |
| `settings` | what the card's gear offers: `toggle`, `choice` (`options: [[value, label]]`), `number` (`min`, `max`) or `text` (`placeholder`) |
| `screenshots` | paths in the folder; when absent, `screenshots/` is used |
| `trust` | `true` asks for the shell's full reach, see Full access |
| `requires` | binaries or services that must be present (`"lact"`), otherwise the widget is hidden |

## Text widgets: no code

A manifest with a `source` and no `Widget.qml` is drawn by the shell's own
renderer: a command (run with `sh`) or a file, read every `interval`
seconds, shown as `view` text, number, gauge (against `max`), sparkline,
or list. Unit and glyph are yours to set. The store's Edit opens a form
for one that exists; a new one is a manifest written by hand.

```json
{ "id": "uptime", "name": "Uptime", "glyph": "clock", "category": "System",
  "description": "How long since boot", "version": "1.0.0",
  "sizes": ["3x2"], "default": "3x2",
  "source": { "command": "uptime -p | sed 's/^up //'", "interval": 30 }, "view": "text" }
```

## QML widgets: the contract

`Widget.qml` is a plain `Item`. The card draws the glass, the title and the
meta; the widget draws its content, filling its parent.

```qml
import QtQuick

Item {
    property var manifest        // the widget.json, parsed
    property string size         // "3x2": the span it is placed at
    property var settings        // the gear's values, defaults filled in
    property var host            // the shell, gated by permissions
    property string title: "Pomodoro"
    property string meta: ""     // the right-hand caption on the card

    Text {
        anchors.centerIn: parent
        text: "25:00"
        color: host.theme.text
        font.family: host.theme.fontMono
        font.pixelSize: 28
    }
}
```

### The sandbox

A widget from someone else runs restricted. The scanner reads its source
before it loads and refuses one that reaches past drawing:

- Imports are limited to `QtQuick`, `QtQuick.Layouts`, `QtQuick.Shapes`,
  `QtQuick.Effects`, `QtQuick.Particles`, `QtQml` and `QtQml.Models`.
  No `qs.*` (the shell), no `Quickshell.*`, no imports of other files.
- No `XMLHttpRequest`, `Qt.createComponent`, `Qt.createQmlObject`,
  `Qt.openUrlExternally`, `Loader` or `WorkerScript`.

Everything else in QtQuick is yours: layouts, shapes, canvas, animation,
timers, text input. This is a source inspection, not a wall; it keeps a
widget honest about what it touches, which is what the permission list is
for.

### `host`

`host` is the only way to the shell. Each service is present when the
manifest asks for it by name and `null` otherwise; a `.write` scope also
grants reading. `host.theme` is always there.

| permission | `host.…` | offers |
|---|---|---|
| always | `theme` | `text`, `text2`, `text3`, `accent`, `ok`, `warn`, `danger`, `surface`, `hairline` (colours), `radius`, `fontUi`, `fontMono` |
| `audio` | `audio` | `volume` (0–1), `muted`, `device` |
| `audio.write` | | `setVolume(v)`, `setMuted(m)` |
| `network` | `network` | `summary`, `online`, `wifi`, `wired` |
| `media` | `media` | `playing`, `present`, `title`, `artist` |
| `media.write` | | `playPause()`, `next()`, `previous()` |
| `system` | `system` | `cpu`, `gpu` (0–1), `memoryUsed`, `memoryTotal` (bytes), `netDown`, `netUp` (bytes/s) |
| `notifications` | `notifications` | `count`, `dnd` |
| `notifications.write` | | `setDnd(on)` |
| `power` | `power` | `onBattery`, `percentage` (−1 without a battery), `profile` |
| `bluetooth` | `bluetooth` | `enabled`, `connected` (a count) |
| `vpn` | `vpn` | `active`, `name` |
| `fetch:<host>` | `fetch(url, done)` | a GET the shell performs to that host (or a subdomain); `done(text, ok)` is called once |
| `notify` | `notify(summary, body)` | a desktop notification under the widget's name |
| always | `store` | the widget's own state, kept per instance across restarts: `get(key, fallback)`, `set(key, value)`, `remove(key)` |

People installing the widget see this list and can switch any entry off;
a switched-off `.write` makes the call a no-op, so read state and draw
from it rather than assuming an action took effect. A `fetch` to a host
that is not granted answers `("", false)`; say so on the card rather than
sitting blank. Name hosts exactly (`fetch:api.open-meteo.com`), one per
service you call; the store shows each one as a pill.

### Full access

`"trust": true` in the manifest asks to run with the shell's whole reach:
`import qs.services` and every singleton, processes, files, the network.
The user's own widgets with it just run. One installed from a source is
installed but does not run until the person grants full access, at
install or later in the sheet, and the store marks it. Ask for it only
when the sandbox truly cannot do the job, say why in the README, and keep
the permission list honest anyway: the store shows both.

A trusted widget may use `WidgetBase` from `qs.ui` as its root, as the
shell's own widgets do, and read the services directly.

## Tools

The store's sheet for one of yours carries an Author row:

- **Check** runs the validator: manifest fields, sizes, glyph, permission
  names, the sandbox scan, and whether the README, CHANGELOG and a
  screenshot are there. Errors stop a listing; notes make a better one.
- **Picture** captures the live card at its default size to
  `screenshots/card.png`, which the store shows in place of a live draw.
- **Tag** turns the folder into a git repository if it is not one, commits
  everything, and tags `v<version>`.

The same tools from a shell:

```
python3 ~/.local/share/isle/shell/scripts/widgetauthor.py new ~/.config/isle/widgets my-widget "My Widget"
python3 ~/.local/share/isle/shell/scripts/widgetauthor.py validate ~/.config/isle/widgets/my-widget
python3 ~/.local/share/isle/shell/scripts/widgetauthor.py share ~/.config/isle/widgets/my-widget
```

## Publishing

A source is a git repository with an `index.json` at its root. The store
clones it shallow and shows what it lists; the default source is
`isle-widgets` beside the shell's own repository, and anyone can add
another under Sources.

```json
{
  "name": "Isle widgets",
  "widgets": [
    { "id": "analog-clock", "path": "widgets/analog-clock" },
    { "id": "pomodoro-timer", "repo": "https://github.com/sam/isle-pomodoro", "ref": "v1.0.0",
      "name": "Pomodoro Timer", "version": "1.0.0", "category": "Yours",
      "description": "A 25-minute focus timer", "author": { "name": "Sam Rivers" },
      "license": "MIT", "sizes": ["3x1", "3x2"], "default": "3x1",
      "permissions": ["notifications.write"], "screenshots": ["previews/pomodoro.png"] }
  ]
}
```

Two ways to be listed:

- **In the registry.** A `path` entry: the widget's folder is committed
  into the registry itself, and its manifest, README, CHANGELOG and
  screenshots are read from there. Nothing else to carry.
- **In your own repository.** A `repo` entry with a `ref`: the store
  clones that tag on install. The index carries the listing, since the
  store does not fetch the repository just to show it; keep `version` in
  step with the tag, and put a screenshot in the registry for the card.

To publish your own: Tag it, push the tag to a repository, and open a pull
request on the registry adding an entry. Bumping `version` in the
manifest, a new CHANGELOG heading and a new tag is a release; people who
installed it see the new version in the store and update from there.
