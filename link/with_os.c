#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdnoreturn.h>
#include "vdisk.c"

extern void urcl_main();

bool skip_nx = false;
uint32_t c_addr = 0;
uint32_t c_page = 0;
char pan_msg[1000];

noreturn void panic() {
    printf("\x1b[0;1;31mURCL Runtime Pancic: %s\x1b[0m\n", pan_msg);
    exit(1);
}

int main(int argc, char* argv[]) {
    if (argc == 2) {
        printf("Read %s into virtual disk.\n", argv[1]);
        FILE* fs = fopen(argv[1], "r");
        fread(vdisk, VD_PSIZE, VD_PAGES, fs);
        fclose(fs);
    }
    urcl_main();
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
            char ret = getchar();
            if (ret == EOF) exit(0);
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
        default:
            sprintf(pan_msg, "Port %u is not supported.", port);
            panic();
    }
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
        default:
            sprintf(pan_msg, "Port %u is not supported.", port);
            panic();
    }
}
