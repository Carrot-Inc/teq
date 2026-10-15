// A blank line after a type's trailing `|` ends it (`newLineOptWhenFollowing` takes one `NEWLINE`, not
// `NEWLINES`): refused.
// expect: 4:14: error: expected end of statement
type T = Int |

  String
@main def run(): Unit = println(1)
