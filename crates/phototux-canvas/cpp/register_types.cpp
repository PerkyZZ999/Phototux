#include "phototux_canvas_item.h"

#include <QtQml/qqml.h>
#include <atomic>
#include <cstdint>

// C ABI entry points for Rust (phototux binary / phototux_canvas lib).

namespace {
struct WgpuExportSlot {
    std::atomic<uint64_t> handle{0};
    std::atomic<int> width{0};
    std::atomic<int> height{0};
    std::atomic<int> layout{0};
    std::atomic<bool> pending{false};
};

WgpuExportSlot g_export;
} // namespace

extern "C" void phototux_canvas_register_types()
{
    qmlRegisterType<PhototuxCanvasItem>("PhototuxCanvas", 1, 0, "PhototuxCanvas");
}

// Called from Rust after wgpu probe (before or after QML load).
extern "C" void phototux_canvas_set_wgpu_export(unsigned long long handle, int width, int height,
                                               int layout)
{
    g_export.handle.store(handle, std::memory_order_relaxed);
    g_export.width.store(width, std::memory_order_relaxed);
    g_export.height.store(height, std::memory_order_relaxed);
    g_export.layout.store(layout, std::memory_order_relaxed);
    g_export.pending.store(true, std::memory_order_release);
}

// Pull pending export into a canvas item (called from item constructor / show).
void phototux_canvas_apply_pending_export(PhototuxCanvasItem *item)
{
    if (!item || !g_export.pending.load(std::memory_order_acquire))
        return;
    item->setWgpuImageHandle(g_export.handle.load(std::memory_order_relaxed),
                             g_export.width.load(std::memory_order_relaxed),
                             g_export.height.load(std::memory_order_relaxed),
                             g_export.layout.load(std::memory_order_relaxed));
}
