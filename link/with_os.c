#include <stdio.h>
#include <stdint.h>
#include "vdisk.c"

extern void urcl_main();

int main() {
    urcl_main();
}

uint32_t c_addr = 0;
uint32_t c_page = 0;
char cbuf[1000];

void shift_buf() {
    for (int i = 1; i < 1000; i++) {
        cbuf[i-1] = cbuf[i];
    }
}


uint32_t urcl_pin(uint32_t port) {
    int32_t a;
    switch ((uint8_t) port) {
        case 1:     // TEXT, etc.
        case 16:
        case 17:
        case 18:
        case 19:
        case 20:
            if (cbuf[0] == 0) {
                scanf("%1000s\n", cbuf);
            }
            char ret = cbuf[0];
            shift_buf();
            return ret;
        case 2:     // NUMB, INT
        case 24:
            scanf("%d", &a);
            return a;
        case 25:    // UINT
            scanf("%u", &a);
            return a;
        case 27:    // HEX
            scanf("%x", &a);
            return a;

        case 32:    // ADDR
            return c_addr;
        case 34:    // PAGE
            return c_page;
        case 33:    // BUS
            return vd_read(c_addr, c_page);
    }

    return 0;
}

void urcl_pout(uint32_t port, uint32_t data) {
    switch ((uint8_t) port) {
        case 1:     // TEXT, etc.
        case 16:
        case 17:
        case 18:
        case 19:
        case 20:
            putchar((unsigned char) data);
            break;
        case 2:     // NUMB, INT
        case 24:
            printf("%d", data);
            break;
        case 25:    // UINT
            printf("%u", data);
            break;
        case 27:    // HEX
            printf("%X", data);
            break;

        case 32:    // ADDR
            c_addr = data;
            break;
        case 34:    // PAGE
            c_page = data;
            break;
        case 33:    // BUS
            vd_write(c_addr, c_page, data);
            break;
    }
}
