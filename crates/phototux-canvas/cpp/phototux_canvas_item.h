#pragma once

#include <QQuickRhiItem>
#include <QQuickRhiItemRenderer>
#include <QByteArray>
#include <QString>
#include <memory>

// Forward decls — full RHI types only in the .cpp (private headers).
class QRhi;
class QRhiBuffer;
class QRhiShaderResourceBindings;
class QRhiGraphicsPipeline;
class QRhiResourceUpdateBatch;
class QRhiCommandBuffer;
class QRhiSampler;
class QRhiTexture;
class QQuickWindow;
class QVulkanInstance;

// Production GPU viewport (DR-023).
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
    // Bumped by the host whenever a new composite is published. Writing it is
    // what schedules a repaint, so the canvas no longer needs a clock ticking
    // every vsync to notice that its content changed.
    Q_PROPERTY(int contentTick READ contentTick WRITE setContentTick NOTIFY contentTickChanged)
    Q_PROPERTY(bool selectionAnts READ selectionAnts WRITE setSelectionAnts NOTIFY selectionAntsChanged)
    Q_PROPERTY(QString gpuStatus READ gpuStatus WRITE setGpuStatus NOTIFY gpuStatusChanged)
    Q_PROPERTY(float fps READ fps WRITE setFps NOTIFY fpsChanged)

public:
    explicit PhototuxCanvasItem(QQuickItem *parent = nullptr);
    ~PhototuxCanvasItem() override;

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
    int contentTick() const { return m_contentTick; }
    void setContentTick(int t);

    bool selectionAnts() const { return m_selectionAnts; }
    void setSelectionAnts(bool v);

    QString gpuStatus() const { return m_gpuStatus; }
    void setGpuStatus(const QString &s);

    float fps() const { return m_fps; }
    void setFps(float f);

    // Optional wgpu VkImage import attempt (quint64 raw handle).
    void setWgpuImageHandle(quint64 handle, int width, int height, int layout);
    void setSelectionImageHandle(quint64 handle, int width, int height, int layout);

signals:
    void zoomChanged();
    void panXChanged();
    void panYChanged();
    void docWidthChanged();
    void docHeightChanged();
    void hasDocumentChanged();
    void phaseChanged();
    void contentTickChanged();
    void selectionAntsChanged();
    void gpuStatusChanged();
    void fpsChanged();

protected:
    QQuickRhiItemRenderer *createRenderer() override;
    void itemChange(ItemChange change, const ItemChangeData &data) override;

private:
    void configureSharedVulkanDevice(QQuickWindow *window);

    float m_zoom = 1.f;
    float m_panX = 0.f;
    float m_panY = 0.f;
    int m_docW = 1920;
    int m_docH = 1080;
    bool m_hasDoc = false;
    float m_phase = 0.f;
    int m_contentTick = 0;
    bool m_selectionAnts = false;
    float m_fps = 0.f;
    QString m_gpuStatus = QStringLiteral("GPU viewport idle");
    quint64 m_wgpuHandle = 0;
    int m_wgpuW = 0;
    int m_wgpuH = 0;
    int m_wgpuLayout = 0;
    bool m_wgpuDirty = false;
    quint64 m_selHandle = 0;
    int m_selW = 0;
    int m_selH = 0;
    int m_selLayout = 0;
    bool m_selDirty = false;
    bool m_deviceConfigured = false;
    std::unique_ptr<QVulkanInstance> m_vulkanInstance;

    friend class PhototuxCanvasRenderer;
};

// Applied from pending wgpu export slot (see register_types.cpp).
void phototux_canvas_apply_pending_export(PhototuxCanvasItem *item);
bool phototux_canvas_get_wgpu_device(quint64 *instance, quint64 *physicalDevice,
                                    quint64 *device, quint32 *queueFamilyIndex,
                                    quint32 *queueIndex);
extern "C" void phototux_canvas_lock_shared_queue();
extern "C" void phototux_canvas_unlock_shared_queue();

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
    bool tryImportSelectionTexture();
    bool ensureDummySelectionTexture();

    QRhi *m_rhi = nullptr;
    QRhiBuffer *m_vbuf = nullptr;
    QRhiBuffer *m_ubuf = nullptr;
    QRhiShaderResourceBindings *m_srb = nullptr;
    QRhiGraphicsPipeline *m_pipeline = nullptr;
    QRhiSampler *m_sampler = nullptr;
    QRhiSampler *m_selSampler = nullptr;
    QRhiTexture *m_importedTexture = nullptr;
    QRhiTexture *m_selectionTexture = nullptr;
    bool m_selectionIsDummy = true;

    float m_zoom = 1.f;
    float m_panX = 0.f;
    float m_panY = 0.f;
    int m_docW = 1920;
    int m_docH = 1080;
    bool m_hasDoc = false;
    float m_phase = 0.f;
    bool m_selectionAnts = false;
    float m_itemW = 1.f;
    float m_itemH = 1.f;

    quint64 m_wgpuHandle = 0;
    int m_wgpuW = 0;
    int m_wgpuH = 0;
    int m_wgpuLayout = 0;
    bool m_tryImport = false;
    bool m_importAttempted = false;
    bool m_importOk = false;
    quint64 m_selHandle = 0;
    int m_selW = 0;
    int m_selH = 0;
    int m_selLayout = 0;
    bool m_trySelImport = false;
    bool m_selImportAttempted = false;
    bool m_firstRenderReported = false;
    QString m_importNote;

    QByteArray m_vertShader;
    QByteArray m_fragShader;
};
