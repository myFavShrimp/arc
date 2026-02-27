#include <stdio.h>
#include <string.h>
#include "greet.h"
#include "bubble.h"

int main(int argc, char *argv[]) {
    const char *name = "World";
    int shrimpsay = 0;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--name") == 0 && i + 1 < argc) {
            name = argv[++i];
        } else if (strcmp(argv[i], "--shrimpsay") == 0) {
            shrimpsay = 1;
        }
    }

    char greeting[256];
    greet(name, greeting, sizeof(greeting));

    if (shrimpsay) {
        print_shrimpsay(greeting);
    } else {
        printf("%s\n", greeting);
    }

    return 0;
}
