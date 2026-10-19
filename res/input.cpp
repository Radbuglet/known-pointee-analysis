#include <cstdint>
#include <cstdio>

[[noreturn]] void panic();

void arbitrary_side_effect();

extern "C"
{
    [[noinline]]
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
