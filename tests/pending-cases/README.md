Programs scalac accepts that teq still rejects or runs otherwise, with the output scalac gives. No suite runs this
directory (a directory under `tests/cases/` would be run as a program); a case moves to `tests/cases/` with the change that makes it pass.

`scalameta-parser/` holds programs of scalameta's parser tests (tests/fix/scalameta) that scalac's parser accepts and
teq's refuses or reads otherwise (scalac's reading in the note), one a file and each a case of its own, scalameta's tree
in its note, which teq fix's tree gives once teq types the program; a case goes with the change that makes teq's parser
read it as scalac does, and one listed in tests/fix/scalameta/LISTED.txt, a defect of teq's lexer, with its line.
