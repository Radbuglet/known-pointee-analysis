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

__attribute__((noinline)) void demo_1(uint64_t *foo, uint8_t *bar)
{
    *foo = 1;
    *bar = 1;

    if (*foo != 1)
    {
        puts("I'm allegedly unreachable!");
    }
}

__attribute__((noinline)) void demo_2(uint64_t *foo, uint8_t *bar)
{
    *foo = 1;
    *bar = 1;

    printf("*foo = 0x%llx\n", *foo);
}

int main()
{
    puts("demo 1");
    uint64_t a = 1;
    // https://en.cppreference.com/c/language/object#Strict_aliasing
    demo_1(&a, reinterpret_cast<uint8_t *>(&a) + 1);

    puts("demo 2");
    uint64_t b = 1;
    // https://en.cppreference.com/c/language/object#Strict_aliasing
    demo_2(&b, reinterpret_cast<uint8_t *>(&b) + 1);
}
