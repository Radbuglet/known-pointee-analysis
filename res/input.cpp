#include <cstdint>

void panic();

bool *unrelated(bool *input);

void meow(bool *foo, bool *bar)
{
    // Iteration 1
    if (!*foo)
    {
        *foo = true;
    }
    else
    {
        panic();
    }

    if (!*bar)
    {
        *bar = true;
    }
    else
    {
        panic();
    }

    foo = unrelated(foo);

    *foo = false;
    *bar = false;

    // Iteration 2
    if (!*foo)
    {
        *foo = true;
    }
    else
    {
        panic();
    }

    if (!*bar)
    {
        *bar = true;
    }
    else
    {
        panic();
    }

    foo = unrelated(foo);

    *foo = false;
    *bar = false;
}
