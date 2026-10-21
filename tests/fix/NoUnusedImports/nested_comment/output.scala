package edits.nested_comment
object O { val a = 1; val b = 2 }
import O.a
object Test { val x = a }
