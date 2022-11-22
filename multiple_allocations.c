#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

extern char *alloc(unsigned long long);
extern void dealloc(char *);

int main() {
  char *m;
  int non_nulls = 0;
  int nulls = 0;
  srand(42);
  for (int i = 0; i < 200000; i++) {
    m = alloc((rand() % 200) + 1);
    if (m != NULL) {
      non_nulls++;
    } else {
      nulls++;
    }
    if (i % 2 == 0) {
      dealloc(m);
    }
  }
  char *last = NULL;
  for (int i = 0; i < 1300000; i++) {
    m = alloc((rand() % 200) + 1);
    if (m != NULL) {
      non_nulls++;
    } else {
      nulls++;
    }
    if (i % 2 == 0) {
      dealloc(m);
    }
  }
  printf("Null allocs: %d\n", nulls);
  printf("Non-null allocs: %d\n", non_nulls);
}
