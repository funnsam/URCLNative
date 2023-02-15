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

build_link:
	gcc -c link/with_os.c -o link/with_os.o -O2

all: cli build_link
