#include "phototux_canvas_item.h"

#include <rhi/qrhi.h>
#include <QFile>
#include <QColor>
#include <cmath>
#include <cstdio>
#include <cstring>

// Embedded qsb paths are written next to the library via build.rs OUT_DIR symlink-ish:
// build.rs copies baked shaders into OUT_DIR and defines PHOTOTUX_SHADER_DIR.
#ifndef PHOTOTUX_SHADER_DIR
#define PHOTOTUX_SHADER_DIR "."
#endif

namespace {

struct Vertex {
    float x, y;
    float u, v;
};

struct UBuf {
    float color[4];
    float params[4];
};

static QByteArray loadShader(const char *name)
{
    const QString path = QString::fromUtf8(PHOTOTUX_SHADER_DIR) + QLatin1Char('/') + QLatin1String(name);
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly))
        return {};
    return f.readAll();
}

// Map document screen rect → NDC verts for a triangle strip (TL, TR, BL, BR).
static void fillDocQuad(Vertex out[4], float itemW, float itemH,
                        float panX, float panY, float zoom,
                        float docW, float docH)
{
    if (itemW < 1.f)
        itemW = 1.f;
    if (itemH < 1.f)
        itemH = 1.f;
    const float cx = itemW * 0.5f;
    const float cy = itemH * 0.5f;
    // world (0,0) and (docW,docH) → screen
    const float x0 = (0.f - panX) * zoom + cx;
    const float y0 = (0.f - panY) * zoom + cy;
    const float x1 = (docW - panX) * zoom + cx;
    const float y1 = (docH - panY) * zoom + cy;

    auto toNdc = [&](float sx, float sy, float &nx, float &ny) {
        nx = (sx / itemW) * 2.f - 1.f;
        ny = 1.f - (sy / itemH) * 2.f; // Y-down screen → Y-up NDC
    };

    float nx0, ny0, nx1, ny1;
    toNdc(x0, y0, nx0, ny0);
    toNdc(x1, y1, nx1, ny1);

    out[0] = {nx0, ny0, 0.f, 0.f};
    out[1] = {nx1, ny0, 1.f, 0.f};
    out[2] = {nx0, ny1, 0.f, 1.f};
    out[3] = {nx1, ny1, 1.f, 1.f};
}

} // namespace

PhototuxCanvasItem::PhototuxCanvasItem(QQuickItem *parent)
    : QQuickRhiItem(parent)
{
    setMirrorVertically(false);
    // Continuous frames while visible so FPS gate can be measured.
    setSampleCount(1);
    phototux_canvas_apply_pending_export(this);
}

void PhototuxCanvasItem::setZoom(float z)
{
    if (qFuzzyCompare(m_zoom, z))
        return;
    m_zoom = z;
    emit zoomChanged();
    update();
}

void PhototuxCanvasItem::setPanX(float v)
{
    if (qFuzzyCompare(m_panX, v))
        return;
    m_panX = v;
    emit panXChanged();
    update();
}

void PhototuxCanvasItem::setPanY(float v)
{
    if (qFuzzyCompare(m_panY, v))
        return;
    m_panY = v;
    emit panYChanged();
    update();
}

void PhototuxCanvasItem::setDocWidth(int w)
{
    if (m_docW == w)
        return;
    m_docW = w;
    emit docWidthChanged();
    update();
}

void PhototuxCanvasItem::setDocHeight(int h)
{
    if (m_docH == h)
        return;
    m_docH = h;
    emit docHeightChanged();
    update();
}

void PhototuxCanvasItem::setHasDocument(bool v)
{
    if (m_hasDoc == v)
        return;
    m_hasDoc = v;
    emit hasDocumentChanged();
    update();
}

void PhototuxCanvasItem::setPhase(float p)
{
    if (qFuzzyCompare(m_phase, p))
        return;
    m_phase = p;
    emit phaseChanged();
    update();
}

void PhototuxCanvasItem::setGpuStatus(const QString &s)
{
    if (m_gpuStatus == s)
        return;
    m_gpuStatus = s;
    emit gpuStatusChanged();
}

void PhototuxCanvasItem::setFps(float f)
{
    if (qFuzzyCompare(m_fps, f))
        return;
    m_fps = f;
    emit fpsChanged();
}

