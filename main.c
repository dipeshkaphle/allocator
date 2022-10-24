#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

extern char *alloc(unsigned long long);
extern void dealloc(char *);

int main() {
  char *m = alloc(8 * 1024 * 1024);

  /* char *m = alloc(300 * 1024 * 1024); */
  /* assert(m == NULL); */
  /* return 0; */

  FILE *fd = fopen("output", "r");
  fseek(fd, 0, SEEK_END);
  int len = ftell(fd);
  fseek(fd, 0, SEEK_SET);

  // Not doing error handling
  fread(m, 1, len, fd);
  m[len] = '\0';
  fclose(fd);
  if (strlen(m) == len) {
    const char *s = "success\n";
    write(1, s, strlen(s));
  }

#ifndef LEAK
  dealloc(m);
#endif
}
