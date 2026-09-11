// isle-windows: drag-to-edge window tiling, in the compositor.
//
// A floating window dragged to a screen edge (Super+drag or its title bar) tiles there when dropped: a side is
// that half, a corner that quarter, the top the whole work area. The shell draws a translucent target while
// the pointer is over an edge. A tiled window dragged away gets its old size back. Tiles keep an 8px margin
// from the edges and each other and the island's strip at the top, mirroring general:gaps_out, and a window's
// decorations (a title bar above it) fit inside the tile.
//
// The same tiling by key or from the title bar's double click: hyprctl isle snap <zone> | zoom | restore.
//
// A client's own minimise button asks the compositor, whose state handler ignores the request (and, for an X11
// client, re-runs its sticky maximise toggle instead). The request is honoured here by parking the window on
// the shell's hidden workspace, the same as the title bar's minimise button.
//
// The drag is the layout's own: dragController() holds the target while one is in progress, and every drag,
// however it started, ends in CLayoutManager::endDragTarget, which is hooked for the drop.
//
// An X11 client dragging its own title bar asks with _NET_WM_MOVERESIZE, which this Hyprland's XWM does not
// handle; the request is answered here the way the compositor answers xdg_toplevel.move, so a Chromium or
// GTK window under XWayland can be moved and resized by its own controls.
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/output/Monitor.hpp>
#include <hyprland/src/layout/LayoutManager.hpp>
#include <hyprland/src/layout/supplementary/DragController.hpp>
#include <hyprland/src/layout/target/Target.hpp>
#include <hyprland/src/managers/input/InputManager.hpp>
#include <hyprland/src/config/supplementary/executor/Executor.hpp>
#include <hyprland/src/event/EventBus.hpp>
#include <hyprland/src/helpers/signal/Signal.hpp>
#include <hyprland/src/SharedDefs.hpp>
#include <hyprutils/math/Box.hpp>
#include <hyprutils/math/Vector2D.hpp>

#include <unordered_map>
#include <string>
#include <format>
#include <sstream>

// windowForXID and handleClientMessage are private to the X window manager. The standard headers it pulls in
// are included above, so the access change reaches only it.
#define private public
#include <hyprland/src/xwayland/XWM.hpp>
#include <hyprland/src/protocols/XDGShell.hpp>
#undef private
#include <hyprland/src/xwayland/XWayland.hpp>
#include <hyprland/src/xwayland/XSurface.hpp>
#include <hyprland/src/desktop/state/WindowState.hpp>
#include <hyprland/src/desktop/state/FocusState.hpp>
#include <xcb/xproto.h>

inline HANDLE PHANDLE = nullptr;

static CFunctionHook*                                    g_endDragHook   = nullptr;
static SP<SHyprCtlCommand>                               g_ctl;
static CFunctionHook*                                    g_clientMsgHook = nullptr;
static CFunctionHook*                                    g_stateHook     = nullptr;
static CHyprSignalListener                               g_moveCb;
static std::string                                       g_dragZone;
static std::unordered_map<Desktop::View::CWindow*, CBox> g_preTile; // the box a window had before its first tile

constexpr double EDGE = 30;

APICALL EXPORT std::string PLUGIN_API_VERSION() {
    return HYPRLAND_API_VERSION;
}

// general:gaps_out as top, right, bottom, left.
static void readGaps(int& top, int& right, int& bottom, int& left) {
    top = 48; right = 8; bottom = 8; left = 8;
    const auto OUT = HyprlandAPI::invokeHyprctlCommand("getoption", "general:gaps_out");
    const auto POS = OUT.find("css gap data:");
    if (POS == std::string::npos)
        return;
    std::string rest = OUT.substr(POS + 13);
    int v[4], n = 0;
    try {
        size_t i = 0;
        while (n < 4 && i < rest.size()) {
            while (i < rest.size() && !isdigit(rest[i]) && rest[i] != '-') i++;
            if (i >= rest.size()) break;
            size_t j = i;
            while (j < rest.size() && (isdigit(rest[j]) || rest[j] == '-')) j++;
            v[n++] = std::stoi(rest.substr(i, j - i));
            i = j;
        }
    } catch (...) { return; }
    if (n == 1) top = right = bottom = left = v[0];
    else if (n == 4) { top = v[0]; right = v[1]; bottom = v[2]; left = v[3]; }
}

static CBox workArea(PHLMONITOR mon) {
    CBox b = mon->logicalBoxMinusReserved();
    int t, r, bo, l;
    readGaps(t, r, bo, l);
    return CBox(b.x + l, b.y + t, b.w - l - r, b.h - t - bo);
}

