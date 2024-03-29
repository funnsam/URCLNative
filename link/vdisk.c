uint32_t vdisk[VD_TSIZE];

uint32_t vd_read(uint32_t addr, uint32_t page) {
    return vdisk[addr + page * VD_PSIZE];
}

uint32_t vd_write(uint32_t addr, uint32_t page, uint32_t data) {
    vdisk[addr + page * VD_PSIZE] = data;
}
