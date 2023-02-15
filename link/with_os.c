#include <stdio.h>
#include <stdint.h>

extern void urcl_main();

void main() {
    urcl_main();
}

void urcl_pout(uint32_t port, uint32_t data) {
    switch ((uint8_t) port) {
        case 1:
            putchar((unsigned char) data);
            break;
        case 2:
            printf("%d", data);
            break;
    }
}
