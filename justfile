default:
    cargo xtask bundle toby

install: default
    cp -r target/bundled/Toby.vst3 /Library/Audio/Plug-Ins/VST3
