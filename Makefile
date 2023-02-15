FNAME = target/debug/urcln
ifeq ($(OS), Windows_NT)
	FNAME = target/debug/urcln.exe
endif

RMNAME = urcln
ifeq ($(OS), Windows_NT)
	RMNAME = urcln.exe
endif

NPROC = $(shell nproc)

cli:
	cargo build -j$(NPROC)
	-rm $(RMNAME)
	mv $(FNAME) . -f
