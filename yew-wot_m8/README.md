# Yew Wot M8


## Normalized Water Years

Okay I've made a bad design decision and here I am documenting it so I don't forget.

When one "normalizes a water year", in order to preserve the original date of the survey/tap, the `date_recording` value is set to the original `date_observation`;  `date_recording` is used throughout the codebase to refer to the original date.  Yeah it's ugly but I'm kind of done with refactoring and just want to yeet this to the world already.