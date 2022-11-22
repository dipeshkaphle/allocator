#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

extern char *alloc(unsigned long long);
extern void dealloc(char *);

int main() {
  char *m = alloc(8 * 1024 * 1024);
  if (m == NULL) {
    perror("NULL memory, alloc failed");
  }

  int y = 2;
  int *x = &y;

  fputs("INFO: About to dealloc alloc'd memory\n", stderr);
  fflush(stderr);
  dealloc(m);
  fputs("INFO: Deallocated the memory allocated using 'alloc'\n", stderr);
  fflush(stderr);
  fputs("INFO: About to dealloc invalid stack memory\n", stderr);
  dealloc((char *)x);
  fputs("WARNING: Shouldn't be here\n", stderr);
}
