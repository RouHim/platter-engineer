# Set german as default keyboard locale
locale-gen
localectl set-keymap de

# Start NetworkManger
systemctl start NetworkManager.service

# Start graphical interface
if [[ ! $DISPLAY && $XDG_VTNR -eq 1 ]]; then
    # set custom background
    cp ~/.wallpaper/bg.png /usr/share/backgrounds/xfce/xfce-shapes.svg

    # Set Theme to Adwaita-dark
    xfconf-query -c xsettings -p /Net/ThemeName -s "Adwaita-dark"

    # Start GUI
    startx
fi