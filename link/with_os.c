#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdnoreturn.h>

#include "vdisk.h"

extern void urcl_main();

bool skip_nx = false;
uint16_t c_addr = 0;
uint16_t c_page = 0;
char pan_msg[1000];

noreturn void panic() {
    printf("\x1b[0;1;31mURCL Panic: \x1b[0m%s\n", pan_msg);
    exit(1);
}

int main(int argc, char* argv[]) {
    if (argc == 2) {
        FILE* fs = fopen(argv[1], "r");
        int rc = fread(vdisk, 1, VD_PAGES, fs);
        fclose(fs);
        printf("Read %d bytes into virtual disk.\n", rc);
    }
    urcl_main();
}

uint16_t urcl_pin(uint16_t port) {
    switch ((uint8_t) port) {
        case 1:     // TEXT, etc.
        case 16:
        case 17:
        case 18:
        case 19:
        case 20: {
            char ret = getchar();
            if (ret == 0xFF) exit(0);
            return ret;
        }
        case 24: {  // INT
            int32_t a;
            scanf("%d", &a);
            return a;
        }
        case 2:
        case 25: {  // NUMB, UINT
            uint32_t a;
            scanf("%u", &a);
            return a;
        }
        case 27: {  // HEX
            uint32_t a;
            scanf("%x", &a);
            return a;
        }
        case 32: {  // ADDR
            return c_addr;
        }
        case 34: {  // PAGE
            return c_page;
        }
        case 33: {  // BUS
            return vd_read(c_addr, c_page);
        }
        default: {
            sprintf(pan_msg, "Port %u is not supported.", port);
            panic();
        }
    }
}

void urcl_pout(uint16_t port, uint16_t data) {
    switch ((uint8_t) port) {
        case 1:     // TEXT, etc.
        case 16:
        case 17:
        case 18:
        case 19:
        case 20: {
            putchar((unsigned char) data);
            break;
        }
        case 24: {  // INT
            printf("%d", data);
            break;
        }
        case 2:     // NUMB, UINT
        case 25: {
            printf("%u", data);
            break;
        }
        case 27: {  // HEX
            printf("%04X", data);
            break;
        }
        case 32: {  // ADDR
            c_addr = data;
            break;
        }
        case 34: {  // PAGE
            c_page = data;
            break;
        }
        case 33: {  // BUS
            vd_write(c_addr, c_page, data);
            break;
        }
        default: {
            sprintf(pan_msg, "Port %u is not supported.", port);
            panic();
        }
    }
}