static std::string zoneAt(const Vector2D& c, PHLMONITOR mon) {
    const CBox   M  = mon->logicalBox();
    const double x0 = M.x, y0 = M.y, x1 = M.x + M.w, y1 = M.y + M.h;
    const bool   L = c.x <= x0 + EDGE, R = c.x >= x1 - EDGE, T = c.y <= y0 + EDGE, B = c.y >= y1 - EDGE;
    if (L && T) return "top-left";
    if (R && T) return "top-right";
    if (L && B) return "bottom-left";
    if (R && B) return "bottom-right";
    if (L) return "left";
    if (R) return "right";
    if (T) return "up";
    return "";
}

static CBox boxFor(const CBox& wa, const std::string& zone) {
    const double gap = 8, hw = (wa.w - gap) / 2, hh = (wa.h - gap) / 2;
    if (zone == "left")         return CBox(wa.x, wa.y, hw, wa.h);
    if (zone == "right")        return CBox(wa.x + wa.w - hw, wa.y, hw, wa.h);
    if (zone == "top-left")     return CBox(wa.x, wa.y, hw, hh);
    if (zone == "top-right")    return CBox(wa.x + wa.w - hw, wa.y, hw, hh);
    if (zone == "bottom-left")  return CBox(wa.x, wa.y + wa.h - hh, hw, hh);
    if (zone == "bottom-right") return CBox(wa.x + wa.w - hw, wa.y + wa.h - hh, hw, hh);
    return wa;
}

// The tile is where the window and its decorations go; the window itself sits inside by its reserved extents.
static CBox fit(const CBox& tile, const PHLWINDOW& win) {
    const auto EXT = win->getFullWindowReservedArea();
    return CBox(tile.x + EXT.topLeft.x, tile.y + EXT.topLeft.y, tile.w - EXT.topLeft.x - EXT.bottomRight.x, tile.h - EXT.topLeft.y - EXT.bottomRight.y);
}

static void tile(const PHLWINDOW& win, PHLMONITOR mon, const std::string& zone) {
    if (!win || !win->m_target || !mon)
        return;
    if (!g_preTile.contains(win.get()))
        g_preTile[win.get()] = win->m_target->position();
    g_layoutManager->setTargetGeom(fit(boxFor(workArea(mon), zone), win), win->m_target);
}

// The size from before the first tile comes back, at the window's current place (a drop) or its old one (a key).
static void untile(const PHLWINDOW& win, bool atCurrentPlace) {
    if (!win || !win->m_target)
        return;
    const auto IT = g_preTile.find(win.get());
    if (IT == g_preTile.end())
        return;
    const CBox OLD = IT->second, CUR = win->m_target->position();
    g_preTile.erase(IT);
    g_layoutManager->setTargetGeom(atCurrentPlace ? CBox(CUR.x, CUR.y, OLD.w, OLD.h) : OLD, win->m_target);
}

static bool filled(const PHLWINDOW& win, PHLMONITOR mon) {
    const CBox WANT = fit(workArea(mon), win), CUR = win->m_target->position();
    return std::abs(CUR.w - WANT.w) < 2 && std::abs(CUR.h - WANT.h) < 2;
}

static std::string shellPath() {
    const char* SHELL = getenv("ISLE_SHELL_PATH");
    const char* HOME  = getenv("HOME");
    return SHELL ? SHELL : std::string(HOME ? HOME : "") + "/.config/quickshell/isle";
}

// The shell draws the preview; the rect is "x y w h", empty to clear. Called on a zone change only.
static void ghost(const std::string& rect) {
    if (Config::Supplementary::executor())
        Config::Supplementary::executor()->spawn("qs -p \"" + shellPath() + "\" ipc call windows ghost \"" + rect + "\"");
}

static void setGhost(const std::string& zone, PHLMONITOR mon) {
    if (zone == g_dragZone)
        return;
    g_dragZone = zone;
    if (zone.empty()) {
        ghost("");
        return;
    }
    const CBox B = boxFor(workArea(mon), zone);
    ghost(std::format("{} {} {} {}", (int)B.x, (int)B.y, (int)B.w, (int)B.h));
}

// The floating window being moved right now, if any. A drag in progress is the controller holding a target in
// move mode, the test changeMouseBindMode itself uses; dragThresholdReached is not part of it, as
// handleKeybinds resets it after the mouse bind that starts a Super+drag.
static PHLWINDOW movingWindow() {
    if (!g_layoutManager)
        return nullptr;
    const auto& DC = g_layoutManager->dragController();
    if (!DC || DC->mode() != MBIND_MOVE)
        return nullptr;
    const auto TGT = DC->target();
    if (!TGT || !TGT->floating())
        return nullptr;
    return TGT->window();
}

