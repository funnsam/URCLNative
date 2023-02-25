#include <stdlib.h>
#include <stdint.h>

#define VD_PAGES 0xffff
#define VD_PSIZE 256
#define VD_TSIZE VD_PAGES * VD_PSIZE

uint32_t vdisk[VD_TSIZE];
uint32_t vd_read(uint32_t addr, uint32_t page);
uint32_t vd_write(uint32_t addr, uint32_t page, uint32_t data);

#include "vdisk.c"
