# Linux desktop integration

Install release artifacts under the standard XDG prefixes:

```text
/usr/bin/phototux
/usr/share/applications/io.github.PerkyZZ999.PhotoTux.desktop
/usr/share/icons/hicolor/scalable/apps/io.github.PerkyZZ999.PhotoTux.svg
/usr/share/metainfo/io.github.PerkyZZ999.PhotoTux.metainfo.xml
```

The desktop entry associates standard `image/png` and `image/jpeg` MIME types.
PhotoTux opens the first desktop-provided file because v1 is single-document.
File → Open and Export use Qt Quick Dialogs, which route through the desktop
portal on supported Wayland environments.
