package edits.nested_comment
object O { val a = 1; val b = 2 }
import /* outer /* inner */ outer */ O.{a, b}
object Test { val x = a }
