#include <QtQml/qqmlextensionplugin.h>

Q_IMPORT_QML_PLUGIN(PhotoTuxQmlPlugin)

extern "C" void phototux_qml_force_link() noexcept
{
}
