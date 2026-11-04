# Demo environment for the VHS tapes and screenshots: fake drives, a throw-away
# HOME with a chosen theme, and a 700 MB sparse image. Never touches real drives.
#
#   use examples/vhs/setup.nu demo-env; demo-env "Tokyo Night"

# The settings files of the demo HOME, as {path, text}.
export def settings-files [home: string, theme: string]: nothing -> list<record<path: string, text: string>> {
    let theme_json = {theme: $theme} | to json --raw
    let dirs = [$"($home)/.config/flashkraft" $"($home)/Library/Application Support/flashkraft"]
    $dirs | each {|d| [tui-settings.json gui-settings.json] | each {|f| {path: $"($d)/($f)", text: (($theme_json) + "\n")} } } | flatten
}

# Point HOME at a fresh demo home with `theme` and create the sparse demo image.
export def --env demo-env [theme: string = "Tokyo Night"] {
    let home = "/tmp/fk/home"
    rm -rf $home
    for f in (settings-files $home $theme) {
        mkdir ($f.path | path dirname)
        $f.text | save --force $f.path
    }
    ^truncate -s 700m /tmp/fk/debian-12.iso
    $env.FLASHKRAFT_DEMO = "1"
    $env.HOME = $home
    $env.XDG_CONFIG_HOME = $"($home)/.config"
}
