#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

void c_init_sdk(void);

void c_get_events(const int8_t *context, void (*callback)(const int8_t*, const int8_t*));
