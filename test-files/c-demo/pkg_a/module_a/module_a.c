/*
 * Copyright (c) 2024 Martin Lampacher. All rights reserved.
 */

#include "module_a.h"
#include <stdbool.h>

/***********************************************************************************************************************
 * Data
 **********************************************************************************************************************/

static volatile bool _changeme = false;

/***********************************************************************************************************************
 * Functions
 **********************************************************************************************************************/

void module_a_init (void)
{
    uint32_t some_value = 1;
    // nothing to see here
    _changeme = !_changeme;

    CALC (_changeme, some_value);

    if (some_value < MODULE_A_SMTH)
    {
        some_value++;
    }
}
