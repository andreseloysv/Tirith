#!/bin/sh
# Written by NeoBot: inside its network sandbox, which macOS will not let a tool nest another in.
PATH=$(printf %s "$PATH" | tr : '\n' | grep -vxF '{dir}' | paste -sd: -)
