#pragma once

#include <QQuickRhiItem>
#include <QQuickRhiItemRenderer>
#include <QByteArray>
#include <QString>

// Forward decls — full RHI types only in the .cpp (private headers).
class QRhi;
class QRhiBuffer;
class QRhiShaderResourceBindings;
class QRhiGraphicsPipeline;
class QRhiResourceUpdateBatch;
class QRhiCommandBuffer;

// Production GPU viewport (ADR-003 / ADR-005 / ADR-010).
// QQuickRhiItem present path is GPU-only (no QImage full-frame upload).

class PhototuxCanvasItem : public QQuickRhiItem
{
    Q_OBJECT
    Q_PROPERTY(float zoom READ zoom WRITE setZoom NOTIFY zoomChanged)
    Q_PROPERTY(float panX READ panX WRITE setPanX NOTIFY panXChanged)
    Q_PROPERTY(float panY READ panY WRITE setPanY NOTIFY panYChanged)
    Q_PROPERTY(int docWidth READ docWidth WRITE setDocWidth NOTIFY docWidthChanged)
    Q_PROPERTY(int docHeight READ docHeight WRITE setDocHeight NOTIFY docHeightChanged)
    Q_PROPERTY(bool hasDocument READ hasDocument WRITE setHasDocument NOTIFY hasDocumentChanged)
    Q_PROPERTY(float phase READ phase WRITE setPhase NOTIFY phaseChanged)
    Q_PROPERTY(QString gpuStatus READ gpuStatus WRITE setGpuStatus NOTIFY gpuStatusChanged)
    Q_PROPERTY(float fps READ fps WRITE setFps NOTIFY fpsChanged)

public:
    explicit PhototuxCanvasItem(QQuickItem *parent = nullptr);

    float zoom() const { return m_zoom; }
    void setZoom(float z);

    float panX() const { return m_panX; }
    void setPanX(float v);

    float panY() const { return m_panY; }
    void setPanY(float v);

    int docWidth() const { return m_docW; }
    void setDocWidth(int w);

    int docHeight() const { return m_docH; }
    void setDocHeight(int h);

    bool hasDocument() const { return m_hasDoc; }
    void setHasDocument(bool v);

    float phase() const { return m_phase; }
    void setPhase(float p);

    QString gpuStatus() const { return m_gpuStatus; }
    void setGpuStatus(const QString &s);

    float fps() const { return m_fps; }
    void setFps(float f);

    // Optional wgpu VkImage import attempt (quint64 raw handle).
    void setWgpuImageHandle(quint64 handle, int width, int height, int layout);

signals:
    void zoomChanged();
    void panXChanged();
    void panYChanged();
    void docWidthChanged();
    void docHeightChanged();
    void hasDocumentChanged();
    void phaseChanged();
    void gpuStatusChanged();
    void fpsChanged();

protected:
    QQuickRhiItemRenderer *createRenderer() override;

private:
    float m_zoom = 1.f;
    float m_panX = 0.f;
    float m_panY = 0.f;
    int m_docW = 1920;
    int m_docH = 1080;
    bool m_hasDoc = false;
    float m_phase = 0.f;
    float m_fps = 0.f;
    QString m_gpuStatus = QStringLiteral("GPU viewport idle");
    quint64 m_wgpuHandle = 0;
    int m_wgpuW = 0;
    int m_wgpuH = 0;
    int m_wgpuLayout = 0;
    bool m_wgpuDirty = false;

    friend class PhototuxCanvasRenderer;
};

// Applied from pending wgpu export slot (see register_types.cpp).
void phototux_canvas_apply_pending_export(PhototuxCanvasItem *item);

class PhototuxCanvasRenderer : public QQuickRhiItemRenderer
{
public:
    PhototuxCanvasRenderer() = default;
    ~PhototuxCanvasRenderer() override;

protected:
    void initialize(QRhiCommandBuffer *cb) override;
    void synchronize(QQuickRhiItem *item) override;
    void render(QRhiCommandBuffer *cb) override;

private:
    bool ensurePipeline();
    void releaseResources();
    void updateGeometry(QRhiResourceUpdateBatch *u);
    bool tryImportWgpuTexture();

    QRhi *m_rhi = nullptr;
    QRhiBuffer *m_vbuf = nullptr;
    QRhiBuffer *m_ubuf = nullptr;
    QRhiShaderResourceBindings *m_srb = nullptr;
    QRhiGraphicsPipeline *m_pipeline = nullptr;

    float m_zoom = 1.f;
    float m_panX = 0.f;
    float m_panY = 0.f;
    int m_docW = 1920;
    int m_docH = 1080;
    bool m_hasDoc = false;
    float m_phase = 0.f;
    float m_itemW = 1.f;
    float m_itemH = 1.f;

    quint64 m_wgpuHandle = 0;
    int m_wgpuW = 0;
    int m_wgpuH = 0;
    int m_wgpuLayout = 0;
    bool m_tryImport = false;
    bool m_importAttempted = false;
    bool m_importOk = false;
    QString m_importNote;

    QByteArray m_vertShader;
    QByteArray m_fragShader;
};
