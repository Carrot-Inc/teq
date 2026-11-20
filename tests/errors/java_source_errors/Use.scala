// The reader's own errors in a Java source, in the reference's words: a text block whose opening delimiter is not
// followed by a line's end; an identifier where a modifier stands (`non` without `-sealed`).
// expect: Non.java:1:1: error: Identifier 'non' is not allowed here
// expect: Text.java:2:34: error: illegal text block open delimiter sequence, missing line terminator
// expect: Text.java:3:5: error: illegal text block open delimiter sequence, missing line terminator
object Use
