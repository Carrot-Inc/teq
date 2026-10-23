/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Templates as scalameta 4.17.0 lays them out (scalameta.txt): from `extends` where it is written, else from the body;
// a given's and an anonymous class's from their first parent; the body a `Template.Body` of the braces or the colon's
// region (a given's `with` body from its first statement), with its self, else empty at the definition's end.
object Braced { val marker = 1 } // assert: DumpLayout.layout
object Colon:
  val c = 1
object Empty
object EmptyBraces {}
object Extends extends Serializable
object ExtendsBraces extends Serializable { val e = 2 }
object ExtendsColon extends AnyRef with Serializable:
  val f = 3
object Selfish { self =>
  val g = self.hashCode
}
object Givens:
  given AnyRef with
    val i = 4
  given named: AnyRef with { val j = 5 }
  val anon = new AnyRef { val k = 6 }
  val anonColon = new AnyRef:
    val l = 7
  val nested = new AnyRef {
    val m = new AnyRef { val n = 8 }.hashCode
  }
