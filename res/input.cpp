#include <cstdint>

void panic();

bool *unrelated(bool *input);

void meow(bool *foo, bool *bar)
{
    for (uint32_t i = 0; i < 1024; i += 1)
    {
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
}
