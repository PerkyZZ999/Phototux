# Linux desktop integration

Install release artifacts under the standard XDG prefixes:

```text
/usr/bin/phototux
/usr/share/applications/io.github.PerkyZZ999.PhotoTux.desktop
/usr/share/icons/hicolor/scalable/apps/io.github.PerkyZZ999.PhotoTux.svg
/usr/share/icons/hicolor/256x256/apps/io.github.PerkyZZ999.PhotoTux.png
/usr/share/metainfo/io.github.PerkyZZ999.PhotoTux.metainfo.xml
```

The desktop entry associates standard `image/png` and `image/jpeg` MIME types.
PhotoTux opens desktop-provided files into the tabbed document session ([DR-024](../../internal_docs/Appendix/Decision-Register.md#dr-024--document-session-model)).
File → Open and Export use Qt Quick Dialogs, which route through the desktop
portal on supported Wayland environments.
