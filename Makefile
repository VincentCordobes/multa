PREFIX=/usr/local
BINDIR=$(PREFIX)/bin

multa:
	cargo build --release

install: multa 
	install -D target/release/multa $(DESTDIR)$(BINDIR)/multa

uninstall:
	$(RM) $(DESTDIR)$(BINDIR)/multa

clean:
	cargo clean

.PHONY: clean install uninstall
