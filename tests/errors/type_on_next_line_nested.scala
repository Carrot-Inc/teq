// expect: 9:7: error: expected the end of the indented type, found new line
// expect: 2 errors found
// A second type in a function type's result region inside an alias's region is reported, and
// the definitions after the alias stay the object's.
object O:
  type T =
    Int =>
      String
      Boolean
  val kept = 1
val check = O.kept