static void onMouseMove(const Vector2D& c) {
    const auto WIN = movingWindow();
    const auto MON = WIN ? WIN->m_monitor.lock() : nullptr;
    if (!MON) {
        if (!g_dragZone.empty()) { g_dragZone.clear(); ghost(""); }
        return;
    }
    setGhost(zoneAt(c, MON), MON);
}

// The drop. The layout's own end-of-drag runs first, so the tile is applied to a settled window.
typedef void (*endDragFn)(void*);
static void hkEndDragTarget(void* thisptr) {
    const auto WIN  = movingWindow();
    const auto MON  = WIN ? WIN->m_monitor.lock() : nullptr;
    const auto ZONE = MON ? zoneAt(g_pInputManager->getMouseCoordsInternal(), MON) : std::string();

    (*(endDragFn)g_endDragHook->m_original)(thisptr);

    if (!g_dragZone.empty()) { g_dragZone.clear(); ghost(""); }
    if (!WIN || !MON || !WIN->m_target)
        return;

    if (!ZONE.empty())
        tile(WIN, MON, ZONE);
    else
        untile(WIN, true);
}

// _NET_WM_MOVERESIZE: data32[2] is the direction, 0..7 a resize edge clockwise from top-left, 8 a move, 11 a cancel.
static std::optional<Layout::eRectCorner> x11ResizeCorner(uint32_t dir) {
    using namespace Layout;
    switch (dir) {
        case 0: return CORNER_TOPLEFT;
        case 1: return CORNER_TOP;
        case 2: return CORNER_TOPRIGHT;
        case 3: return CORNER_RIGHT;
        case 4: return CORNER_BOTTOMRIGHT;
        case 5: return CORNER_BOTTOM;
        case 6: return CORNER_BOTTOMLEFT;
        case 7: return CORNER_LEFT;
        default: return std::nullopt;
    }
}

static void onX11MoveResize(CXWM* wm, xcb_client_message_event_t* e) {
    const auto SURF = wm->windowForXID(e->window);
    if (!SURF)
        return;
    PHLWINDOW WIN;
    for (const auto& w : Desktop::windowState()->windows()) {
        if (w->m_xwaylandSurface == SURF) {
            WIN = w;
            break;
        }
    }
    if (!WIN || !WIN->m_isMapped || WIN->isHidden() || !g_layoutManager)
        return;
    const uint32_t DIR = e->data.data32[2];
    const auto&    DC  = g_layoutManager->dragController();
    if (DIR == 11) {
        if (DC->target() && DC->target() == WIN->layoutTarget())
            g_layoutManager->endDragTarget();
        return;
    }
    if (DC->target())
        return;
    if (DIR == 8 || DIR == 10)
        g_layoutManager->beginDragTarget(WIN->layoutTarget(), MBIND_MOVE, std::nullopt, true);
    else if (const auto CORNER = x11ResizeCorner(DIR))
        g_layoutManager->beginDragTarget(WIN->layoutTarget(), MBIND_RESIZE, CORNER, true);
}

// hyprctl isle snap <left|right|up|down|top-left|top-right|bottom-left|bottom-right> | zoom | restore
static std::string ctl(eHyprCtlOutputFormat, std::string req) {
    const auto WIN = Desktop::focusState()->window();
    const auto MON = WIN ? WIN->m_monitor.lock() : nullptr;
    if (!WIN || !MON || !WIN->m_isFloating || !WIN->m_target)
        return "no floating window";
    std::string args = req.size() > 4 ? req.substr(5) : "";
    if (args == "zoom")
        args = filled(WIN, MON) ? "restore" : "up";
    if (args == "restore" || args == "down") {
        untile(WIN, false);
        return "ok";
    }
    const auto ZONE = args.starts_with("snap ") ? args.substr(5) : args;
    if (ZONE != "left" && ZONE != "right" && ZONE != "up" && ZONE != "top-left" && ZONE != "top-right" && ZONE != "bottom-left" && ZONE != "bottom-right")
        return "unknown zone";
    tile(WIN, MON, ZONE);
    return "ok";
}


typedef void (*updateStateFn)(void*);
static void hkOnUpdateState(void* thisptr) {
    auto*      w   = (Desktop::View::CWindow*)thisptr;
    const auto WIN = w->m_self.lock();
    std::optional<bool>* minimize = nullptr;
    if (WIN && WIN->m_xdgSurface && WIN->m_xdgSurface->m_toplevel)
        minimize = &WIN->m_xdgSurface->m_toplevel->m_state.requestsMinimize;
    else if (WIN && WIN->m_xwaylandSurface)
        minimize = &WIN->m_xwaylandSurface->m_state.requestsMinimize;
    if (minimize && minimize->value_or(false) && WIN->m_isMapped) {
        // Consumed: xdg has no unset, and a later state change must not park the window again.
        *minimize = false;
        if (Config::Supplementary::executor())
            Config::Supplementary::executor()->spawn(std::format("python3 \"{}/scripts/hidewindow.py\" 0x{:x}", shellPath(), (uintptr_t)w));
        return;
    }
    (*(updateStateFn)g_stateHook->m_original)(thisptr);
}

