#include <cstdint>
#include <cstdio>
#include <cstdlib>

[[noreturn]] void panic()
{
    puts("panicked!");
    exit(EXIT_FAILURE);
}

void arbitrary_side_effect()
{
}

extern "C"
{
    void meow_assert(bool cond, unsigned int location)
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
