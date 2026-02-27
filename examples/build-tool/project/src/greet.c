#include <stdio.h>
#include "greet.h"

void greet(const char *name, char *buf, size_t buf_size) {
    snprintf(buf, buf_size, "Hello, %s!", name);
}
