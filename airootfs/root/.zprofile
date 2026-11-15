# Start graphical interface
if [[ ! $DISPLAY && $XDG_VTNR -eq 1 ]]; then
    # Start NetworkManger
    systemctl start NetworkManager.service

    # set custom background
    cp ~/.wallpaper/bg.png /usr/share/backgrounds/xfce/xfce-shapes.svg

    # Set Theme to Adwaita-dark
    xfconf-query -c xsettings -p /Net/ThemeName -s "Adwaita-dark"

    # Start GUI. exec replaces the login shell so that getty restarts and
    # autologin brings X back after "Save & restart" ends the session.
    exec startxfce4
fi