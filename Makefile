APP_NAME := postchi

PREFIX := /usr/local
BINDIR := $(PREFIX)/bin
APPDIR := $(PREFIX)/share/applications
ICONDIR := $(PREFIX)/share/icons/hicolor/256x256/apps

.PHONY: release install uninstall

release:
	cargo build --release

install:
	install -Dm755 target/release/$(APP_NAME) \
		$(DESTDIR)$(BINDIR)/$(APP_NAME)

	install -Dm644 assets/$(APP_NAME).desktop \
		$(DESTDIR)$(APPDIR)/$(APP_NAME).desktop

	install -Dm644 assets/icon.png \
		$(DESTDIR)$(ICONDIR)/$(APP_NAME).png

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/$(APP_NAME)
	rm -f $(DESTDIR)$(APPDIR)/$(APP_NAME).desktop
	rm -f $(DESTDIR)$(ICONDIR)/$(APP_NAME).png
