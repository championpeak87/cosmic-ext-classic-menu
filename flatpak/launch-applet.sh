#!/bin/sh

# Workaround to expand $HOME variable defined in flatpak manifest file
XDG_DATA_DIRS=$(eval echo "$XDG_DATA_DIRS")

exec cosmic-ext-classic-menu-applet "$@"