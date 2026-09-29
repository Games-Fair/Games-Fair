# Bundled fonts

`resource/fonts/` accepts font files only, so the record of where each one came from lives here.

All five draw Sudoku's digits (the board, the pencil marks, the keypad, and the home-screen tile),
whichever the player picks under Settings › Digit font. A Sudoku face has one job: every digit
must have a shape of its own at a glance, down to pencil-mark size, so a 1 never reads as a 7, a 3
or 6 or 9 never closes toward an 8 or 0, and the counters stay open. Each face here does that in a
different voice. All are unmodified and licensed under the SIL Open Font License 1.1; each one's
license text sits beside this file, and each font's name table carries its copyright notice and
license statement, which is how the license travels with the font inside the built app.

Variable fonts ship as their upstream file with only the name changed (`[wght]` became
`-Variable`, because square brackets in a file name trip shells and globs). Each one's default
instance is Regular, the weight Sudoku draws its digits at, so every platform's text engine loads
it as an ordinary regular face. Nunito, considered for the rounded slot, was left out for exactly
this reason: its variable file's default instance is ExtraLight, and upstream ships no static
Regular.

## Atkinson Hyperlegible Next (the default)

The Braille Institute designed Atkinson Hyperlegible for low-vision readers, and its whole brief is
telling similar characters apart: the 1 carries a flag and a foot the 7 lacks, the 6 and 9 end in
open tails, and the 3 cannot close into an 8. Next (2025) is the family's current generation.

| | |
|---|---|
| File | `AtkinsonHyperlegibleNext-Variable.ttf` (weight 200–800, default Regular) |
| Designers | Braille Institute, Applied Design Works, Elliott Scott, Megan Eiswerth, Letters From Sweden |
| Project | https://github.com/googlefonts/atkinson-hyperlegible-next |
| Downloaded from | https://github.com/google/fonts/blob/95f4904fc8bcf26d3420fe315560c96417c6dec7/ofl/atkinsonhyperlegiblenext/AtkinsonHyperlegibleNext%5Bwght%5D.ttf |
| SHA-256 | `5a455d1cfa099b601ab70751bb9673e8fe1854dc4500c80e1a220d0d75e31745` |
| License | [AtkinsonHyperlegibleNext-OFL.txt](AtkinsonHyperlegibleNext-OFL.txt) |

## B612

Designed for Airbus cockpit displays, where a misread digit is costly: the 3 has a flat top and
never reads as an 8, the 6 and 9 end in straight open tails, the 1 has a foot the 7 lacks, and the
4 is open.

| | |
|---|---|
| File | `B612-Regular.ttf` (version 1.008) |
| Designers | Nicolas Chauveau, Thomas Paillot, Jonathan Favre-Lamarine, Jean-Luc Vinot |
| Project | https://github.com/polarsys/b612 |
| Downloaded from | https://github.com/google/fonts/blob/2e8cd558a6a31683ba48900360aa17b6a9380347/ofl/b612/B612-Regular.ttf |
| SHA-256 | `139dce659100a83bf95b48474696e448bee95631ef84fd3d0437ced2bf33cf73` |
| License | [B612-OFL.txt](B612-OFL.txt) |

## Lexend

Built from reading-performance research to reduce visual stress: wide proportions and generous
spacing, so digits stay separate even when small.

| | |
|---|---|
| File | `Lexend-Variable.ttf` (weight 100–900, default Regular) |
| Designers | Bonnie Shaver-Troup, Thomas Jockin, Santiago Orozco, Héctor Gómez, Superunion |
| Project | https://github.com/googlefonts/lexend |
| Downloaded from | https://github.com/google/fonts/blob/e2332cf862ac3145c0ee5f24f04f4c1819b2410b/ofl/lexend/Lexend%5Bwght%5D.ttf |
| SHA-256 | `3add53e641fbc81da64da4bb254285e2831b52b029527bc0714e2b9610832ee6` |
| License | [Lexend-OFL.txt](Lexend-OFL.txt) (Reserved Font Name "RevReading Lexend") |

## Varela Round

A rounded sans with soft, open numerals, for a friendlier grid.

| | |
|---|---|
| File | `VarelaRound-Regular.ttf` |
| Designer | Joe Prince |
| Project | https://github.com/alefalefalef/Varela-Round-Hebrew |
| Downloaded from | https://github.com/google/fonts/blob/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/varelaround/VarelaRound-Regular.ttf |
| SHA-256 | `e1e47eb66dbc2ddc106661338e712d9176c9e83c669a82fde155324823d03aa2` |
| License | [VarelaRound-OFL.txt](VarelaRound-OFL.txt) (Reserved Font Names "Varela", "Varela Round") |

## Libre Baskerville

A book serif optimized for screens, for the look of a newspaper puzzle.

| | |
|---|---|
| File | `LibreBaskerville-Variable.ttf` (weight 400–700, default Regular) |
| Designer | Impallari Type |
| Project | https://github.com/impallari/Libre-Baskerville |
| Downloaded from | https://github.com/google/fonts/blob/b3d4b3ba7c4d54f15ed2be72d7f58b9097c3b252/ofl/librebaskerville/LibreBaskerville%5Bwght%5D.ttf |
| SHA-256 | `05a95421961341c5b2556285e8415df9db27dab4f4abe22b446b3c6a8b916c5d` |
| License | [LibreBaskerville-OFL.txt](LibreBaskerville-OFL.txt) (Reserved Font Name "Libre Baskerville") |
