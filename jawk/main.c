#include <stdio.h>
#include <stdlib.h>

int main() {
	char* buffer = (char*) malloc(1000);
	double test1 = -800020000.0;
	printf("%.13000g\n", test1);
	double test2 = 800020000.0;
	printf("%.13000g", test2);
}