typedef void (*clientMsgFn)(void*, xcb_client_message_event_t*);
static void hkHandleClientMessage(void* thisptr, xcb_client_message_event_t* e) {
    if (e && e->type == HYPRATOMS["_NET_WM_MOVERESIZE"])
        onX11MoveResize((CXWM*)thisptr, e);
    (*(clientMsgFn)g_clientMsgHook->m_original)(thisptr, e);
}

APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE handle) {
    PHANDLE = handle;

    const std::string HASH        = __hyprland_api_get_hash();
    const std::string CLIENT_HASH = __hyprland_api_get_client_hash();
    if (HASH != CLIENT_HASH) {
        HyprlandAPI::addNotification(PHANDLE, "[isle-windows] version mismatch", CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
        throw std::runtime_error("[isle-windows] version mismatch");
    }

    // An override-redirect X11 window (a menu, a tooltip, a combo list) is tagged x11popup as it opens, so the
    // config's rule on that tag spares it the animations and decorations a window gets; the rule engine has no
    // matcher for override-redirect itself. Only dynamic rules can key on the tag: static ones, float among
    // them, are read at map, before this hook runs.
    static auto P_POPUP = Event::bus()->m_events.window.openEarly.listen([](PHLWINDOW w) {
        if (!w || !w->m_isX11 || !w->m_ruleApplicator || !w->isX11OverrideRedirect())
            return;
        w->m_ruleApplicator->m_tagKeeper.applyTag("x11popup", true);
        w->m_ruleApplicator->propertiesChanged(Desktop::Rule::RULE_PROP_TAG);
    });

    for (const auto& fn : HyprlandAPI::findFunctionsByName(PHANDLE, "endDragTarget")) {
        if (!fn.demangled.contains("CLayoutManager"))
            continue;
        g_endDragHook = HyprlandAPI::createFunctionHook(PHANDLE, fn.address, (void*)::hkEndDragTarget);
        break;
    }
    if (!g_endDragHook || !g_endDragHook->hook()) {
        HyprlandAPI::addNotification(PHANDLE, "[isle-windows] could not hook endDragTarget", CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
        throw std::runtime_error("[isle-windows] hook failed");
    }
    for (const auto& fn : HyprlandAPI::findFunctionsByName(PHANDLE, "onUpdateState")) {
        if (!fn.demangled.contains("CWindow"))
            continue;
        g_stateHook = HyprlandAPI::createFunctionHook(PHANDLE, fn.address, (void*)::hkOnUpdateState);
        break;
    }
    if (!g_stateHook || !g_stateHook->hook()) {
        HyprlandAPI::addNotification(PHANDLE, "[isle-windows] could not hook the window state handler", CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
        throw std::runtime_error("[isle-windows] hook failed");
    }
    for (const auto& fn : HyprlandAPI::findFunctionsByName(PHANDLE, "handleClientMessage")) {
        if (!fn.demangled.contains("CXWM"))
            continue;
        g_clientMsgHook = HyprlandAPI::createFunctionHook(PHANDLE, fn.address, (void*)::hkHandleClientMessage);
        break;
    }
    if (!g_clientMsgHook || !g_clientMsgHook->hook()) {
        HyprlandAPI::addNotification(PHANDLE, "[isle-windows] could not hook the X11 client-message handler", CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
        throw std::runtime_error("[isle-windows] hook failed");
    }

    g_moveCb = Event::bus()->m_events.input.mouse.move.listen([](Vector2D c, Event::SCallbackInfo&) { onMouseMove(c); });
    g_ctl    = HyprlandAPI::registerHyprCtlCommand(PHANDLE, SHyprCtlCommand{.name = "isle", .exact = false, .fn = ::ctl});

    return {"isle-windows", "Drag-to-edge tiling for the Isle shell", "Alex Parlett", "1.0"};
}

APICALL EXPORT void PLUGIN_EXIT() {
    g_moveCb.reset();
    if (g_ctl)
        HyprlandAPI::unregisterHyprCtlCommand(PHANDLE, g_ctl);
    if (g_endDragHook)
        g_endDragHook->unhook();
    if (g_clientMsgHook)
        g_clientMsgHook->unhook();
    if (g_stateHook)
        g_stateHook->unhook();
    g_preTile.clear();
}
