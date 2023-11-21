# Set german as default keyboard locale
locale-gen
localectl set-keymap de

# set custom background
cp ~/.wallpaper/bg.svg /usr/share/backgrounds/xfce/xfce-shapes.svg

# Start graphical interface
if [[ ! $DISPLAY && $XDG_VTNR -eq 1 ]]; then
  startx
fi