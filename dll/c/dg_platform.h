/* Force-included into every doomgeneric source file. */
#pragma once
#include <stdlib.h>
#include <process.h>

/* Doom calls exit() when it quits or hits an error, which would take Armored Core down with it. The DLL's dg_exit
   ends only Doom's thread. (stdlib.h is already included above, so its own declaration of exit is unaffected.) */
void dg_exit(int code);
#define exit dg_exit
