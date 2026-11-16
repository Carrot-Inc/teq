// The wildcard import of `scala` outranks the other file's member of the package: `App` is scala's, so the
// object's body is its initialiser and nothing of the program's own trait runs.
import scala.*
object Main extends App { println("body of Main") }
