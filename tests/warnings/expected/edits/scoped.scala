package edits.scoped
object O { val a = 1; val b = 2 }
object Test {
 def f: Int = { import O.a; a }
 def g: Int = { import O.b; b }
}
