// A package whose name ends in a symbolic part takes its body in braces, as one named with letters does: scalac
// accepts `package outer.+ { .. }` and warns that the name is encoded on the class path (E222).
package outer.+ {
  object Held { val value = "held" }
  @main def Test = println(Held.value)
}
