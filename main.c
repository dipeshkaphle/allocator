#include <stdio.h>
#include <string.h>
#include <unistd.h>

extern char *alloc(unsigned long long);
extern void dealloc(char *);

int main() {
  char *m = alloc(16);
  strcpy(m, "Hellooooo World");

  write(1, m, 15);

  // If I uncomment this, fsanitize=address wont show memory leak. No clue why
  /* printf("%s\n", m); */

#ifndef LEAK
  dealloc(m);
#endif
}
