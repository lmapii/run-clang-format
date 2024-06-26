/*
 * Copyright (c) 2024 Martin Lampacher. All rights reserved.
 */

#include "module_b.h"

/***********************************************************************************************************************
 * Definitions
 **********************************************************************************************************************/

// clang-tidy won't complain about uL naming in unused defines
// #define _MAXLOOP 1234uL

/***********************************************************************************************************************
 * Data
 **********************************************************************************************************************/

static volatile uint8_t _some_variable[] = {1, 2, 3};

/***********************************************************************************************************************
 * Functions
 **********************************************************************************************************************/

int main (int argc, const char *argv[]) // NOLINT : unused argument argv
{
    // uint8_t i = 0;
    // module_a_init ();

    module_b_init ();
    module_c_init ();

    _some_variable[0] = 123; // NOLINT: magic number
    _some_variable[0] = 2;

    // for (i = 0; i < _MAXLOOP; i++)
    // {
    //     _some_variable[0] += 1;
    // }
}
