# What this crate does not own

World-space speech balloons and dialogue belong to `bevy_speech_bubbles`; the game's states and pausing, to the game. Audio is the host's: it hears `UiFeedback`. Localization is the host's: every widget takes the string to show. Fonts are the host's: it fills `GooeyFonts`.
