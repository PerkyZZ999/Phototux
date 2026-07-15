#pragma once

#include <QQuickRhiItem>
#include <QQuickRhiItemRenderer>

// Hybrid canvas item (ADR-003 / ADR-010 spike):
// Renders a GPU-side animated clear via Qt RHI (Vulkan on this host).
// Does NOT use CPU QImage upload for the present path.

class SpikeCanvasItem : public QQuickRhiItem
{
    Q_OBJECT
    Q_PROPERTY(float phase READ phase WRITE setPhase NOTIFY phaseChanged)
    Q_PROPERTY(QString statusText READ statusText WRITE setStatusText NOTIFY statusTextChanged)

public:
    explicit SpikeCanvasItem(QQuickItem *parent = nullptr);

    float phase() const { return m_phase; }
    void setPhase(float p);

    QString statusText() const { return m_status; }
    void setStatusText(const QString &s);

signals:
    void phaseChanged();
    void statusTextChanged();

protected:
    QQuickRhiItemRenderer *createRenderer() override;

private:
    float m_phase = 0.f;
    QString m_status = QStringLiteral("SpikeCanvas: init");
    friend class SpikeCanvasRenderer;
};

class SpikeCanvasRenderer : public QQuickRhiItemRenderer
{
public:
    SpikeCanvasRenderer() = default;

protected:
    void initialize(QRhiCommandBuffer *cb) override;
    void synchronize(QQuickRhiItem *item) override;
    void render(QRhiCommandBuffer *cb) override;

private:
    QRhi *m_rhi = nullptr;
    float m_phase = 0.f;
};
