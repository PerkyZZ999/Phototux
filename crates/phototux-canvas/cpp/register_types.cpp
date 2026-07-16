#include "phototux_canvas_item.h"

#include <QtQml/qqml.h>
#include <atomic>
#include <cstdint>
#include <mutex>

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

struct WgpuDeviceSlot {
    std::atomic<uint64_t> instance{0};
    std::atomic<uint64_t> physicalDevice{0};
    std::atomic<uint64_t> device{0};
    std::atomic<uint32_t> queueFamilyIndex{0};
    std::atomic<uint32_t> queueIndex{0};
    std::atomic<bool> ready{false};
};

WgpuDeviceSlot g_device;
std::mutex g_sharedQueueMutex;
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

extern "C" void phototux_canvas_set_wgpu_device(unsigned long long instance,
                                                unsigned long long physicalDevice,
                                                unsigned long long device,
                                                unsigned int queueFamilyIndex,
                                                unsigned int queueIndex)
{
    g_device.instance.store(instance, std::memory_order_relaxed);
    g_device.physicalDevice.store(physicalDevice, std::memory_order_relaxed);
    g_device.device.store(device, std::memory_order_relaxed);
    g_device.queueFamilyIndex.store(queueFamilyIndex, std::memory_order_relaxed);
    g_device.queueIndex.store(queueIndex, std::memory_order_relaxed);
    g_device.ready.store(true, std::memory_order_release);
}

bool phototux_canvas_get_wgpu_device(quint64 *instance, quint64 *physicalDevice,
                                    quint64 *device, quint32 *queueFamilyIndex,
                                    quint32 *queueIndex)
{
    if (!g_device.ready.load(std::memory_order_acquire))
        return false;
    *instance = g_device.instance.load(std::memory_order_relaxed);
    *physicalDevice = g_device.physicalDevice.load(std::memory_order_relaxed);
    *device = g_device.device.load(std::memory_order_relaxed);
    *queueFamilyIndex = g_device.queueFamilyIndex.load(std::memory_order_relaxed);
    *queueIndex = g_device.queueIndex.load(std::memory_order_relaxed);
    return true;
}

extern "C" void phototux_canvas_lock_shared_queue()
{
    g_sharedQueueMutex.lock();
}

extern "C" void phototux_canvas_unlock_shared_queue()
{
    g_sharedQueueMutex.unlock();
}

// Pull pending export into a canvas item (constructor + each synchronize).
void phototux_canvas_apply_pending_export(PhototuxCanvasItem *item)
{
    if (!item || !g_export.pending.exchange(false, std::memory_order_acq_rel))
        return;
    item->setWgpuImageHandle(g_export.handle.load(std::memory_order_relaxed),
                             g_export.width.load(std::memory_order_relaxed),
                             g_export.height.load(std::memory_order_relaxed),
                             g_export.layout.load(std::memory_order_relaxed));
}
