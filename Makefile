target/release/ykctl:
	cargo build --release

ykctl: target/release/ykctl
	cp $< $@
	strip $@
	upx -9 $@

install: ykctl
	install -t $(HOME)/.local/bin -m 0755 $^

clean:
	rm ykctl
	cargo clean

.PHONY: install clean

