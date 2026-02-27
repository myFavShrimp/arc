#include <stdio.h>
#include <string.h>
#include "bubble.h"

void print_shrimpsay(const char *message) {
    size_t len = strlen(message);

    printf(" +");
    for (size_t i = 0; i < len + 2; i++) printf("-");
    printf("+\n");

    printf(" | %s |\n", message);

    printf(" +");
    for (size_t i = 0; i < len + 2; i++) printf("-");
    printf("+\n");

    printf("    \\\n");
    printf("     \\\n");
    printf("      (°>)\n");
    printf("      /|\n");
    printf("      \\|\n");
    printf("      <>\n");
}
