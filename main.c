#include <stdio.h>
#include <string.h>
#include <unistd.h>

extern char *alloc(unsigned long long);
extern void dealloc(char *);

int main() {
  char *m = alloc(8 * 1024 * 1024);
  FILE *fd = fopen("output", "r");
  fseek(fd, 0, SEEK_END);
  int len = ftell(fd);
  fseek(fd, 0, SEEK_SET);
  // Not doing error handling
  fread(m, 1, len, fd);
  m[len] = '\0';
  fclose(fd);
  fprintf(stderr, "%s", m);

  // If I uncomment this, fsanitize=address wont show memory leak. No clue why
  /* printf("%s\n", m); */

#ifndef LEAK
  dealloc(m);
#endif
}
