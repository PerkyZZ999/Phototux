#include "spike_canvas_item.h"

#include <QtQml/qqml.h>

extern "C" void phototux_spike_register_types()
{
    qmlRegisterType<SpikeCanvasItem>("PhototuxSpike", 1, 0, "SpikeCanvas");
}