void PhototuxCanvasItem::setWgpuImageHandle(quint64 handle, int width, int height, int layout)
{
    m_wgpuHandle = handle;
    m_wgpuW = width;
    m_wgpuH = height;
    m_wgpuLayout = layout;
    m_wgpuDirty = true;
    update();
}

QQuickRhiItemRenderer *PhototuxCanvasItem::createRenderer()
{
    return new PhototuxCanvasRenderer;
}

PhototuxCanvasRenderer::~PhototuxCanvasRenderer()
{
    releaseResources();
}

void PhototuxCanvasRenderer::releaseResources()
{
    delete m_pipeline;
    m_pipeline = nullptr;
    delete m_srb;
    m_srb = nullptr;
    delete m_ubuf;
    m_ubuf = nullptr;
    delete m_vbuf;
    m_vbuf = nullptr;
}

void PhototuxCanvasRenderer::initialize(QRhiCommandBuffer *cb)
{
    Q_UNUSED(cb);
    if (m_rhi)
        return;
    m_rhi = rhi();
    m_vertShader = loadShader("canvas.vert.qsb");
    m_fragShader = loadShader("canvas.frag.qsb");
}

void PhototuxCanvasRenderer::synchronize(QQuickRhiItem *item)
{
    auto *canvas = static_cast<PhototuxCanvasItem *>(item);
    // Pull latest composite export from Rust (set after each recomposite).
    phototux_canvas_apply_pending_export(canvas);
    m_zoom = canvas->zoom();
    m_panX = canvas->panX();
    m_panY = canvas->panY();
    m_docW = canvas->docWidth();
    m_docH = canvas->docHeight();
    m_hasDoc = canvas->hasDocument();
    m_phase = canvas->phase();
    m_itemW = float(item->width());
    m_itemH = float(item->height());

    if (canvas->m_wgpuDirty) {
        m_wgpuHandle = canvas->m_wgpuHandle;
        m_wgpuW = canvas->m_wgpuW;
        m_wgpuH = canvas->m_wgpuH;
        m_wgpuLayout = canvas->m_wgpuLayout;
        m_tryImport = m_wgpuHandle != 0;
        canvas->m_wgpuDirty = false;
        m_importAttempted = false;
    }

    // Always publish import / present notes so the shell HUD stays truthful.
    if (!m_importNote.isEmpty() && canvas->gpuStatus() != m_importNote)
        canvas->setGpuStatus(m_importNote);
}

bool PhototuxCanvasRenderer::tryImportWgpuTexture()
{
    if (!m_rhi || m_wgpuHandle == 0 || m_importAttempted)
        return m_importOk;
    m_importAttempted = true;

    // Attempt: wrap exported VkImage as QRhiTexture (requires same VkDevice — often fails).
    QRhiTexture *tex = m_rhi->newTexture(QRhiTexture::RGBA8,
                                         QSize(m_wgpuW > 0 ? m_wgpuW : 1, m_wgpuH > 0 ? m_wgpuH : 1),
                                         1,
                                         QRhiTexture::Flags());
    if (!tex) {
        m_importOk = false;
        m_importNote = QStringLiteral("wgpu import: newTexture failed");
        return false;
    }
    QRhiTexture::NativeTexture native;
    native.object = m_wgpuHandle;
    native.layout = m_wgpuLayout;
    const bool ok = tex->createFrom(native);
    if (ok) {
        m_importOk = true;
        m_importNote = QStringLiteral(
            "wgpu import: createFrom(VkImage) OK — zero-copy path available");
    } else {
        m_importOk = false;
        m_importNote = QStringLiteral(
            "wgpu import: createFrom(VkImage 0x%1) failed (separate VkDevice; DMA-BUF next). "
            "Present: GPU RHI document quad (no CPU full-frame upload).")
                           .arg(m_wgpuHandle, 0, 16);
    }
    std::fprintf(stderr, "[phototux-canvas] %s\n", qPrintable(m_importNote));
    // Keep success textures only once sampling is wired; always free after the attempt.
    delete tex;
    return m_importOk;
}

