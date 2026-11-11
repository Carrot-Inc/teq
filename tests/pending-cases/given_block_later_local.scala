// A block's context holds every given the block defines, the ones after a search too: dotty enters the block's
// definitions before it types them (Typer.typedBlockStats' `index`), and the context's implicits are its scope's
// (Contexts.scala:223-244, `scope.implicitDecls`), so `g`'s search finds the local given, nested deeper than the
// object's (scalac 3.8.4 prints 3). teq binds a frame's givens as the block reaches them and finds the object's: master
// (efddb38de3) and the head (61c1af7941) print 1 (`teq interp`).
object Main:
  given Int = 1
  def f: Int =
    def g: Int = summon[Int]
    given Int = 3
    g
  def main(args: Array[String]): Unit = println(f)
