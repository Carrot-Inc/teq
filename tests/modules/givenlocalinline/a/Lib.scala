package glia

// Local inline givens (dotty's `Parsers.givenDef`: inline methods of the block), expanded at each
// use, in a method whose products the caller reads, and an inline method calling it; a local
// transparent inline method's expansion, in a member and in a top-level definition, names the
// top-level class of its body as its origin (`Inlines.inlineCallTrace`).
object Counter:
  def plain(): (Int, Int) =
    var n = 0
    inline given Int = { n += 1; n }
    (summon[Int], summon[Int])
  def transparent(): (Int, Int) =
    var n = 0
    transparent inline given x: Int = { n += 10; n }
    (summon[Int], x)
  def local(): (Int, Int) =
    var n = 0
    transparent inline def f: Int = { n += 100; n }
    (f, f)
  inline def both(): String = s"${plain()} ${transparent()}"

def topLevel(): Int =
  transparent inline def g: Int = 3
  g + g