bool PhototuxCanvasRenderer::ensurePipeline()
{
    if (!m_rhi)
        return false;
    if (m_pipeline)
        return true;
    if (m_vertShader.isEmpty() || m_fragShader.isEmpty())
        return false;

    m_vbuf = m_rhi->newBuffer(QRhiBuffer::Dynamic, QRhiBuffer::VertexBuffer, sizeof(Vertex) * 4);
    if (!m_vbuf->create())
        return false;

    m_ubuf = m_rhi->newBuffer(QRhiBuffer::Dynamic, QRhiBuffer::UniformBuffer, sizeof(UBuf));
    if (!m_ubuf->create())
        return false;

    m_srb = m_rhi->newShaderResourceBindings();
    m_srb->setBindings({
        QRhiShaderResourceBinding::uniformBuffer(0, QRhiShaderResourceBinding::VertexStage
                                                        | QRhiShaderResourceBinding::FragmentStage,
                                                 m_ubuf),
    });
    if (!m_srb->create())
        return false;

    m_pipeline = m_rhi->newGraphicsPipeline();
    m_pipeline->setTopology(QRhiGraphicsPipeline::TriangleStrip);
    m_pipeline->setShaderStages({{QRhiShaderStage::Vertex, QShader::fromSerialized(m_vertShader)},
                                 {QRhiShaderStage::Fragment, QShader::fromSerialized(m_fragShader)}});
    QRhiVertexInputLayout inputLayout;
    inputLayout.setBindings({{sizeof(Vertex)}});
    inputLayout.setAttributes({
        {0, 0, QRhiVertexInputAttribute::Float2, offsetof(Vertex, x)},
        {0, 1, QRhiVertexInputAttribute::Float2, offsetof(Vertex, u)},
    });
    m_pipeline->setVertexInputLayout(inputLayout);
    m_pipeline->setShaderResourceBindings(m_srb);
    m_pipeline->setRenderPassDescriptor(renderTarget()->renderPassDescriptor());
    if (!m_pipeline->create()) {
        delete m_pipeline;
        m_pipeline = nullptr;
        return false;
    }
    return true;
}

void PhototuxCanvasRenderer::updateGeometry(QRhiResourceUpdateBatch *u)
{
    Vertex verts[4];
    if (m_hasDoc) {
        fillDocQuad(verts, m_itemW, m_itemH, m_panX, m_panY, m_zoom,
                    float(m_docW), float(m_docH));
    } else {
        // Degenerate empty when no document.
        std::memset(verts, 0, sizeof(verts));
    }
    u->updateDynamicBuffer(m_vbuf, 0, sizeof(verts), verts);

    UBuf ub{};
    // Document surface: cool slate (DESIGN surfaceOverlay-ish) with primary tint edge later.
    ub.color[0] = 0.18f;
    ub.color[1] = 0.20f;
    ub.color[2] = 0.28f;
    ub.color[3] = 1.f;
    ub.params[0] = m_hasDoc ? 1.f : 0.f;
    ub.params[1] = m_phase;
    ub.params[2] = 0.f;
    ub.params[3] = 0.f;
    u->updateDynamicBuffer(m_ubuf, 0, sizeof(ub), &ub);
}

void PhototuxCanvasRenderer::render(QRhiCommandBuffer *cb)
{
    if (!m_rhi || !cb)
        return;

    if (m_tryImport && !m_importAttempted)
        tryImportWgpuTexture();

    // Letterbox clear (canvasLetterbox #0C0C0E).
    const QColor letterbox = QColor::fromRgbF(0.047f, 0.047f, 0.055f, 1.0);

    if (!m_hasDoc || !ensurePipeline()) {
        cb->beginPass(renderTarget(), letterbox, {1.0f, 0}, nullptr);
        cb->endPass();
        return;
    }

    QRhiResourceUpdateBatch *u = m_rhi->nextResourceUpdateBatch();
    updateGeometry(u);

    cb->beginPass(renderTarget(), letterbox, {1.0f, 0}, u);
    cb->setGraphicsPipeline(m_pipeline);
    cb->setViewport({0, 0, float(renderTarget()->pixelSize().width()),
                     float(renderTarget()->pixelSize().height())});
    cb->setShaderResources();
    const QRhiCommandBuffer::VertexInput vbufBinding(m_vbuf, 0);
    cb->setVertexInput(0, 1, &vbufBinding);
    cb->draw(4);
    cb->endPass();
}
