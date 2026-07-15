#include "spike_canvas_item.h"

#include <rhi/qrhi.h>
#include <QColor>
#include <cmath>
SpikeCanvasItem::SpikeCanvasItem(QQuickItem *parent)
    : QQuickRhiItem(parent)
{
    // Continuous refresh for animated GPU clear
    setMirrorVertically(false);
}

void SpikeCanvasItem::setPhase(float p)
{
    if (qFuzzyCompare(m_phase, p))
        return;
    m_phase = p;
    emit phaseChanged();
    update();
}

void SpikeCanvasItem::setStatusText(const QString &s)
{
    if (m_status == s)
        return;
    m_status = s;
    emit statusTextChanged();
}

QQuickRhiItemRenderer *SpikeCanvasItem::createRenderer()
{
    return new SpikeCanvasRenderer;
}

void SpikeCanvasRenderer::initialize(QRhiCommandBuffer *cb)
{
    Q_UNUSED(cb);
    if (m_rhi)
        return;
    m_rhi = rhi();
}

void SpikeCanvasRenderer::synchronize(QQuickRhiItem *item)
{
    auto *canvas = static_cast<SpikeCanvasItem *>(item);
    m_phase = canvas->phase();
}

void SpikeCanvasRenderer::render(QRhiCommandBuffer *cb)
{
    // GPU-only present path: clear the item color buffer via RHI (Vulkan backend).
    // This proves hybrid C++ QQuickRhiItem + GPU clear without QImage upload.
    // wgpu texture import is attempted from Rust and reported in status / journal.
    if (!m_rhi || !cb)
        return;

    const QColor c = QColor::fromRgbF(
        0.05 + 0.25 * (0.5 + 0.5 * std::sin(double(m_phase))),
        0.15 + 0.35 * (0.5 + 0.5 * std::sin(double(m_phase) + 2.0)),
        0.55 + 0.40 * (0.5 + 0.5 * std::sin(double(m_phase) + 4.0)),
        1.0);

    // No texture upload batch — GPU clear only (not CPU QImage path).
    cb->beginPass(renderTarget(), c, {1.0f, 0}, nullptr);
    cb->endPass();
}
