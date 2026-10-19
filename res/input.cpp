#include <cstdint>
#include <cstdio>
#include <cstdlib>

extern "C"
{
    __attribute__((noinline)) void meow_assert(bool cond, unsigned int location)
    {
        if (!cond)
        {
            printf("bad analysis at location %u\n", location);
        }
    }
}

void meow(uint64_t *foo, uint8_t *bar)
{
    *foo = 1;
    *bar = 1;

    if (*foo != 1)
    {
        puts("I'm allegedly unreachable!\n");
    }
}

int main()
{
    uint64_t a = 1;
    // https://en.cppreference.com/c/language/object#Strict_aliasing
    meow(&a, reinterpret_cast<uint8_t *>(&a) + 1);
}
