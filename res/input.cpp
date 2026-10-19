#include <cstdint>
#include <cstdio>
#include <cstdlib>

__attribute__((noinline)) [[noreturn]]
void panic()
{
    puts("panicked!");
    exit(EXIT_FAILURE);
}

__attribute__((noinline)) void arbitrary_side_effect()
{
}

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

void meow(uint32_t *foo, uint32_t *bar)
{
    // First iteration
    if (*foo == 1)
    {
        *foo = 2;
    }
    else
    {
        panic();
    }

    if (*bar == 3)
    {
        *bar = 4;
    }
    else
    {
        panic();
    }

    *foo = 1;
    *bar = 3;

    // arbitrary_side_effect();

    // Second iteration
    if (*foo == 1)
    {
        *foo = 2;
    }
    else
    {
        panic();
    }

    if (*bar == 3)
    {
        *bar = 4;
    }
    else
    {
        panic();
    }

    *foo = 1;
    *bar = 3;
}

int main()
{
    uint32_t a = 1;
    uint32_t b = 3;
    meow(&a, &b);
}
