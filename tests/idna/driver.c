/* The idna suite's driver (tests/idna.scala): host names on stdin, one a line as the hex of their bytes (an empty
   line is the empty name), each converted by idn2_to_ascii_8z with the flags 0 that sttp's binding passes; on stdout
   "<code>\t<hex of the output>" a line, the output only where the code is 0. Status 2 on a line it cannot read. */
#include <stdio.h>
#include <string.h>

#include "idn2.h"

static int nibble(int c) {
  if (c >= '0' && c <= '9') return c - '0';
  if (c >= 'a' && c <= 'f') return c - 'a' + 10;
  return -1;
}

int main(void) {
  static char line[1 << 20], name[1 << 19];
  static const char digits[] = "0123456789abcdef";
  unsigned long row = 0;

  while (fgets(line, sizeof line, stdin)) {
    size_t n = strcspn(line, "\n"), i;
    char *out = NULL;
    int rc;
    row++;
    if (line[n] != '\n' && !feof(stdin)) {
      fprintf(stderr, "driver: row %lu is longer than %zu bytes\n", row, sizeof line - 1);
      return 2;
    }
    if (n % 2) {
      fprintf(stderr, "driver: row %lu has an odd number of hex digits\n", row);
      return 2;
    }
    for (i = 0; i < n; i += 2) {
      int hi = nibble((unsigned char)line[i]), lo = nibble((unsigned char)line[i + 1]);
      if (hi < 0 || lo < 0 || (hi == 0 && lo == 0)) {
        fprintf(stderr, "driver: row %lu is not the hex of a name\n", row);
        return 2;
      }
      name[i / 2] = (char)(hi << 4 | lo);
    }
    name[n / 2] = 0;
    rc = idn2_to_ascii_8z(name, &out, 0);
    printf("%d\t", rc);
    if (rc == IDN2_OK) {
      const unsigned char *p;
      for (p = (const unsigned char *)out; *p; p++) {
        putchar(digits[*p >> 4]);
        putchar(digits[*p & 15]);
      }
      idn2_free(out);
    }
    putchar('\n');
  }
  return ferror(stdin) || fflush(stdout) ? 2 : 0;
}
